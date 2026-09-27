//! Go: github.com/yuin/goldmark@v1.7.12/renderer — renders the given AST
//! to certain formats.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use crate::ast::{self, Ast, NodeId, NodeKind, WalkStatus};
use crate::parser::OptionValue;
use crate::util::{self, BufWriter, PrioritizedValue};

pub mod html;

/// Go: `renderer.OptionName`.
pub type OptionName = String;

/// A Config struct is a data structure that holds configuration of the Renderer.
#[derive(Default)]
pub struct Config {
    pub options: HashMap<OptionName, OptionValue>,
    pub node_renderers: Vec<PrioritizedValue<Box<dyn NodeRenderer>>>,
}

// Go: renderer/renderer.go:NewConfig
/// NewConfig returns a new Config.
pub fn new_config() -> Config {
    Config::default()
}

/// An Option interface is a functional option type for the Renderer.
pub trait RendererOption: Send {
    /// SetConfig applies this option.
    fn set_config(self: Box<Self>, c: &mut Config);
}

struct WithNodeRenderers(Vec<PrioritizedValue<Box<dyn NodeRenderer>>>);

impl RendererOption for WithNodeRenderers {
    fn set_config(self: Box<Self>, c: &mut Config) {
        c.node_renderers.extend(self.0);
    }
}

// Go: renderer/renderer.go:WithNodeRenderers
/// WithNodeRenderers is a functional option that allow you to add
/// NodeRenderers to the renderer.
pub fn with_node_renderers(
    ps: Vec<PrioritizedValue<Box<dyn NodeRenderer>>>,
) -> Box<dyn RendererOption> {
    Box::new(WithNodeRenderers(ps))
}

struct WithOption(OptionName, OptionValue);

impl RendererOption for WithOption {
    fn set_config(self: Box<Self>, c: &mut Config) {
        c.options.insert(self.0, self.1);
    }
}

// Go: renderer/renderer.go:WithOption
/// WithOption is a functional option that allow you to set
/// an arbitrary option to the parser.
pub fn with_option(name: &str, value: OptionValue) -> Box<dyn RendererOption> {
    Box::new(WithOption(name.to_string(), value))
}

/// A renderer error (Go `error`).
pub type Error = crate::Error;

/// NodeRendererFunc is a function that renders a given node.
pub type NodeRendererFunc = Arc<
    dyn Fn(&mut dyn BufWriter, &[u8], &Ast, NodeId, bool) -> Result<WalkStatus, Error>
        + Send
        + Sync,
>;

/// A NodeRenderer interface offers NodeRendererFuncs.
pub trait NodeRenderer: Send + Sync {
    /// RendererFuncs registers NodeRendererFuncs to given NodeRendererFuncRegisterer.
    fn register_funcs(self: Arc<Self>, reg: &mut dyn NodeRendererFuncRegisterer);

    /// SetOptioner.SetOption: sets given option to the object.
    /// Unacceptable options may be passed.
    /// Thus implementations must ignore unacceptable options.
    fn set_option(&mut self, _name: &str, _value: &OptionValue) {}
}

/// A NodeRendererFuncRegisterer registers given NodeRendererFunc to this object.
pub trait NodeRendererFuncRegisterer {
    /// Register registers given NodeRendererFunc to this object.
    fn register(&mut self, kind: NodeKind, f: NodeRendererFunc);
}

struct Registry {
    node_renderer_funcs_tmp: HashMap<NodeKind, NodeRendererFunc>,
    max_kind: usize,
}

impl NodeRendererFuncRegisterer for Registry {
    // Go: renderer/renderer.go:renderer.Register
    fn register(&mut self, kind: NodeKind, v: NodeRendererFunc) {
        self.node_renderer_funcs_tmp.insert(kind, v);
        if kind.0 as usize > self.max_kind {
            self.max_kind = kind.0 as usize;
        }
    }
}

/// A Renderer interface renders given AST node to given
/// writer with given Renderer.
pub struct Renderer {
    config: Mutex<Option<Config>>,
    node_renderer_funcs: OnceLock<Vec<Option<NodeRendererFunc>>>,
}

// Go: renderer/renderer.go:NewRenderer
/// NewRenderer returns a new Renderer with given options.
pub fn new_renderer(options: Vec<Box<dyn RendererOption>>) -> Renderer {
    let mut config = new_config();
    for opt in options {
        opt.set_config(&mut config);
    }
    Renderer {
        config: Mutex::new(Some(config)),
        node_renderer_funcs: OnceLock::new(),
    }
}

impl Renderer {
    // Go: renderer/renderer.go:renderer.AddOptions
    /// AddOptions adds given option to this renderer. Options added after
    /// the first Render are ignored (Go dereferences a nil config).
    pub fn add_options(&self, opts: Vec<Box<dyn RendererOption>>) {
        let mut cfg = self.config.lock().unwrap();
        if let Some(cfg) = cfg.as_mut() {
            for opt in opts {
                opt.set_config(cfg);
            }
        }
    }

    fn funcs(&self) -> &Vec<Option<NodeRendererFunc>> {
        self.node_renderer_funcs.get_or_init(|| {
            let mut config = self.config.lock().unwrap().take().expect("renderer config");
            let options = std::mem::take(&mut config.options);
            util::sort_prioritized(&mut config.node_renderers);
            let mut reg = Registry {
                node_renderer_funcs_tmp: HashMap::new(),
                max_kind: 0,
            };
            let l = config.node_renderers.len();
            let mut nrs: Vec<Option<Box<dyn NodeRenderer>>> = config
                .node_renderers
                .into_iter()
                .map(|v| Some(v.value))
                .collect();
            for i in (0..l).rev() {
                let mut nr = nrs[i].take().unwrap();
                for (oname, ovalue) in &options {
                    nr.set_option(oname, ovalue);
                }
                let nr: Arc<dyn NodeRenderer> = Arc::from(nr);
                nr.register_funcs(&mut reg);
            }
            let mut funcs: Vec<Option<NodeRendererFunc>> = vec![None; reg.max_kind + 1];
            for (kind, nr) in reg.node_renderer_funcs_tmp {
                funcs[kind.0 as usize] = Some(nr);
            }
            funcs
        })
    }

    // Go: renderer/renderer.go:renderer.Render
    /// Render renders the given AST node to the given writer with the given Renderer.
    pub fn render(
        &self,
        w: &mut dyn BufWriter,
        source: &[u8],
        ast: &Ast,
        n: NodeId,
    ) -> Result<(), Error> {
        let funcs = self.funcs();
        ast::walk_ref(ast, n, &mut |ast, n, entering| {
            let mut s = WalkStatus::Continue;
            let kind = ast.kind(n).0 as usize;
            // Go indexes the slice by kind and panics past its end.
            let f = &funcs[kind];
            if let Some(f) = f {
                s = f(w, source, ast, n, entering)?;
            }
            Ok(s)
        })?;
        w.flush()
    }
}
