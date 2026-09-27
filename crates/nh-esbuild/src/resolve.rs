//! Port of `internal/js/esbuild/resolve.go`.
//!
//! Owner: Wave B task T16 (js-css-pipeline).


//! Go `internal/js/esbuild/resolve.go`: the Hugo import resolver plugin (onResolve `.*`: shims,
//! externals, relDir, `resolveComponent` in the assets fs trying `.js/.ts/.tsx/.jsx`, `index.*`,
//! dir index, strip `.js`; onLoad `ns-hugo-imp`) and the `@params` plugin.

/// Go: `esbuild.ResolveComponent(impPath, resolve)`.
// Go: internal/js/esbuild/resolve.go:ResolveComponent
pub fn resolve_component<T>(imp_path: &str, resolve: &dyn Fn(&str) -> Option<(T, bool)>) -> Option<T> {
    todo!()
}

pub const NS_HUGO_IMPORT: &str = "ns-hugo-imp";
pub const NS_HUGO_PARAMS: &str = "ns-hugo-params";

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: internal/js/esbuild/resolve.go (323 lines; 4/5 funcs executed)
//   types: fsResolver
// EX L67-119: ResolveComponent[T any](impPath string, resolve func(string) (v T, found, isDir bool)) (v T, found bool)
//    L122-132: ResolveResource(impPath string, resourceGetter resource.ResourceGetter) (r resource.Resource)
// EX L134-136: newFSResolver(fs afero.Fs) *fsResolver
// EX L143-155: (r *fsResolver) resolveComponent(impPath string) *hugofs.FileMeta
// EX L157-323: createBuildPlugins(rs *resources.Spec, assetsResolver *fsResolver, depsManager identity.Manager, opts Options) ([]api.Plugin, error)
// ---------------------------------------------------------------------------
