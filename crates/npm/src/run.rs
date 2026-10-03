//! [`run_program`]: a package's program on Deno's runtime with Node compatibility.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use deno_config::deno_json::NodeModulesDirMode;
use deno_error::JsErrorBox;
use deno_lib::worker::{
    LibMainWorkerFactory, LibMainWorkerOptions, LibWorkerFactoryRoots, StorageKeyResolver,
};
use deno_resolver::factory::{
    ConfigDiscoveryOption, ResolverFactory, ResolverFactoryOptions, WorkspaceFactory,
    WorkspaceFactoryOptions,
};
use deno_runtime::deno_permissions::{
    Permissions, PermissionsContainer, RuntimePermissionDescriptorParser,
};
use deno_runtime::deno_tls::RootCertStoreProvider;
use deno_runtime::deno_tls::rustls::RootCertStore;
use deno_runtime::{FeatureChecker, WorkerExecutionMode, WorkerLogLevel};
use sys_traits::impls::RealSys;

use crate::loader::{LoaderFactory, Services};

/// The child process of [`ssg_base::RUN_PACKAGE_COMMAND`]: `<node_modules> <package> <bin>
/// [args…]`, the arguments after the command. Prints what failed; returns the exit code.
#[must_use]
pub fn run_main(args: &[OsString]) -> i32 {
    let result = (|| {
        let args = args
            .iter()
            .map(|a| {
                a.to_str()
                    .map(str::to_owned)
                    .ok_or_else(|| anyhow::anyhow!("argument is not UTF-8: {}", a.display()))
            })
            .collect::<anyhow::Result<Vec<String>>>()?;
        let [node_modules, package, bin, rest @ ..] = args.as_slice() else {
            anyhow::bail!("usage: <node_modules> <package> <bin> [args…]");
        };
        run_program(Path::new(node_modules), package, bin, rest.to_vec())
    })();
    result.unwrap_or_else(|e| {
        eprintln!("error: {e:#}");
        1
    })
}

/// Runs the program `bin` of `<node_modules>/<package>` with `args`, in this process's working
/// directory and environment, with every permission (as Node runs it). Returns its exit code.
///
/// Starts V8 for the process: call it once, from the main thread of a process that runs
/// nothing else (`process.exit()` ends the process).
///
/// # Errors
/// A package without that program; the program's uncaught exception.
pub fn run_program(
    node_modules: &Path,
    package: &str,
    bin: &str,
    args: Vec<String>,
) -> anyhow::Result<i32> {
    // Deno's TLS (fetch) builds rustls configurations without naming a provider.
    let _ = rustls::crypto::ring::default_provider().install_default();
    deno_core::JsRuntime::init_platform(None);
    let root = node_modules
        .parent()
        .map_or_else(|| PathBuf::from("."), Path::to_owned);
    let cwd = std::env::current_dir()?;
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?;
    rt.block_on(async {
        let workspace = Arc::new(WorkspaceFactory::new(
            RealSys,
            root.clone(),
            WorkspaceFactoryOptions {
                config_discovery: ConfigDiscoveryOption::Discover {
                    start_paths: vec![root.clone()],
                },
                node_modules_dir: Some(NodeModulesDirMode::Manual),
                root_node_modules_dir_override: Some(node_modules.to_owned()),
                no_lock: true,
                ..WorkspaceFactoryOptions::default()
            },
        ));
        let resolver = ResolverFactory::new(workspace, ResolverFactoryOptions::default());
        let node_resolver = resolver.node_resolver()?.clone();
        let services = Arc::new(Services {
            node_resolver: node_resolver.clone(),
            npm_module_loader: resolver.npm_module_loader()?.clone(),
            cjs_tracker: resolver.cjs_tracker()?.clone(),
        });
        let factory = LibMainWorkerFactory::new(
            Arc::new(deno_runtime::deno_web::BlobStore::default()),
            None,
            None,
            Arc::new(FeatureChecker::default()),
            Arc::new(deno_runtime::deno_fs::RealFs),
            None,
            None,
            Box::new(LoaderFactory(services)),
            node_resolver,
            deno_lib::npm::create_npm_process_state_provider(resolver.npm_resolver()?),
            resolver.pkg_json_resolver().clone(),
            Arc::new(DefaultRoots),
            StorageKeyResolver::empty(),
            RealSys,
            options(args, &cwd)?,
            LibWorkerFactoryRoots::default(),
            None,
        );
        let main = factory.resolve_npm_binary_entrypoint(&node_modules.join(package), Some(bin))?;
        let permissions = PermissionsContainer::new(
            Arc::new(RuntimePermissionDescriptorParser::new(RealSys)),
            Permissions::allow_all(),
        );
        let mut worker = factory.create_main_worker(
            WorkerExecutionMode::Run,
            permissions,
            main,
            Vec::new(),
            Vec::new(),
        )?;
        Ok(worker.run().await?)
    })
}

/// The main worker of a program run like `node <bin> <args>`.
fn options(argv: Vec<String>, cwd: &Path) -> anyhow::Result<LibMainWorkerOptions> {
    Ok(LibMainWorkerOptions {
        argv,
        log_level: WorkerLogLevel::Info,
        enable_raw_imports: false,
        enable_testing_features: false,
        has_node_modules_dir: true,
        inspect_brk: false,
        inspect_wait: false,
        trace_ops: None,
        is_inspecting: false,
        is_standalone: false,
        auto_serve: false,
        location: None,
        argv0: Some("node".to_owned()),
        node_debug: None,
        node_cluster_unique_id: None,
        node_cluster_sched_policy: None,
        otel_config: Default::default(),
        origin_data_folder_path: None,
        seed: None,
        unsafely_ignore_certificate_errors: None,
        skip_op_registration: false,
        node_ipc_init: None,
        no_legacy_abort: false,
        startup_snapshot: deno_snapshots::CLI_SNAPSHOT,
        residual_lazy_js_sources: deno_snapshots::RESIDUAL_LAZY_JS,
        residual_lazy_esm_sources: deno_snapshots::RESIDUAL_LAZY_ESM,
        serve_port: None,
        serve_host: None,
        close_on_idle: false,
        maybe_initial_cwd: Some(deno_path_util::url_from_directory_path(cwd)?),
        disable_offscreen_canvas: true,
    })
}

/// The root certificates of `fetch()` over HTTPS: the Mozilla set Deno ships.
struct DefaultRoots;

impl RootCertStoreProvider for DefaultRoots {
    fn get_or_try_init(&self) -> Result<&RootCertStore, JsErrorBox> {
        static STORE: OnceLock<RootCertStore> = OnceLock::new();
        Ok(STORE.get_or_init(deno_runtime::deno_tls::create_default_root_cert_store))
    }
}
