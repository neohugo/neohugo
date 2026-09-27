// Go: github.com/yuin/goldmark@v1.7.12/parser/list_item.go

use super::list::{
    EMPTY_LIST_ITEM_WITH_BLANK_LINES, ListItemType, SKIP_LIST_PARSER_KEY, calc_list_offset,
    last_offset, matches_list_item,
};
use super::{BlockParser, Context, State};
use crate::ast::{Ast, NodeId, NodeValue};
use crate::text::Reader;
use crate::util;

struct ListItemParser;

// Go: parser/list_item.go:NewListItemParser
/// NewListItemParser returns a new BlockParser that
/// parses list items.
pub fn new_list_item_parser() -> Box<dyn BlockParser> {
    Box::new(ListItemParser)
}

impl BlockParser for ListItemParser {
    fn trigger(&self) -> Option<&[u8]> {
        Some(b"-+*0123456789")
    }

    // Go: parser/list_item.go:listItemParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        if !matches!(ast.value(parent), NodeValue::List(_)) {
            // list item must be a child of a list
            return (None, State::NO_CHILDREN);
        }
        let list = parent;
        let offset = last_offset(ast, list);
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        let (m, typ) = matches_list_item(&line, false);
        if typ == ListItemType::NotList {
            return (None, State::NO_CHILDREN);
        }
        if m[1] - offset > 3 {
            return (None, State::NO_CHILDREN);
        }

        pc.set(*EMPTY_LIST_ITEM_WITH_BLANK_LINES, None);

        let item_offset = calc_list_offset(&line, &m);
        let node = ast.new_list_item(m[3] + item_offset);
        if m[4] < 0 || util::is_blank(&line[m[4] as usize..m[5] as usize]) {
            return (Some(node), State::NO_CHILDREN);
        }

        let (pos, padding) = util::indent_position(&line[m[4] as usize..], m[4], item_offset);
        let child = m[3] + pos;
        reader.advance_and_set_padding(child, padding);
        (Some(node), State::HAS_CHILDREN)
    }

    // Go: parser/list_item.go:listItemParser.Continue
    fn continue_<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> State {
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        if util::is_blank(&line) {
            reader.advance_to_eol();
            return State::CONTINUE | State::HAS_CHILDREN;
        }

        let offset = last_offset(ast, ast.parent(node).expect("list item has no parent"));
        let is_empty =
            ast.child_count(node) == 0 && pc.get(*EMPTY_LIST_ITEM_WITH_BLANK_LINES).is_some();
        let lo = reader.line_offset();
        let (indent, _) = util::indent_width(&line, lo);
        if (is_empty || indent < offset) && indent < 4 {
            let (_, typ) = matches_list_item(&line, true);
            // new list item found
            if typ != ListItemType::NotList {
                pc.set_value(*SKIP_LIST_PARSER_KEY, true);
                return State::CLOSE;
            }
            if !is_empty {
                return State::CLOSE;
            }
        }
        let lo = reader.line_offset();
        let (pos, padding) = util::indent_position(&line, lo, offset);
        reader.advance_and_set_padding(pos, padding);

        State::CONTINUE | State::HAS_CHILDREN
    }

    fn close<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
        // nothing to do
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }

    fn can_accept_indented_line(&self) -> bool {
        false
    }
}
