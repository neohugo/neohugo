//! The host side of `esbuild --service`: process start, request routing, builds and plugin
//! callbacks.
//!
//! The binary's version is read at runtime (`esbuild --version`) and the service is started as
//! `esbuild --service=<version> --ping`; its first bytes on stdout repeat the version. A reader
//! thread splits stdout into packets and routes them: responses to the thread waiting for them,
//! callback requests (`on-start`, `on-resolve`, `on-load`) to the build whose key they carry,
//! and esbuild's pings are answered directly.
//!
//! A build's plugin callbacks run on the thread that called [`Service::build`], which blocks
//! until the build ends anyway, so callbacks may borrow from the caller. Many builds may run at
//! once on one service.
//!
//! Callback semantics follow esbuild's native (Go) plugins: an `on-resolve` callback that returns
//! `None` passes on to the next matching callback and finally to esbuild's own resolver, and so
//! does an `on-load` callback.
//!
//! Only `std` is used here.

use std::collections::{BTreeMap, HashMap};
use std::error::Error as StdError;
use std::fmt;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex, MutexGuard};

use super::protocol::{self, Packet, PacketKind, ProtocolError, Value};
use crate::options::{DropKind, Format, Jsx, Loader, Platform, SourceMap, Target};

/// An error of the esbuild service.
#[derive(Debug)]
pub enum ServiceError {
    /// The binary could not be run.
    Spawn { binary: PathBuf, source: io::Error },
    /// `esbuild --version` failed or printed nothing.
    Version { binary: PathBuf, detail: String },
    /// The service announced a different version than `--version` printed.
    VersionMismatch { expected: String, got: String },
    /// esbuild sent a packet that cannot be decoded.
    Protocol(ProtocolError),
    /// The service stopped (its stdout closed, or writing to it failed).
    Stopped(String),
    /// esbuild answered a request with an error.
    Request(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Spawn { binary, source } => {
                write!(f, "cannot run esbuild {}: {source}", binary.display())?;
                if source.kind() == io::ErrorKind::NotFound {
                    f.write_str(" (install esbuild on PATH or set NEOHUGO_ESBUILD_BINARY)")?;
                }
                Ok(())
            }
            Self::Version { binary, detail } => write!(
                f,
                "cannot read the version of esbuild {}: {detail}",
                binary.display()
            ),
            Self::VersionMismatch { expected, got } => write!(
                f,
                "the esbuild service reports version {got:?}, expected {expected:?}"
            ),
            Self::Protocol(e) => write!(f, "invalid packet from the esbuild service: {e}"),
            Self::Stopped(reason) => write!(f, "the esbuild service stopped: {reason}"),
            Self::Request(e) => write!(f, "esbuild: {e}"),
        }
    }
}

impl StdError for ServiceError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        match self {
            Self::Spawn { source, .. } => Some(source),
            Self::Protocol(e) => Some(e),
            _ => None,
        }
    }
}

/// The error a plugin callback returns; esbuild reports it as a build error of that plugin.
pub type CallbackError = Box<dyn StdError + Send + Sync>;

/// The arguments of an `on-resolve` callback.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ResolveArgs {
    /// The import path as written.
    pub path: String,
    /// The importing module's path (`<stdin>` for the entry script).
    pub importer: String,
    /// The importing module's namespace.
    pub namespace: String,
    pub resolve_dir: String,
    /// `import-statement`, `require-call`, `entry-point`, ...
    pub kind: String,
}

/// What an `on-resolve` callback resolved an import to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Resolved {
    /// A module at `path` in `namespace` (`None`: esbuild's `file` namespace).
    Module {
        path: String,
        namespace: Option<String>,
    },
    /// Left to the runtime.
    External { path: String },
}

/// The arguments of an `on-load` callback.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LoadArgs {
    pub path: String,
    pub namespace: String,
    pub suffix: String,
}

/// What an `on-load` callback loaded.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Loaded {
    pub contents: Vec<u8>,
    /// Where the module's own imports are resolved from.
    pub resolve_dir: Option<String>,
    /// `None`: esbuild picks the loader from the path.
    pub loader: Option<Loader>,
}

/// An `on-resolve` callback: `Ok(None)` passes on to the next one.
pub type ResolveFn<'a> = Box<dyn Fn(&ResolveArgs) -> Result<Option<Resolved>, CallbackError> + 'a>;
/// An `on-load` callback: `Ok(None)` passes on to the next one.
pub type LoadFn<'a> = Box<dyn Fn(&LoadArgs) -> Result<Option<Loaded>, CallbackError> + 'a>;

/// A callback with the filter esbuild evaluates before calling it.
pub struct Hook<F> {
    /// A Go regular expression matched against the import (or module) path by esbuild.
    pub filter: String,
    /// Only paths in this namespace (`None`: every namespace).
    pub namespace: Option<String>,
    pub callback: F,
}

/// An esbuild plugin: named callbacks, registered in order.
pub struct Plugin<'a> {
    pub name: String,
    pub on_resolve: Vec<Hook<ResolveFn<'a>>>,
    pub on_load: Vec<Hook<LoadFn<'a>>>,
}

/// The entry script, passed as esbuild's stdin.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Stdin {
    pub contents: Vec<u8>,
    pub loader: Loader,
    /// Where the script's imports are resolved from.
    pub resolve_dir: Option<PathBuf>,
}

/// One build: esbuild's options for a bundle of the stdin script (the output is returned, not
/// written).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BuildRequest {
    pub stdin: Stdin,
    /// The absolute working directory: output comments and `node_modules` lookups are relative
    /// to it.
    pub working_dir: PathBuf,
    /// Output paths (and source map paths) are relative to it.
    pub out_dir: Option<PathBuf>,
    pub tsconfig: Option<PathBuf>,
    pub target: Target,
    pub format: Format,
    pub platform: Platform,
    pub minify: bool,
    pub source_map: SourceMap,
    pub sources_content: bool,
    pub defines: BTreeMap<String, String>,
    pub externals: Vec<String>,
    /// Real filenames.
    pub inject: Vec<PathBuf>,
    pub drop: Option<DropKind>,
    pub jsx: Jsx,
    pub jsx_factory: Option<String>,
    pub jsx_fragment: Option<String>,
    pub jsx_import_source: Option<String>,
    /// Extension (with its dot) → loader.
    pub loaders: BTreeMap<String, Loader>,
}

impl BuildRequest {
    /// A bundle of `stdin` with esbuild's defaults.
    #[must_use]
    pub fn new(stdin: Stdin, working_dir: PathBuf) -> Self {
        Self {
            stdin,
            working_dir,
            out_dir: None,
            tsconfig: None,
            target: Target::EsNext,
            format: Format::Iife,
            platform: Platform::Browser,
            minify: false,
            source_map: SourceMap::None,
            sources_content: true,
            defines: BTreeMap::new(),
            externals: Vec::new(),
            inject: Vec::new(),
            drop: None,
            jsx: Jsx::Transform,
            jsx_factory: None,
            jsx_fragment: None,
            jsx_import_source: None,
            loaders: BTreeMap::new(),
        }
    }

    /// The command-line flags the service rebuilds esbuild's options from. Messages are always
    /// returned in the response; `--log-level=silent` only keeps esbuild from printing them.
    #[must_use]
    pub fn flags(&self) -> Vec<String> {
        let mut f = vec![
            "--log-level=silent".to_owned(),
            "--log-limit=0".to_owned(),
            "--bundle".to_owned(),
            format!("--target={}", self.target),
            format!("--format={}", self.format),
            format!("--platform={}", self.platform),
            format!("--jsx={}", self.jsx),
        ];
        if !self.sources_content {
            f.push("--sources-content=false".to_owned());
        }
        if self.minify {
            f.extend(
                [
                    "--minify-syntax",
                    "--minify-whitespace",
                    "--minify-identifiers",
                ]
                .map(str::to_owned),
            );
        }
        if let Some(d) = self.drop {
            f.push(format!("--drop:{d}"));
        }
        for (flag, value) in [
            ("jsx-factory", &self.jsx_factory),
            ("jsx-fragment", &self.jsx_fragment),
            ("jsx-import-source", &self.jsx_import_source),
        ] {
            if let Some(v) = value {
                f.push(format!("--{flag}={v}"));
            }
        }
        for (k, v) in &self.defines {
            f.push(format!("--define:{k}={v}"));
        }
        if self.source_map != SourceMap::None {
            f.push(format!("--sourcemap={}", self.source_map));
        }
        if let Some(d) = &self.out_dir {
            f.push(format!("--outdir={}", d.display()));
        }
        if let Some(t) = &self.tsconfig {
            f.push(format!("--tsconfig={}", t.display()));
        }
        for e in &self.externals {
            f.push(format!("--external:{e}"));
        }
        for i in &self.inject {
            f.push(format!("--inject:{}", i.display()));
        }
        for (ext, l) in &self.loaders {
            f.push(format!("--loader:{ext}={l}"));
        }
        f.push(format!("--loader={}", self.stdin.loader));
        f
    }
}

/// A source location in an esbuild message.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Location {
    /// The module path as esbuild names it (`<stdin>`, a path relative to the working dir, or
    /// an absolute path in a plugin namespace).
    pub file: String,
    pub namespace: String,
    /// 1-based.
    pub line: u32,
    /// 0-based, in bytes.
    pub column: u32,
    pub length: u32,
    pub line_text: String,
}

/// A note attached to a message.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Note {
    pub text: String,
    pub location: Option<Location>,
}

/// An esbuild error or warning.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Message {
    /// The plugin that raised it (empty for esbuild itself).
    pub plugin_name: String,
    pub text: String,
    pub location: Option<Location>,
    pub notes: Vec<Note>,
}

/// An output file of a build.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OutputFile {
    pub path: String,
    pub contents: Vec<u8>,
}

/// The result of a build. A build with errors has no output files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BuildResult {
    pub output_files: Vec<OutputFile>,
    pub errors: Vec<Message>,
    pub warnings: Vec<Message>,
}

/// A packet routed to a waiting request.
enum Incoming {
    /// A callback request from esbuild for this build.
    Callback { id: u32, request: Value },
    /// The response to the request.
    Response(Value),
    /// The service stopped.
    Stopped(String),
}

#[derive(Default)]
struct Routes {
    /// Request id → the thread waiting for its response.
    responses: HashMap<u32, Sender<Incoming>>,
    /// Build key → the thread answering its callbacks.
    builds: HashMap<u32, Sender<Incoming>>,
    stopped: Option<String>,
}

/// The service's stdin, shared by the requesting threads and the reader (ping responses).
type SharedStdin = Arc<Mutex<Option<ChildStdin>>>;

/// A running `esbuild --service` process. Dropping it closes the service's stdin, which ends
/// the process.
pub struct Service {
    binary: PathBuf,
    version: String,
    child: Mutex<Child>,
    stdin: SharedStdin,
    routes: Arc<Mutex<Routes>>,
    next_id: AtomicU32,
    next_key: AtomicU32,
}

impl fmt::Debug for Service {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Service")
            .field("binary", &self.binary)
            .field("version", &self.version)
            .finish_non_exhaustive()
    }
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// Runs `binary --version`.
///
/// # Errors
/// When the binary cannot run, fails, or prints no version.
pub fn binary_version(binary: &Path) -> Result<String, ServiceError> {
    let out = Command::new(binary)
        .arg("--version")
        .stdin(Stdio::null())
        .output()
        .map_err(|source| ServiceError::Spawn {
            binary: binary.to_owned(),
            source,
        })?;
    let version = String::from_utf8_lossy(&out.stdout).trim().to_owned();
    if !out.status.success() || version.is_empty() {
        return Err(ServiceError::Version {
            binary: binary.to_owned(),
            detail: format!(
                "{}: {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ),
        });
    }
    Ok(version)
}

impl Service {
    /// Starts `binary --service=<version> --ping`, with the version read from
    /// `binary --version`, and checks the version the service announces.
    ///
    /// # Errors
    /// When the binary cannot run or the service announces another version.
    pub fn start(binary: &Path) -> Result<Self, ServiceError> {
        let version = binary_version(binary)?;
        let mut child = Command::new(binary)
            .arg(format!("--service={version}"))
            .arg("--ping")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|source| ServiceError::Spawn {
                binary: binary.to_owned(),
                source,
            })?;
        let (Some(mut stdout), Some(stdin)) = (child.stdout.take(), child.stdin.take()) else {
            unreachable!("both pipes were requested");
        };
        let announced = match read_announced_version(&mut stdout) {
            Ok(v) => v,
            Err(e) => {
                stop(&mut child);
                return Err(ServiceError::Stopped(format!("no version announced: {e}")));
            }
        };
        if announced != version {
            stop(&mut child);
            return Err(ServiceError::VersionMismatch {
                expected: version,
                got: announced,
            });
        }

        let stdin: SharedStdin = Arc::new(Mutex::new(Some(stdin)));
        let routes: Arc<Mutex<Routes>> = Arc::default();
        let (reader_routes, reader_stdin) = (Arc::clone(&routes), Arc::clone(&stdin));
        let spawned = std::thread::Builder::new()
            .name("esbuild-service".to_owned())
            .spawn(move || read_loop(stdout, &reader_routes, &reader_stdin));
        if let Err(source) = spawned {
            stop(&mut child);
            return Err(ServiceError::Spawn {
                binary: binary.to_owned(),
                source,
            });
        }
        Ok(Self {
            binary: binary.to_owned(),
            version,
            child: Mutex::new(child),
            stdin,
            routes,
            next_id: AtomicU32::new(0),
            next_key: AtomicU32::new(0),
        })
    }

    /// The binary the service runs.
    #[must_use]
    pub fn binary(&self) -> &Path {
        &self.binary
    }

    /// The esbuild version the service runs.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.version
    }

    /// Sends a `ping` request and waits for its response.
    ///
    /// # Errors
    /// When the service stopped.
    pub fn ping(&self) -> Result<(), ServiceError> {
        let (id, rx) = self.register(None)?;
        let _route = Route {
            routes: &self.routes,
            id,
            key: None,
        };
        self.send_request(id, Value::map([("command", Value::string("ping"))]))?;
        loop {
            match rx.recv() {
                Ok(Incoming::Response(_)) => return Ok(()),
                Ok(Incoming::Callback { .. }) => {}
                Ok(Incoming::Stopped(reason)) => return Err(ServiceError::Stopped(reason)),
                Err(_) => return Err(ServiceError::Stopped("reader ended".to_owned())),
            }
        }
    }

    /// Runs one build with `plugins`, answering their callbacks on this thread.
    ///
    /// # Errors
    /// When the service stopped or refused the request. Build errors (syntax errors,
    /// unresolved imports, callback errors) are in [`BuildResult::errors`].
    pub fn build(
        &self,
        req: &BuildRequest,
        plugins: &[Plugin<'_>],
    ) -> Result<BuildResult, ServiceError> {
        let callbacks = Callbacks::new(plugins);
        let key = self.next_key.fetch_add(1, Ordering::Relaxed);
        let (id, rx) = self.register(Some(key))?;
        let _route = Route {
            routes: &self.routes,
            id,
            key: Some(key),
        };

        let mut request = BTreeMap::from([
            ("command".to_owned(), Value::string("build")),
            ("key".to_owned(), int(key)),
            ("entries".to_owned(), Value::Array(Vec::new())),
            ("flags".to_owned(), Value::strings(req.flags())),
            ("write".to_owned(), Value::Bool(false)),
            (
                "stdinContents".to_owned(),
                Value::Bytes(req.stdin.contents.clone()),
            ),
            (
                "absWorkingDir".to_owned(),
                Value::string(req.working_dir.to_string_lossy()),
            ),
            ("nodePaths".to_owned(), Value::Array(Vec::new())),
            ("context".to_owned(), Value::Bool(false)),
        ]);
        if let Some(dir) = &req.stdin.resolve_dir {
            request.insert(
                "stdinResolveDir".to_owned(),
                Value::string(dir.to_string_lossy()),
            );
        }
        if !plugins.is_empty() {
            request.insert("plugins".to_owned(), Callbacks::registration(plugins));
        }
        self.send_request(id, Value::Map(request))?;

        loop {
            let incoming = rx
                .recv()
                .map_err(|_| ServiceError::Stopped("reader ended".to_owned()))?;
            match incoming {
                Incoming::Stopped(reason) => return Err(ServiceError::Stopped(reason)),
                Incoming::Response(v) => {
                    if let Some(e) = v.get("error").and_then(Value::as_str) {
                        return Err(ServiceError::Request(e.to_owned()));
                    }
                    return Ok(decode_build_result(&v));
                }
                Incoming::Callback { id, request } => {
                    let response = callbacks.answer(&request);
                    self.send(&Packet {
                        id,
                        kind: PacketKind::Response,
                        value: response,
                    })?;
                }
            }
        }
    }

    /// Allocates a request id and registers its routes.
    fn register(&self, key: Option<u32>) -> Result<(u32, Receiver<Incoming>), ServiceError> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let (tx, rx) = channel();
        let mut routes = lock(&self.routes);
        if let Some(reason) = &routes.stopped {
            return Err(ServiceError::Stopped(reason.clone()));
        }
        if let Some(key) = key {
            routes.builds.insert(key, tx.clone());
        }
        routes.responses.insert(id, tx);
        Ok((id, rx))
    }

    fn send_request(&self, id: u32, value: Value) -> Result<(), ServiceError> {
        self.send(&Packet {
            id,
            kind: PacketKind::Request,
            value,
        })
    }

    fn send(&self, packet: &Packet) -> Result<(), ServiceError> {
        send_to(&self.stdin, packet)
    }
}

impl Drop for Service {
    fn drop(&mut self) {
        // The end of stdin ends the service.
        lock(&self.stdin).take();
        let _ = lock(&self.child).wait();
    }
}

fn stop(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn int(key: u32) -> Value {
    Value::Int(i32::try_from(key).expect("fewer than 2^31 builds per service"))
}

fn read_announced_version(stdout: &mut ChildStdout) -> io::Result<String> {
    let mut len = [0u8; 4];
    stdout.read_exact(&mut len)?;
    let len = usize::try_from(u32::from_le_bytes(len))
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut v = vec![0; len];
    stdout.read_exact(&mut v)?;
    Ok(String::from_utf8_lossy(&v).into_owned())
}

fn send_to(stdin: &SharedStdin, packet: &Packet) -> Result<(), ServiceError> {
    let bytes = protocol::encode(packet);
    let mut stdin = lock(stdin);
    let w = stdin
        .as_mut()
        .ok_or_else(|| ServiceError::Stopped("stdin closed".to_owned()))?;
    w.write_all(&bytes)
        .and_then(|()| w.flush())
        .map_err(|e| ServiceError::Stopped(e.to_string()))
}

/// Removes a request's routes when it ends.
struct Route<'a> {
    routes: &'a Mutex<Routes>,
    id: u32,
    key: Option<u32>,
}

impl Drop for Route<'_> {
    fn drop(&mut self) {
        let mut r = lock(self.routes);
        r.responses.remove(&self.id);
        if let Some(k) = self.key {
            r.builds.remove(&k);
        }
    }
}

/// Reads packets until stdout closes, then fails every waiting request.
fn read_loop(mut stdout: ChildStdout, routes: &Mutex<Routes>, stdin: &SharedStdin) {
    let mut chunk = vec![0u8; 16 * 1024];
    let mut stream = Vec::new();
    let reason = 'read: loop {
        let n = match stdout.read(&mut chunk) {
            Ok(0) => break "the service closed its output".to_owned(),
            Ok(n) => n,
            Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
            Err(e) => break e.to_string(),
        };
        stream.extend_from_slice(&chunk[..n]);
        let mut rest: &[u8] = &stream;
        while let Some((frame, after)) = protocol::split_frame(rest) {
            rest = after;
            match protocol::decode(frame) {
                Ok(p) => dispatch(routes, stdin, p),
                Err(e) => break 'read ServiceError::Protocol(e).to_string(),
            }
        }
        let consumed = stream.len() - rest.len();
        stream.drain(..consumed);
    };
    let mut guard = lock(routes);
    let r = &mut *guard;
    r.stopped = Some(reason.clone());
    for (_, tx) in r.responses.drain().chain(r.builds.drain()) {
        let _ = tx.send(Incoming::Stopped(reason.clone()));
    }
}

fn dispatch(routes: &Mutex<Routes>, stdin: &SharedStdin, p: Packet) {
    if p.kind == PacketKind::Response {
        if let Some(tx) = lock(routes).responses.remove(&p.id) {
            let _ = tx.send(Incoming::Response(p.value));
        }
        return;
    }
    let command = p.value.get("command").and_then(Value::as_str);
    if command == Some("ping") {
        let _ = send_to(
            stdin,
            &Packet {
                id: p.id,
                kind: PacketKind::Response,
                value: Value::Map(BTreeMap::new()),
            },
        );
        return;
    }
    let target = p
        .value
        .get("key")
        .and_then(Value::as_int)
        .and_then(|k| u32::try_from(k).ok())
        .and_then(|k| lock(routes).builds.get(&k).cloned());
    let response = match target {
        Some(tx) => match tx.send(Incoming::Callback {
            id: p.id,
            request: p.value,
        }) {
            Ok(()) => return,
            Err(_) => "the build already ended".to_owned(),
        },
        None => format!("unknown request {:?}", command.unwrap_or("")),
    };
    // Never leave the service waiting on an answer.
    let _ = send_to(
        stdin,
        &Packet {
            id: p.id,
            kind: PacketKind::Response,
            value: Value::map([("errors", Value::Array(vec![error_message("", &response)]))]),
        },
    );
}

/// One registered callback.
enum Callback<'p, 'a> {
    Resolve(&'p str, &'p Hook<ResolveFn<'a>>),
    Load(&'p str, &'p Hook<LoadFn<'a>>),
}

/// The callbacks of one build; a callback's id is its index. Ids are assigned plugin by plugin,
/// each plugin's `on_resolve` hooks before its `on_load` hooks.
struct Callbacks<'p, 'a>(Vec<Callback<'p, 'a>>);

impl<'p, 'a> Callbacks<'p, 'a> {
    fn new(plugins: &'p [Plugin<'a>]) -> Self {
        let mut all = Vec::new();
        for p in plugins {
            let name = p.name.as_str();
            all.extend(p.on_resolve.iter().map(|h| Callback::Resolve(name, h)));
            all.extend(p.on_load.iter().map(|h| Callback::Load(name, h)));
        }
        Self(all)
    }

    /// The `plugins` field of the build request.
    fn registration(plugins: &[Plugin<'_>]) -> Value {
        fn hook<F>(id: &mut i32, h: &Hook<F>) -> Value {
            let v = Value::map([
                ("id", Value::Int(*id)),
                ("filter", Value::string(h.filter.as_str())),
                (
                    "namespace",
                    Value::string(h.namespace.as_deref().unwrap_or_default()),
                ),
            ]);
            *id += 1;
            v
        }
        let mut next = 0;
        let mut out = Vec::with_capacity(plugins.len());
        for p in plugins {
            let on_resolve = p.on_resolve.iter().map(|h| hook(&mut next, h)).collect();
            let on_load = p.on_load.iter().map(|h| hook(&mut next, h)).collect();
            out.push(Value::map([
                ("name", Value::string(p.name.as_str())),
                ("onStart", Value::Bool(false)),
                ("onEnd", Value::Bool(false)),
                ("onResolve", Value::Array(on_resolve)),
                ("onLoad", Value::Array(on_load)),
            ]));
        }
        Value::Array(out)
    }

    /// Answers a callback request of this build.
    fn answer(&self, request: &Value) -> Value {
        match request.get("command").and_then(Value::as_str) {
            Some("on-start") => Value::map([
                ("errors", Value::Array(Vec::new())),
                ("warnings", Value::Array(Vec::new())),
            ]),
            Some("on-resolve") => self.on_resolve(request),
            Some("on-load") => self.on_load(request),
            other => Value::map([(
                "errors",
                Value::Array(vec![error_message(
                    "",
                    &format!("unknown request {:?}", other.unwrap_or_default()),
                )]),
            )]),
        }
    }

    /// The callbacks esbuild matched, in its order.
    fn matched<'r>(
        &'r self,
        request: &'r Value,
    ) -> impl Iterator<Item = (i32, &'r Callback<'p, 'a>)> {
        request
            .get("ids")
            .and_then(Value::as_array)
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_int)
            .filter_map(|id| Some((id, self.0.get(usize::try_from(id).ok()?)?)))
    }

    fn on_resolve(&self, request: &Value) -> Value {
        let args = ResolveArgs {
            path: field(request, "path"),
            importer: field(request, "importer"),
            namespace: field(request, "namespace"),
            resolve_dir: field(request, "resolveDir"),
            kind: field(request, "kind"),
        };
        for (id, cb) in self.matched(request) {
            let Callback::Resolve(plugin, hook) = cb else {
                continue;
            };
            let mut m = BTreeMap::from([("id".to_owned(), Value::Int(id))]);
            match (hook.callback)(&args) {
                Ok(None) => continue,
                Ok(Some(Resolved::Module { path, namespace })) => {
                    m.insert("path".to_owned(), Value::String(path));
                    if let Some(ns) = namespace {
                        m.insert("namespace".to_owned(), Value::String(ns));
                    }
                }
                Ok(Some(Resolved::External { path })) => {
                    m.insert("path".to_owned(), Value::String(path));
                    m.insert("external".to_owned(), Value::Bool(true));
                }
                Err(e) => {
                    m.insert("errors".to_owned(), callback_error(plugin, &*e));
                }
            }
            return Value::Map(m);
        }
        Value::Map(BTreeMap::new())
    }

    fn on_load(&self, request: &Value) -> Value {
        let args = LoadArgs {
            path: field(request, "path"),
            namespace: field(request, "namespace"),
            suffix: field(request, "suffix"),
        };
        for (id, cb) in self.matched(request) {
            let Callback::Load(plugin, hook) = cb else {
                continue;
            };
            let mut m = BTreeMap::from([("id".to_owned(), Value::Int(id))]);
            match (hook.callback)(&args) {
                Ok(None) => continue,
                Ok(Some(loaded)) => {
                    m.insert("contents".to_owned(), Value::Bytes(loaded.contents));
                    if let Some(dir) = loaded.resolve_dir {
                        m.insert("resolveDir".to_owned(), Value::String(dir));
                    }
                    if let Some(l) = loaded.loader {
                        m.insert("loader".to_owned(), Value::string(l.as_str()));
                    }
                }
                Err(e) => {
                    m.insert("errors".to_owned(), callback_error(plugin, &*e));
                }
            }
            return Value::Map(m);
        }
        Value::Map(BTreeMap::new())
    }
}

fn callback_error(plugin: &str, e: &(dyn StdError + Send + Sync)) -> Value {
    Value::Array(vec![error_message(plugin, &e.to_string())])
}

fn field(v: &Value, key: &str) -> String {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// A message in the protocol's shape.
fn error_message(plugin_name: &str, text: &str) -> Value {
    Value::map([
        ("id", Value::string("")),
        ("pluginName", Value::string(plugin_name)),
        ("text", Value::string(text)),
        ("location", Value::Null),
        ("notes", Value::Array(Vec::new())),
        ("detail", Value::Int(-1)),
    ])
}

fn decode_location(v: Option<&Value>) -> Option<Location> {
    let v = v.filter(|v| **v != Value::Null)?;
    let num = |k: &str| {
        v.get(k)
            .and_then(Value::as_int)
            .and_then(|i| u32::try_from(i).ok())
            .unwrap_or(0)
    };
    Some(Location {
        file: field(v, "file"),
        namespace: field(v, "namespace"),
        line: num("line"),
        column: num("column"),
        length: num("length"),
        line_text: field(v, "lineText"),
    })
}

fn decode_messages(v: Option<&Value>) -> Vec<Message> {
    v.and_then(Value::as_array)
        .unwrap_or_default()
        .iter()
        .map(|m| Message {
            plugin_name: field(m, "pluginName"),
            text: field(m, "text"),
            location: decode_location(m.get("location")),
            notes: m
                .get("notes")
                .and_then(Value::as_array)
                .unwrap_or_default()
                .iter()
                .map(|n| Note {
                    text: field(n, "text"),
                    location: decode_location(n.get("location")),
                })
                .collect(),
        })
        .collect()
}

fn decode_build_result(v: &Value) -> BuildResult {
    BuildResult {
        output_files: v
            .get("outputFiles")
            .and_then(Value::as_array)
            .unwrap_or_default()
            .iter()
            .map(|f| OutputFile {
                path: field(f, "path"),
                contents: f
                    .get("contents")
                    .and_then(Value::as_bytes)
                    .unwrap_or_default()
                    .to_vec(),
            })
            .collect(),
        errors: decode_messages(v.get("errors")),
        warnings: decode_messages(v.get("warnings")),
    }
}
