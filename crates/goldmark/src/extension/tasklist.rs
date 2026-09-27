// Go: github.com/yuin/goldmark@v1.7.12/extension/tasklist.go

use std::sync::Arc;

use super::ast as east;
use crate::ast::{Ast, NodeId, NodeValue, WalkStatus};
use crate::parser::{self, Context, InlineParser, OptionValue};
use crate::renderer::html::{self, HtmlOption};
use crate::renderer::{self, NodeRenderer, NodeRendererFuncRegisterer};
use crate::text::Reader;
use crate::util::{self, BufWriter};
use crate::{Extender, Markdown};

/// Go RE2 `\s` (Perl class): `[\t\n\f\r ]`.
fn is_re_space(c: u8) -> bool {
    matches!(c, b'\t' | b'\n' | b'\x0c' | b'\r' | b' ')
}

// Go: extension/tasklist.go:taskListRegexp (`^\[([\sxX])\]\s*`)
/// `taskListRegexp.FindSubmatchIndex(line)`: `(m[1], m[2])` = (match end,
/// start of group 1; the group is one byte long).
pub(crate) fn task_list_regexp_match(line: &[u8]) -> Option<(usize, usize)> {
    if line.len() < 3 || line[0] != b'[' {
        return None;
    }
    let c = line[1];
    if !(is_re_space(c) || c == b'x' || c == b'X') || line[2] != b']' {
        return None;
    }
    let mut end = 3;
    while end < line.len() && is_re_space(line[end]) {
        end += 1;
    }
    Some((end, 1))
}

struct TaskCheckBoxParser;

// Go: extension/tasklist.go:NewTaskCheckBoxParser
/// NewTaskCheckBoxParser returns a new  InlineParser that can parse
/// checkboxes in list items.
/// This parser must take precedence over the parser.LinkParser.
pub fn new_task_check_box_parser() -> Box<dyn InlineParser> {
    Box::new(TaskCheckBoxParser)
}

impl InlineParser for TaskCheckBoxParser {
    // Go: extension/tasklist.go:taskCheckBoxParser.Trigger
    fn trigger(&self) -> &[u8] {
        b"["
    }

    // Go: extension/tasklist.go:taskCheckBoxParser.Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        block: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> Option<NodeId> {
        // Given AST structure must be like
        // - List
        //   - ListItem         : parent.Parent
        //     - TextBlock      : parent
        //       (current line)
        let pp = ast.parent(parent)?;
        if ast.first_child(pp) != Some(parent) {
            return None;
        }

        if ast.has_children(parent) {
            return None;
        }
        if !matches!(ast.value(pp), NodeValue::ListItem(_)) {
            return None;
        }
        let (line, _) = block.peek_line();
        let line = line.unwrap_or_default();
        let (m1, m2) = task_list_regexp_match(&line)?;
        let value = line[m2];
        block.advance(m1 as i64);
        let checked = value == b'x' || value == b'X';
        Some(east::new_task_check_box(ast, checked))
    }

    // Go: extension/tasklist.go:taskCheckBoxParser.CloseBlock(parent, pc) does not match
    // parser.CloseBlocker (which takes a text.Reader too), so Go never
    // registers it as a close blocker; it is a no-op anyway.
}

/// TaskCheckBoxHTMLRenderer is a renderer.NodeRenderer implementation that
/// renders checkboxes in list items.
pub struct TaskCheckBoxHTMLRenderer {
    pub config: html::Config,
}

// Go: extension/tasklist.go:NewTaskCheckBoxHTMLRenderer
/// NewTaskCheckBoxHTMLRenderer returns a new TaskCheckBoxHTMLRenderer.
pub fn new_task_check_box_html_renderer(opts: Vec<Box<dyn HtmlOption>>) -> Box<dyn NodeRenderer> {
    let mut r = TaskCheckBoxHTMLRenderer {
        config: html::new_config(),
    };
    for opt in opts {
        opt.set_html_option(&mut r.config);
    }
    Box::new(r)
}

impl TaskCheckBoxHTMLRenderer {
    // Go: extension/tasklist.go:TaskCheckBoxHTMLRenderer.renderTaskCheckBox
    fn render_task_check_box(
        &self,
        w: &mut dyn BufWriter,
        _source: &[u8],
        ast: &Ast,
        node: NodeId,
        entering: bool,
    ) -> Result<WalkStatus, crate::Error> {
        if !entering {
            return Ok(WalkStatus::Continue);
        }
        let n = east::must::<east::TaskCheckBox>(ast, node, "TaskCheckBox");

        if n.is_checked {
            w.write_string("<input checked=\"\" disabled=\"\" type=\"checkbox\"");
        } else {
            w.write_string("<input disabled=\"\" type=\"checkbox\"");
        }
        if self.config.xhtml {
            w.write_string(" /> ");
        } else {
            w.write_string("> ");
        }
        Ok(WalkStatus::Continue)
    }
}

impl NodeRenderer for TaskCheckBoxHTMLRenderer {
    // Go: extension/tasklist.go:TaskCheckBoxHTMLRenderer.RegisterFuncs
    fn register_funcs(self: Arc<Self>, reg: &mut dyn NodeRendererFuncRegisterer) {
        reg.register(
            *east::KIND_TASK_CHECK_BOX,
            html::bind(&self, TaskCheckBoxHTMLRenderer::render_task_check_box),
        );
    }

    // Go: the embedded html.Config's SetOption
    fn set_option(&mut self, name: &str, value: &OptionValue) {
        self.config.set_option(name, value);
    }
}

/// Go: `type taskList struct{}`.
pub struct TaskListExt;

// Go: extension/tasklist.go:TaskList
/// TaskList is an extension that allow you to use GFM task lists.
pub fn task_list() -> Box<dyn Extender> {
    Box::new(TaskListExt)
}

impl Extender for TaskListExt {
    // Go: extension/tasklist.go:taskList.Extend
    fn extend(&self, m: &mut Markdown) {
        m.parser()
            .add_options(vec![parser::with_inline_parsers(vec![util::prioritized(
                new_task_check_box_parser(),
                0,
            )])]);
        m.renderer()
            .add_options(vec![renderer::with_node_renderers(vec![
                util::prioritized(new_task_check_box_html_renderer(Vec::new()), 500),
            ])]);
    }
}

#[cfg(test)]
mod tests {
    use super::task_list_regexp_match;

    #[test]
    fn regexp() {
        assert_eq!(task_list_regexp_match(b"[x] a"), Some((4, 1)));
        assert_eq!(task_list_regexp_match(b"[ ]\n"), Some((4, 1)));
        assert_eq!(task_list_regexp_match(b"[\t] \t\n"), Some((6, 1)));
        assert_eq!(task_list_regexp_match(b"[\x0b]"), None);
        assert_eq!(task_list_regexp_match(b"[X]"), Some((3, 1)));
        assert_eq!(task_list_regexp_match(b"[y]"), None);
        assert_eq!(task_list_regexp_match(b"[x"), None);
    }
}
