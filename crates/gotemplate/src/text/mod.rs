//! Go: tpl/internal/go_templates/texttemplate — data-driven templates for
//! generating textual output (go1.24.0 fork with Hugo's hooks).

mod exec;
pub(crate) mod funcs;
mod hugo;
mod template;

pub use exec::MAX_EXEC_DEPTH;
pub use funcs::{
    BUILTIN_NAMES, Builtin, Func, FuncValue, eval_args, go_funcs, html_escape, html_escape_string,
    html_escaper, js_escape, js_escape_string, js_escaper, url_query_escaper,
};
pub use hugo::{
    DefaultHelper, ErrorValue, ExecHelper, Executer, Preparer, TryError, TryValue,
    is_truthful_value,
};
pub use template::{FuncMap, MissingKeyAction, Template, TplOption};
