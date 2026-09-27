//! Port of `modules/client.go`.
//!
//! only the project-module path (no go/npm)
//!
//! Owner: Wave B task T09 (allconfig-modules).


use std::sync::Arc;

use super::config::Config;

/// Go: `modules.ClientConfig`.
#[derive(Clone)]
pub struct ClientConfig {
    pub working_dir: String,
    pub themes_dir: String,
    pub publish_dir: String,
    pub environment: String,
    pub cache_dir: String,
    pub module_config: Config,
    pub ignore_module_does_not_exist: bool,
}

/// Go: `modules.Client` (project-module subset: no `go`/`npm` invocations).
pub struct Client {
    pub ccfg: ClientConfig,
    pub module_config: Config,
}

impl Client {
    // Go: modules/client.go:NewClient
    pub fn new(cfg: ClientConfig) -> Client {
        todo!()
    }
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (generated from the Go sources; `EX` = executed by the seeksnack build,
// see specs/architecture-core-data/neohugo-executed-funcs.txt). Port every EX item faithfully;
// non-EX items are ported when cheap or stubbed with an explicit unsupported error.
// Source: modules/client.go (873 lines; 4/26 funcs executed)
//   types: Client, goOutputReplacerWriter, ClientConfig, goBinaryStatus, goModule, goModuleError, goModules
// EX L67-95: NewClient(cfg ClientConfig) *Client
//    L124-147: (c *Client) Graph(w io.Writer) error
//    L150-161: (c *Client) Tidy() error
//    L175-290: (c *Client) Vendor() error
//    L293-344: (c *Client) Get(args ...string) error
//    L346-351: (c *Client) get(args ...string) error
//    L356-365: (c *Client) Init(path string) error
//    L372-390: (c *Client) Verify(clean bool) error
//    L392-422: (c *Client) Clean(pattern string) error
//    L424-426: (c *Client) runVerify() error
//    L428-430: isProbablyModule(path string) bool
// EX L432-517: (c *Client) listGoMods() (goModules, error)
//    L519-531: (c *Client) rewriteGoMod(name string, isGoMod map[string]bool) error
//    L533-581: (c *Client) rewriteGoModRewrite(name string, isGoMod map[string]bool) ([]byte, error)
//    L583-598: (c *Client) rmVendorDir(vendorDir string) error
//    L600-657: (c *Client) runGo( ctx context.Context, stdout io.Writer, args ...string, ) error
//    L668-675: (w goOutputReplacerWriter) Write(p []byte) (n int, err error)
//    L677-703: (c *Client) tidy(mods Modules, goModOnly bool) error
//    L705-707: (c *Client) shouldVendor(path string) bool
//    L709-725: (c *Client) createThemeDirname(modulePath string, isProjectMod bool) (string, error)
//    L761-763: (c ClientConfig) shouldIgnoreVendor(path string) bool
// EX L765-794: (cfg ClientConfig) toEnv() []string
//    L818-830: (modules goModules) GetByPath(p string) *goModule
// EX L832-840: (modules goModules) GetMain() *goModule
//    L842-862: getModlineSplitter(isGoMod bool) func(line string) []string
//    L864-873: pathVersion(m Module) string
// ---------------------------------------------------------------------------
