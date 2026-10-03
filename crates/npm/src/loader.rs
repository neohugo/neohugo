//! Module loading for [`crate::run_program`]: Node's resolution in the `node_modules` that is
//! there ("bring your own node_modules"), files read by Deno's npm module loader (CommonJS
//! translated for `import`, `require` through Node's own loader).

use std::borrow::Cow;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use deno_core::{
    FastString, ModuleLoadOptions, ModuleLoadReferrer, ModuleLoadResponse, ModuleLoader,
    ModuleSource, ModuleSourceCode, ModuleSpecifier, ModuleType,
    RequestedModuleType as CoreModuleType, ResolutionKind,
};
use deno_error::JsErrorBox;
use deno_lib::worker::{CreateModuleLoaderResult, ModuleLoaderFactory};
use deno_media_type::MediaType;
use deno_resolver::cjs::CjsTrackerRc;
use deno_resolver::loader::{DenoNpmModuleLoaderRc, LoadedModuleSource, RequestedModuleType};
use deno_resolver::npm::{DenoInNpmPackageChecker, NpmResolver};
use deno_runtime::deno_node::{NodeRequireLoader, NodeResolver};
use deno_runtime::deno_permissions::PermissionsContainer;
use node_resolver::errors::PackageJsonLoadError;
use node_resolver::{NodeResolutionKind, ResolutionMode};
use sys_traits::impls::RealSys;
use url::Url;

pub(crate) type Resolver = NodeResolver<DenoInNpmPackageChecker, NpmResolver<RealSys>, RealSys>;

/// What the loaders share.
pub(crate) struct Services {
    pub(crate) node_resolver: Arc<Resolver>,
    pub(crate) npm_module_loader: DenoNpmModuleLoaderRc<RealSys>,
    pub(crate) cjs_tracker: CjsTrackerRc<DenoInNpmPackageChecker, RealSys>,
}

fn js_error(e: impl std::fmt::Display) -> JsErrorBox {
    JsErrorBox::generic(e.to_string())
}

/// The loaders of the main worker and of web workers (the same: every module is a file).
pub(crate) struct LoaderFactory(pub(crate) Arc<Services>);

impl ModuleLoaderFactory for LoaderFactory {
    fn create_for_main(&self, _permissions: PermissionsContainer) -> CreateModuleLoaderResult {
        CreateModuleLoaderResult {
            module_loader: Rc::new(Loader(Arc::clone(&self.0))),
            node_require_loader: Rc::new(RequireLoader {
                cjs_tracker: self.0.cjs_tracker.clone(),
            }),
            hook_registry: None,
        }
    }

    fn create_for_worker(
        &self,
        parent_permissions: PermissionsContainer,
        _permissions: PermissionsContainer,
        _main_module_blob: Option<(ModuleSpecifier, Arc<deno_runtime::deno_web::Blob>)>,
    ) -> CreateModuleLoaderResult {
        self.create_for_main(parent_permissions)
    }
}

/// ES modules: specifiers resolved as Node resolves them, sources as Deno's npm loader reads
/// them.
struct Loader(Arc<Services>);

impl ModuleLoader for Loader {
    fn resolve(
        &self,
        specifier: &str,
        referrer: &str,
        _kind: ResolutionKind,
    ) -> Result<ModuleSpecifier, JsErrorBox> {
        if let Ok(url) = Url::parse(specifier) {
            return Ok(url);
        }
        let referrer = Url::parse(referrer).map_err(js_error)?;
        self.0
            .node_resolver
            .resolve(
                specifier,
                &referrer,
                ResolutionMode::Import,
                NodeResolutionKind::Execution,
            )
            .map_err(js_error)?
            .into_url()
            .map_err(js_error)
    }

    fn load(
        &self,
        specifier: &ModuleSpecifier,
        referrer: Option<&ModuleLoadReferrer>,
        options: ModuleLoadOptions,
    ) -> ModuleLoadResponse {
        let services = Arc::clone(&self.0);
        let specifier = specifier.clone();
        let referrer = referrer.map(|r| r.specifier.clone());
        let requested = match options.requested_module_type {
            CoreModuleType::Json => RequestedModuleType::Json,
            _ => RequestedModuleType::None,
        };
        ModuleLoadResponse::Async(Box::pin(async move {
            let loaded = services
                .npm_module_loader
                .load(
                    Cow::Borrowed(&specifier),
                    referrer.as_ref(),
                    &requested,
                    None,
                )
                .await
                .map_err(js_error)?;
            let module_type = match loaded.media_type {
                MediaType::Json => ModuleType::Json,
                MediaType::Wasm => ModuleType::Wasm,
                _ => ModuleType::JavaScript,
            };
            let code = match loaded.source {
                LoadedModuleSource::ArcStr(s) => ModuleSourceCode::String(s.to_string().into()),
                LoadedModuleSource::String(s) => ModuleSourceCode::String(s.into_owned().into()),
                LoadedModuleSource::ArcBytes(b) => {
                    ModuleSourceCode::Bytes(b.to_vec().into_boxed_slice().into())
                }
                LoadedModuleSource::Bytes(b) => {
                    ModuleSourceCode::Bytes(b.into_owned().into_boxed_slice().into())
                }
            };
            Ok(ModuleSource::new(module_type, code, &specifier, None))
        }))
    }
}

/// `require()`: files read from disk (every read is allowed, as under Node).
#[derive(Debug)]
struct RequireLoader {
    cjs_tracker: CjsTrackerRc<DenoInNpmPackageChecker, RealSys>,
}

impl NodeRequireLoader for RequireLoader {
    fn ensure_read_permission<'a>(
        &self,
        _permissions: &mut PermissionsContainer,
        path: Cow<'a, Path>,
    ) -> Result<Cow<'a, Path>, JsErrorBox> {
        Ok(path)
    }

    fn load_text_file_lossy(&self, path: &Path) -> Result<FastString, JsErrorBox> {
        let bytes = std::fs::read(path).map_err(js_error)?;
        Ok(String::from_utf8_lossy(&bytes).into_owned().into())
    }

    fn is_maybe_cjs(&self, specifier: &Url) -> Result<bool, PackageJsonLoadError> {
        self.cjs_tracker
            .is_maybe_cjs(specifier, MediaType::from_specifier(specifier))
    }

    fn is_maybe_cjs_from_require(&self, specifier: &Url) -> Result<bool, PackageJsonLoadError> {
        self.cjs_tracker
            .is_maybe_cjs_from_require(specifier, MediaType::from_specifier(specifier))
    }
}
