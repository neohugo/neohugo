//! Module `service::client`.
//!
//! NEW: esbuild service client: build request, on-resolve/on-load plugin callbacks
//!
//! Owner: Wave B task T16 (js-css-pipeline).

//! Client for the pinned esbuild 0.25.6 native binary (`esbuild --service=0.25.6 --ping`).
//!
//! Go links esbuild and calls `api.Build(options)` with Go plugins. The port runs the same esbuild
//! code as a child process and talks to it the way esbuild's own JavaScript API does
//! (`lib/shared/common.ts` of the npm package, which is the host side of
//! `cmd/esbuild/service.go`):
//!
//! - the `build` request carries the options as CLI flags (`cli.ParseBuildOptions` rebuilds the
//!   same `api.BuildOptions`; [`build_flags`] writes exactly the flags `flagsForBuildOptions`
//!   would write for Hugo's options), the stdin contents, the working dir and the plugins;
//! - plugins are registered as `{name, onResolve: [{id, filter, namespace}], onLoad: [...]}`;
//!   esbuild evaluates the filters (Go regexps, as in the Go API) and sends `on-start`,
//!   `on-resolve` and `on-load` requests with the matching callback ids, which the host answers.
//!
//! The callbacks of one build run on the thread that called [`ServiceClient::build`] (it blocks
//! until the build ends anyway), so they may borrow from the caller. A reader thread routes the
//! incoming packets: responses by request id, callback requests by build key, pings answered.
//!
//! Callback semantics follow Go's native plugins, not the JavaScript host: an `onResolve`
//! callback that returns no path and is not external passes on to the next matching callback
//! (esbuild's `RunOnResolvePlugins` `continue`), and an `onLoad` callback without contents
//! passes on too (the JavaScript host would stop at the first callback that returns an object).

use std::collections::HashMap;
use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, OnceLock};

use nh_common::Result;
use nh_common::herrors::Error;

use super::protocol::{
    Packet, PacketValue, decode_packet, encode_packet, read_length_prefixed_slice,
};
use crate::build::{BuildResult, Location, Message, Note, OutputFile};
use crate::options::CompiledBuildOptions;

/// The esbuild version the port speaks to (go.mod pins `github.com/evanw/esbuild v0.25.6`).
pub const ESBUILD_VERSION: &str = "0.25.6";

/// The environment variable that names the esbuild binary (HUGO_LAYER.md §6.8).
pub const ENV_ESBUILD_BINARY: &str = "NEOHUGO_ESBUILD_BINARY";

/// Go: `api.OnResolveArgs`.
#[derive(Clone, Debug, Default)]
pub struct OnResolveArgs {
    pub path: String,
    pub importer: String,
    pub namespace: String,
    pub resolve_dir: String,
    /// `entry-point`, `import-statement`, `require-call`, ...
    pub kind: String,
}

/// Go: `api.OnResolveResult` (the fields Hugo sets). An empty `path` without `external` passes
/// on to the next callback.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OnResolveResult {
    pub path: String,
    pub external: bool,
    pub namespace: String,
}

/// Go: `api.OnLoadArgs`.
#[derive(Clone, Debug, Default)]
pub struct OnLoadArgs {
    pub path: String,
    pub namespace: String,
    pub suffix: String,
}

/// Go: `api.OnLoadResult` (the fields Hugo sets). `contents == None` passes on to the next
/// callback.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct OnLoadResult {
    pub contents: Option<Vec<u8>>,
    pub resolve_dir: String,
    /// The CLI loader name (`js`, `ts`, `json`, ...), "" for esbuild's default.
    pub loader: String,
}

pub type OnResolveFn<'a> = Box<dyn Fn(&OnResolveArgs) -> Result<OnResolveResult> + 'a>;
pub type OnLoadFn<'a> = Box<dyn Fn(&OnLoadArgs) -> Result<OnLoadResult> + 'a>;

/// Go: `api.Plugin` after `Setup` ran: its callbacks with their filters.
pub struct Plugin<'a> {
    pub name: String,
    /// `(filter, namespace, callback)`; the filter is a Go regexp (esbuild compiles it).
    pub on_resolve: Vec<(String, String, OnResolveFn<'a>)>,
    pub on_load: Vec<(String, String, OnLoadFn<'a>)>,
}

/// Plugin callbacks (Hugo's resolver + params plugins) invoked by the service. Kept for the
/// skeleton's callers; [`ServiceClient::build_with_plugins`] takes esbuild-shaped [`Plugin`]s.
pub trait PluginHost: Send + Sync {
    /// onResolve: (path, importer, namespace, resolveDir) -> Some((path, namespace)) or None (native).
    fn on_resolve(
        &self,
        path: &str,
        importer: &str,
        namespace: &str,
        resolve_dir: &str,
    ) -> Result<Option<(String, String)>>;
    /// onLoad for Hugo namespaces -> (contents, resolveDir, loader).
    fn on_load(&self, path: &str, namespace: &str) -> Result<Option<(Vec<u8>, String, String)>>;
}

/// An incoming packet routed to a build.
enum Incoming {
    /// A request from esbuild for this build (`on-start`, `on-resolve`, `on-load`).
    Request(u32, PacketValue),
    /// The response to the build request.
    Response(PacketValue),
    /// The service stopped (stdout closed or a protocol error).
    Stopped(String),
}

#[derive(Default)]
struct Routes {
    /// Request id -> the build waiting for its response.
    responses: HashMap<u32, Sender<Incoming>>,
    /// Build key -> the build that answers its callbacks.
    builds: HashMap<i32, Sender<Incoming>>,
    stopped: Option<String>,
}

/// The child's stdin, shared by the builds and the reader thread (ping responses).
type SharedStdin = Arc<Mutex<Option<ChildStdin>>>;

/// A running `esbuild --service` process.
struct Running {
    child: Mutex<Child>,
    stdin: SharedStdin,
    routes: Arc<Mutex<Routes>>,
    next_request_id: Mutex<u32>,
    next_build_key: Mutex<i32>,
}

impl Running {
    // The host side of runService: spawn, read the version, start the reader.
    fn start(binary: &str) -> Result<Running> {
        let mut child = Command::new(binary)
            .arg(format!("--service={ESBUILD_VERSION}"))
            .arg("--ping")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| {
                Error::feature_not_available(format!(
                    "failed to start esbuild {}: {e}",
                    go_strconv::quote(binary)
                ))
            })?;
        let mut stdout = child
            .stdout
            .take()
            .ok_or_else(|| Error::new("esbuild: no stdout"))?;
        let stdin: SharedStdin = Arc::new(Mutex::new(child.stdin.take()));

        // The protocol always starts with the version.
        let mut len = [0u8; 4];
        let version = stdout
            .read_exact(&mut len)
            .and_then(|_| {
                let mut v = vec![0u8; u32::from_le_bytes(len) as usize];
                stdout.read_exact(&mut v).map(|_| v)
            })
            .map_err(|e| Error::new(format!("Cannot start service: {e}")));
        let version = match version {
            Ok(v) => String::from_utf8_lossy(&v).into_owned(),
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e);
            }
        };
        if version != ESBUILD_VERSION {
            let _ = child.kill();
            let _ = child.wait();
            return Err(Error::new(format!(
                "Cannot start service: Host version \"{ESBUILD_VERSION}\" does not match binary version {}",
                go_strconv::quote(&version)
            )));
        }

        let routes: Arc<Mutex<Routes>> = Arc::default();
        let (reader_routes, reader_stdin) = (routes.clone(), stdin.clone());
        std::thread::Builder::new()
            .name("esbuild-service-reader".to_string())
            .spawn(move || read_loop(stdout, reader_routes, reader_stdin))
            .map_err(|e| Error::new(format!("esbuild: {e}")))?;
        Ok(Running {
            child: Mutex::new(child),
            stdin,
            routes,
            next_request_id: Mutex::new(0),
            next_build_key: Mutex::new(0),
        })
    }

    fn send(&self, bytes: &[u8]) -> Result<()> {
        send_to(&self.stdin, bytes)
    }
}

fn send_to(stdin: &SharedStdin, bytes: &[u8]) -> Result<()> {
    let mut stdin = lock(stdin);
    match stdin.as_mut() {
        Some(w) => w
            .write_all(bytes)
            .and_then(|_| w.flush())
            .map_err(|e| Error::new(format!("The service was stopped: {e}"))),
        None => Err(Error::new("The service was stopped")),
    }
}

impl Drop for Running {
    fn drop(&mut self) {
        // Closing stdin ends the service (end of stdin -> runService returns).
        lock(&self.stdin).take();
        let mut child = lock(&self.child);
        let _ = child.wait();
    }
}

fn lock<T>(m: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

// The host side of `runService`'s stdin loop: read all complete packets and dispatch them.
fn read_loop(
    mut stdout: std::process::ChildStdout,
    routes: Arc<Mutex<Routes>>,
    stdin: SharedStdin,
) {
    let mut buffer = vec![0u8; 16 * 1024];
    let mut stream: Vec<u8> = Vec::new();
    let reason = loop {
        let n = match stdout.read(&mut buffer) {
            Ok(0) => break "The service was stopped".to_string(),
            Ok(n) => n,
            Err(e) => break format!("The service was stopped: {e}"),
        };
        stream.extend_from_slice(&buffer[..n]);

        // Process all complete (i.e. not partial) packets
        let mut bytes: &[u8] = &stream;
        let mut bad = None;
        while let Some((packet, after)) = read_length_prefixed_slice(bytes) {
            bytes = after;
            match decode_packet(packet) {
                Some((p, _)) => dispatch(&routes, &stdin, p),
                None => {
                    bad = Some("esbuild: invalid packet".to_string());
                    break;
                }
            }
        }
        if let Some(b) = bad {
            break b;
        }
        let consumed = stream.len() - bytes.len();
        stream.drain(..consumed);
    };
    let mut r = lock(&routes);
    r.stopped = Some(reason.clone());
    for (_, s) in r.responses.drain() {
        let _ = s.send(Incoming::Stopped(reason.clone()));
    }
    for (_, s) in r.builds.drain() {
        let _ = s.send(Incoming::Stopped(reason.clone()));
    }
}

// The host's `handleIncomingPacket`: responses to their waiting build, pings answered here
// (`sendResponse(id, {})`), callback requests to the build with their key.
fn dispatch(routes: &Arc<Mutex<Routes>>, stdin: &SharedStdin, p: Packet) {
    if !p.is_request {
        let s = lock(routes).responses.remove(&p.id);
        if let Some(s) = s {
            let _ = s.send(Incoming::Response(p.value));
        }
        return;
    }
    if p.value.get("command").and_then(PacketValue::as_str) == Some("ping") {
        let _ = send_to(
            stdin,
            &encode_packet(&Packet {
                id: p.id,
                is_request: false,
                value: PacketValue::Map(vec![]),
            }),
        );
        return;
    }
    let target = p
        .value
        .get("key")
        .and_then(PacketValue::as_int)
        .and_then(|k| lock(routes).builds.get(&k).cloned());
    match target {
        Some(s) => {
            let _ = s.send(Incoming::Request(p.id, p.value));
        }
        None => {
            // No such build. The JavaScript host throws `Invalid command` and answers with the
            // error, so the service never blocks on it.
            let command = p
                .value
                .get("command")
                .and_then(PacketValue::as_str)
                .unwrap_or("undefined")
                .to_string();
            let _ = send_to(
                stdin,
                &encode_packet(&Packet {
                    id: p.id,
                    is_request: false,
                    value: PacketValue::map([(
                        "errors",
                        PacketValue::Array(vec![error_message(
                            "",
                            &format!("Invalid command: {command}"),
                        )]),
                    )]),
                }),
            );
        }
    }
}

/// The running service process (started lazily on the first build when created with
/// [`ServiceClient::lazy`]).
pub struct ServiceClient {
    /// The binary that was (or will be) started.
    pub binary: String,
    pub(crate) state: Mutex<()>,
    running: OnceLock<std::result::Result<Running, Error>>,
    /// For [`ServiceClient::lazy`]: the working dir the binary is looked up from.
    working_dir: Option<String>,
}

impl ServiceClient {
    /// Starts `binary --service=0.25.6 --ping` now and checks its version.
    pub fn start(binary: &str) -> Result<ServiceClient> {
        let c = ServiceClient {
            binary: binary.to_string(),
            state: Mutex::new(()),
            running: OnceLock::new(),
            working_dir: None,
        };
        let r = Running::start(binary);
        let err = r.as_ref().err().cloned();
        let _ = c.running.set(r);
        match err {
            Some(e) => Err(e),
            None => Ok(c),
        }
    }

    /// A client that finds and starts the binary on its first build (HUGO_LAYER.md §6.8 lookup
    /// order, see [`find_binary`]). Go links esbuild, so a build that never calls `js.Build`
    /// never needs one.
    pub fn lazy(working_dir: &str) -> ServiceClient {
        ServiceClient {
            binary: String::new(),
            state: Mutex::new(()),
            running: OnceLock::new(),
            working_dir: Some(working_dir.to_string()),
        }
    }

    fn running(&self) -> Result<&Running> {
        let r = self.running.get_or_init(|| {
            let binary = match &self.working_dir {
                Some(wd) => find_binary(wd)?,
                None => self.binary.clone(),
            };
            Running::start(&binary)
        });
        r.as_ref().map_err(Clone::clone)
    }

    /// The skeleton's entry point: plugins as a [`PluginHost`] (one `.*` onResolve and one `.*`
    /// onLoad callback in the Hugo import namespace).
    pub fn build(
        &self,
        opts: &CompiledBuildOptions,
        plugins: Option<&dyn PluginHost>,
    ) -> Result<BuildResult> {
        let mut ps = Vec::new();
        if let Some(h) = plugins {
            ps.push(Plugin {
                name: "hugo-plugin-host".to_string(),
                on_resolve: vec![(
                    ".*".to_string(),
                    String::new(),
                    Box::new(move |a: &OnResolveArgs| {
                        Ok(
                            match h.on_resolve(
                                &a.path,
                                &a.importer,
                                &a.namespace,
                                &a.resolve_dir,
                            )? {
                                Some((path, namespace)) => OnResolveResult {
                                    path,
                                    namespace,
                                    external: false,
                                },
                                None => OnResolveResult::default(),
                            },
                        )
                    }) as OnResolveFn<'_>,
                )],
                on_load: vec![(
                    ".*".to_string(),
                    String::new(),
                    Box::new(move |a: &OnLoadArgs| {
                        Ok(match h.on_load(&a.path, &a.namespace)? {
                            Some((contents, resolve_dir, loader)) => OnLoadResult {
                                contents: Some(contents),
                                resolve_dir,
                                loader,
                            },
                            None => OnLoadResult::default(),
                        })
                    }) as OnLoadFn<'_>,
                )],
            });
        }
        self.build_with_plugins(opts, &ps)
    }

    /// Go: `api.Build(opts)` with Go plugins, over the service (`buildOrContextImpl` of the
    /// JavaScript host with `write: false`).
    pub fn build_with_plugins(
        &self,
        opts: &CompiledBuildOptions,
        plugins: &[Plugin<'_>],
    ) -> Result<BuildResult> {
        let running = self.running()?;

        let flags = build_flags(opts)?;

        // Callback ids are numbered over all plugins in registration order (JS
        // `nextCallbackID++`); esbuild sends the matching ids in that order.
        let mut resolve_cbs: Vec<(&str, &OnResolveFn<'_>)> = Vec::new();
        let mut load_cbs: Vec<(&str, &OnLoadFn<'_>)> = Vec::new();
        let mut request_plugins = Vec::new();
        let mut next_id: i32 = 0;
        for p in plugins {
            let mut on_resolve = Vec::new();
            for (filter, ns, cb) in &p.on_resolve {
                on_resolve.push(PacketValue::map([
                    ("id", PacketValue::Int(next_id)),
                    ("filter", PacketValue::str(filter.as_str())),
                    ("namespace", PacketValue::str(ns.as_str())),
                ]));
                resolve_cbs.push((p.name.as_str(), cb));
                next_id += 1;
            }
            let mut on_load = Vec::new();
            for (filter, ns, cb) in &p.on_load {
                on_load.push(PacketValue::map([
                    ("id", PacketValue::Int(next_id)),
                    ("filter", PacketValue::str(filter.as_str())),
                    ("namespace", PacketValue::str(ns.as_str())),
                ]));
                load_cbs.push((p.name.as_str(), cb));
                next_id += 1;
            }
            request_plugins.push(PacketValue::map([
                ("name", PacketValue::str(p.name.as_str())),
                ("onStart", PacketValue::Bool(false)),
                ("onEnd", PacketValue::Bool(false)),
                ("onResolve", PacketValue::Array(on_resolve)),
                ("onLoad", PacketValue::Array(on_load)),
            ]));
        }
        // Ids are assigned in one sequence; map them back to the callback lists.
        let mut resolve_ids: HashMap<i32, usize> = HashMap::new();
        let mut load_ids: HashMap<i32, usize> = HashMap::new();
        {
            let (mut id, mut ri, mut li) = (0, 0, 0);
            for p in plugins {
                for _ in &p.on_resolve {
                    resolve_ids.insert(id, ri);
                    ri += 1;
                    id += 1;
                }
                for _ in &p.on_load {
                    load_ids.insert(id, li);
                    li += 1;
                    id += 1;
                }
            }
        }

        let (tx, rx): (Sender<Incoming>, Receiver<Incoming>) = channel();
        let (key, request_id) = {
            let key = {
                let mut k = lock(&running.next_build_key);
                let key = *k;
                *k += 1;
                key
            };
            let id = {
                let mut n = lock(&running.next_request_id);
                let id = *n;
                *n += 1;
                id
            };
            let mut r = lock(&running.routes);
            if let Some(reason) = &r.stopped {
                return Err(Error::new(reason.clone()));
            }
            r.builds.insert(key, tx.clone());
            r.responses.insert(id, tx);
            (key, id)
        };
        let _cleanup = BuildRoute {
            routes: &running.routes,
            key,
            request_id,
        };

        let mut request = vec![
            ("command", PacketValue::str("build")),
            ("key", PacketValue::Int(key)),
            (
                "entries",
                PacketValue::Array(
                    opts.entry_points
                        .iter()
                        .map(|e| {
                            PacketValue::Array(vec![
                                PacketValue::str(""),
                                PacketValue::str(e.as_str()),
                            ])
                        })
                        .collect(),
                ),
            ),
            ("flags", PacketValue::strings(&flags)),
            ("write", PacketValue::Bool(false)),
            (
                "stdinContents",
                match &opts.stdin_contents {
                    Some(c) => PacketValue::Bytes(c.clone()),
                    None => PacketValue::Null,
                },
            ),
            (
                "stdinResolveDir",
                if opts.stdin_contents.is_some() && !opts.stdin_resolve_dir.is_empty() {
                    PacketValue::str(opts.stdin_resolve_dir.as_str())
                } else {
                    PacketValue::Null
                },
            ),
            (
                "absWorkingDir",
                PacketValue::str(opts.abs_working_dir.as_str()),
            ),
            ("nodePaths", PacketValue::Array(vec![])),
            ("context", PacketValue::Bool(false)),
        ];
        if !plugins.is_empty() {
            request.push(("plugins", PacketValue::Array(request_plugins)));
        }
        running.send(&encode_packet(&Packet {
            id: request_id,
            is_request: true,
            value: PacketValue::map(request),
        }))?;

        loop {
            let msg = rx
                .recv()
                .map_err(|_| Error::new("The service was stopped"))?;
            match msg {
                Incoming::Stopped(reason) => return Err(Error::new(reason)),
                Incoming::Response(v) => {
                    if let Some(e) = v.get("error").and_then(PacketValue::as_str) {
                        return Err(Error::new(e.to_string()));
                    }
                    return Ok(decode_build_response(&v));
                }
                Incoming::Request(id, req) => {
                    let response = match req.get("command").and_then(PacketValue::as_str) {
                        Some("ping") => PacketValue::Map(vec![]),
                        Some("on-start") => PacketValue::map([
                            ("errors", PacketValue::Array(vec![])),
                            ("warnings", PacketValue::Array(vec![])),
                        ]),
                        Some("on-resolve") => on_resolve(&req, &resolve_cbs, &resolve_ids),
                        Some("on-load") => on_load(&req, &load_cbs, &load_ids),
                        other => PacketValue::map([(
                            "errors",
                            PacketValue::Array(vec![error_message(
                                "",
                                &format!("Invalid command: {}", other.unwrap_or("undefined")),
                            )]),
                        )]),
                    };
                    running.send(&encode_packet(&Packet {
                        id,
                        is_request: false,
                        value: response,
                    }))?;
                }
            }
        }
    }
}

/// Removes a build's routes when it ends.
struct BuildRoute<'a> {
    routes: &'a Arc<Mutex<Routes>>,
    key: i32,
    request_id: u32,
}

impl Drop for BuildRoute<'_> {
    fn drop(&mut self) {
        let mut r = lock(self.routes);
        r.builds.remove(&self.key);
        r.responses.remove(&self.request_id);
    }
}

/// An error message in the protocol's shape (`sanitizeMessages`/`extractErrorMessageV8`).
fn error_message(plugin_name: &str, text: &str) -> PacketValue {
    PacketValue::map([
        ("id", PacketValue::str("")),
        ("pluginName", PacketValue::str(plugin_name)),
        ("text", PacketValue::str(text)),
        ("location", PacketValue::Null),
        ("notes", PacketValue::Array(vec![])),
        ("detail", PacketValue::Int(-1)),
    ])
}

fn callback_ids(req: &PacketValue) -> Vec<i32> {
    req.get("ids")
        .and_then(PacketValue::as_array)
        .map(|a| a.iter().filter_map(PacketValue::as_int).collect())
        .unwrap_or_default()
}

fn str_field(req: &PacketValue, k: &str) -> String {
    req.get(k)
        .and_then(PacketValue::as_str)
        .unwrap_or("")
        .to_string()
}

// The host's `on-resolve` request callback, with Go's native plugin semantics (see the module
// docs): the first callback that returns a path or external wins.
fn on_resolve(
    req: &PacketValue,
    cbs: &[(&str, &OnResolveFn<'_>)],
    ids: &HashMap<i32, usize>,
) -> PacketValue {
    let args = OnResolveArgs {
        path: str_field(req, "path"),
        importer: str_field(req, "importer"),
        namespace: str_field(req, "namespace"),
        resolve_dir: str_field(req, "resolveDir"),
        kind: str_field(req, "kind"),
    };
    for id in callback_ids(req) {
        let Some(&i) = ids.get(&id) else { continue };
        let (name, cb) = cbs[i];
        match cb(&args) {
            Err(e) => {
                return PacketValue::map([
                    ("id", PacketValue::Int(id)),
                    (
                        "errors",
                        PacketValue::Array(vec![error_message(name, &e.to_string())]),
                    ),
                ]);
            }
            Ok(r) => {
                if r.path.is_empty() && !r.external {
                    continue;
                }
                let mut m = vec![
                    ("id", PacketValue::Int(id)),
                    ("path", PacketValue::str(r.path)),
                ];
                if r.external {
                    m.push(("external", PacketValue::Bool(true)));
                }
                if !r.namespace.is_empty() {
                    m.push(("namespace", PacketValue::str(r.namespace)));
                }
                return PacketValue::map(m);
            }
        }
    }
    PacketValue::Map(vec![])
}

// The host's `on-load` request callback (Go semantics: a result without contents passes on).
fn on_load(
    req: &PacketValue,
    cbs: &[(&str, &OnLoadFn<'_>)],
    ids: &HashMap<i32, usize>,
) -> PacketValue {
    let args = OnLoadArgs {
        path: str_field(req, "path"),
        namespace: str_field(req, "namespace"),
        suffix: str_field(req, "suffix"),
    };
    for id in callback_ids(req) {
        let Some(&i) = ids.get(&id) else { continue };
        let (name, cb) = cbs[i];
        match cb(&args) {
            Err(e) => {
                return PacketValue::map([
                    ("id", PacketValue::Int(id)),
                    (
                        "errors",
                        PacketValue::Array(vec![error_message(name, &e.to_string())]),
                    ),
                ]);
            }
            Ok(r) => {
                let Some(contents) = r.contents else { continue };
                let mut m = vec![
                    ("id", PacketValue::Int(id)),
                    ("contents", PacketValue::Bytes(contents)),
                ];
                if !r.resolve_dir.is_empty() {
                    m.push(("resolveDir", PacketValue::str(r.resolve_dir)));
                }
                if !r.loader.is_empty() {
                    m.push(("loader", PacketValue::str(r.loader)));
                }
                return PacketValue::map(m);
            }
        }
    }
    PacketValue::Map(vec![])
}

// Go: cmd/esbuild/service.go:decodeLocation (the host side of encodeLocation)
fn decode_location(v: Option<&PacketValue>) -> Option<Location> {
    let v = v?;
    if matches!(v, PacketValue::Null) {
        return None;
    }
    let s = |k: &str| str_field(v, k);
    let i = |k: &str| v.get(k).and_then(PacketValue::as_int).unwrap_or(0) as i64;
    let mut namespace = s("namespace");
    if namespace.is_empty() {
        namespace = "file".to_string();
    }
    Some(Location {
        file: s("file"),
        namespace,
        line: i("line"),
        column: i("column"),
        length: i("length"),
        line_text: s("lineText"),
        suggestion: s("suggestion"),
    })
}

// Go: cmd/esbuild/service.go:decodeMessages (the host side of encodeMessages)
fn decode_messages(v: Option<&PacketValue>) -> Vec<Message> {
    let Some(a) = v.and_then(PacketValue::as_array) else {
        return Vec::new();
    };
    a.iter()
        .map(|m| Message {
            id: str_field(m, "id"),
            plugin_name: str_field(m, "pluginName"),
            text: str_field(m, "text"),
            location: decode_location(m.get("location")),
            notes: m
                .get("notes")
                .and_then(PacketValue::as_array)
                .map(|ns| {
                    ns.iter()
                        .map(|n| Note {
                            text: str_field(n, "text"),
                            location: decode_location(n.get("location")),
                        })
                        .collect()
                })
                .unwrap_or_default(),
        })
        .collect()
}

// The host side of `resultToResponse` (`write == false`: the output files come back).
fn decode_build_response(v: &PacketValue) -> BuildResult {
    let output_files = v
        .get("outputFiles")
        .and_then(PacketValue::as_array)
        .map(|a| {
            a.iter()
                .map(|f| OutputFile {
                    path: str_field(f, "path"),
                    contents: f
                        .get("contents")
                        .and_then(PacketValue::as_bytes)
                        .unwrap_or_default()
                        .to_vec(),
                    hash: str_field(f, "hash"),
                })
                .collect()
        })
        .unwrap_or_default();
    BuildResult {
        output_files,
        errors: decode_messages(v.get("errors")),
        warnings: decode_messages(v.get("warnings")),
    }
}

/// The CLI flags for Hugo's build options: what esbuild's JavaScript `flagsForBuildOptions` writes
/// for the same `api.BuildOptions` (`cli.ParseBuildOptions` then rebuilds them in the service).
/// Only the fields Hugo's `Options.compile` sets are written; a zero value is written as no flag,
/// as in the Go API.
pub fn build_flags(o: &CompiledBuildOptions) -> Result<Vec<String>> {
    let mut flags: Vec<String> = Vec::new();
    // pushLogFlags: Go's zero LogLevel is silent (messages are still returned).
    flags.push("--log-level=silent".to_string());
    flags.push("--log-limit=0".to_string());
    // pushCommonFlags
    if !o.sources_content {
        flags.push("--sources-content=false".to_string());
    }
    if !o.target.is_empty() {
        flags.push(format!("--target={}", o.target));
    }
    if !o.format.is_empty() {
        flags.push(format!("--format={}", o.format));
    }
    if !o.platform.is_empty() {
        flags.push(format!("--platform={}", o.platform));
    }
    if o.minify_syntax {
        flags.push("--minify-syntax".to_string());
    }
    if o.minify_whitespace {
        flags.push("--minify-whitespace".to_string());
    }
    if o.minify_identifiers {
        flags.push("--minify-identifiers".to_string());
    }
    for what in &o.drop {
        flags.push(format!("--drop:{what}"));
    }
    if !o.jsx.is_empty() {
        flags.push(format!("--jsx={}", o.jsx));
    }
    if !o.jsx_factory.is_empty() {
        flags.push(format!("--jsx-factory={}", o.jsx_factory));
    }
    if !o.jsx_fragment.is_empty() {
        flags.push(format!("--jsx-fragment={}", o.jsx_fragment));
    }
    if !o.jsx_import_source.is_empty() {
        flags.push(format!("--jsx-import-source={}", o.jsx_import_source));
    }
    for (key, value) in &o.define {
        if key.contains('=') {
            return Err(Error::new(format!("Invalid define: {key}")));
        }
        flags.push(format!("--define:{key}={value}"));
    }
    // flagsForBuildOptions
    if !o.sourcemap.is_empty() && o.sourcemap != "none" {
        flags.push(format!("--sourcemap={}", o.sourcemap));
    }
    if o.bundle {
        flags.push("--bundle".to_string());
    }
    if !o.outdir.is_empty() {
        flags.push(format!("--outdir={}", o.outdir));
    }
    if !o.tsconfig.is_empty() {
        flags.push(format!("--tsconfig={}", o.tsconfig));
    }
    for name in &o.external {
        flags.push(format!("--external:{name}"));
    }
    for path in &o.inject {
        flags.push(format!("--inject:{path}"));
    }
    for (ext, loader) in &o.loader {
        if ext.contains('=') {
            return Err(Error::new(format!("Invalid loader extension: {ext}")));
        }
        flags.push(format!("--loader:{ext}={loader}"));
    }
    if o.stdin_contents.is_some() && !o.stdin_loader.is_empty() {
        flags.push(format!("--loader={}", o.stdin_loader));
    }
    Ok(flags)
}

/// The esbuild binary (HUGO_LAYER.md §6.8): `$NEOHUGO_ESBUILD_BINARY`, else
/// `<workingDir>/node_modules/@esbuild/<os>-<arch>/bin/esbuild`, else
/// `<workingDir>/node_modules/.bin/esbuild`, else `esbuild` on `PATH`. The version is checked
/// when the service starts (its first bytes are the version).
pub fn find_binary(working_dir: &str) -> Result<String> {
    let env = std::env::var_os(ENV_ESBUILD_BINARY).map(|p| p.to_string_lossy().into_owned());
    let path = std::env::var_os("PATH").map(|p| p.to_string_lossy().into_owned());
    find_binary_with(env.as_deref(), working_dir, path.as_deref())
}

/// [`find_binary`] with `$NEOHUGO_ESBUILD_BINARY` and `$PATH` given (`None` = unset).
pub fn find_binary_with(
    env_binary: Option<&str>,
    working_dir: &str,
    path_env: Option<&str>,
) -> Result<String> {
    if let Some(p) = env_binary
        && !p.is_empty()
    {
        return Ok(p.to_string());
    }
    let (os, arch) = npm_platform();
    let candidates = [
        format!("{working_dir}/node_modules/@esbuild/{os}-{arch}/bin/esbuild"),
        format!("{working_dir}/node_modules/.bin/esbuild"),
    ];
    for c in candidates {
        if std::path::Path::new(&c).is_file() {
            return Ok(c);
        }
    }
    if let Ok(p) = nh_config::hexec::look_path_in("esbuild", path_env) {
        return Ok(p);
    }
    Err(Error::feature_not_available(format!(
        "esbuild {ESBUILD_VERSION} not found: set {ENV_ESBUILD_BINARY} (tools/esbuild/build.sh builds it)"
    )))
}

/// npm's `process.platform`/`process.arch` names (the `@esbuild/<os>-<arch>` package).
fn npm_platform() -> (&'static str, &'static str) {
    let os = match std::env::consts::OS {
        "macos" => "darwin",
        "windows" => "win32",
        other => other,
    };
    let arch = match std::env::consts::ARCH {
        "x86_64" => "x64",
        "aarch64" => "arm64",
        "x86" => "ia32",
        other => other,
    };
    (os, arch)
}
