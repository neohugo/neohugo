// Go: github.com/yuin/goldmark@v1.7.12/parser/list.go

use std::sync::LazyLock;

use super::setext_headings::matches_setext_heading_bar;
use super::thematic_break::is_thematic_break;
use super::{BlockParser, Context, ContextKey, State, new_context_key};
use crate::ast::{Ast, NodeId, NodeValue};
use crate::text::Reader;
use crate::util;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ListItemType {
    NotList,
    BulletList,
    OrderedList,
}

pub(crate) static SKIP_LIST_PARSER_KEY: LazyLock<ContextKey> = LazyLock::new(new_context_key);
pub(crate) static EMPTY_LIST_ITEM_WITH_BLANK_LINES: LazyLock<ContextKey> =
    LazyLock::new(new_context_key);

// Go: parser/list.go:parseListItem
// Same as
// `^(([ ]*)([\-\*\+]))(\s+.*)?\n?$`.FindSubmatchIndex or
// `^(([ ]*)(\d{1,9}[\.\)]))(\s+.*)?\n?$`.FindSubmatchIndex.
pub(crate) fn parse_list_item(line: &[u8]) -> ([i64; 6], ListItemType) {
    let mut i = 0usize;
    let l = line.len();
    let mut ret = [0i64; 6];
    while i < l && line[i] == b' ' {
        let c = line[i];
        if c == b'\t' {
            return (ret, ListItemType::NotList);
        }
        i += 1;
    }
    if i > 3 {
        return (ret, ListItemType::NotList);
    }
    ret[0] = 0;
    ret[1] = i as i64;
    ret[2] = i as i64;
    let typ;
    if i < l && (line[i] == b'-' || line[i] == b'*' || line[i] == b'+') {
        i += 1;
        ret[3] = i as i64;
        typ = ListItemType::BulletList;
    } else if i < l {
        while i < l && util::is_numeric(line[i]) {
            i += 1;
        }
        ret[3] = i as i64;
        if ret[3] == ret[2] || ret[3] - ret[2] > 9 {
            return (ret, ListItemType::NotList);
        }
        if i < l && (line[i] == b'.' || line[i] == b')') {
            i += 1;
            ret[3] = i as i64;
        } else {
            return (ret, ListItemType::NotList);
        }
        typ = ListItemType::OrderedList;
    } else {
        return (ret, ListItemType::NotList);
    }
    if i < l && line[i] != b'\n' {
        let (w, _) = util::indent_width(&line[i..], 0);
        if w == 0 {
            return (ret, ListItemType::NotList);
        }
    }
    if i >= l {
        ret[4] = -1;
        ret[5] = -1;
        return (ret, typ);
    }
    ret[4] = i as i64;
    ret[5] = line.len() as i64;
    if line[(ret[5] - 1) as usize] == b'\n' && line[i] != b'\n' {
        ret[5] -= 1;
    }
    (ret, typ)
}

// Go: parser/list.go:matchesListItem
pub(crate) fn matches_list_item(source: &[u8], strict: bool) -> ([i64; 6], ListItemType) {
    let (m, typ) = parse_list_item(source);
    if typ != ListItemType::NotList && (!strict || strict && m[1] < 4) {
        return (m, typ);
    }
    (m, ListItemType::NotList)
}

// Go: parser/list.go:calcListOffset
pub(crate) fn calc_list_offset(source: &[u8], m: &[i64; 6]) -> i64 {
    let offset;
    if m[4] < 0 || util::is_blank(&source[m[4] as usize..]) {
        // list item starts with a blank line
        offset = 1;
    } else {
        let (o, _) = util::indent_width(&source[m[4] as usize..], m[4]);
        offset = if o > 4 {
            // offseted codeblock
            1
        } else {
            o
        };
    }
    offset
}

// Go: parser/list.go:lastOffset
pub(crate) fn last_offset(ast: &Ast, node: NodeId) -> i64 {
    if let Some(last_child) = ast.last_child(node) {
        return ast
            .list_item(last_child)
            .expect("interface conversion: not *ast.ListItem")
            .offset;
    }
    0
}

struct ListParser;

// Go: parser/list.go:NewListParser
/// NewListParser returns a new BlockParser that
/// parses lists.
/// This parser must take precedence over the ListItemParser.
pub fn new_list_parser() -> Box<dyn BlockParser> {
    Box::new(ListParser)
}

impl BlockParser for ListParser {
    fn trigger(&self) -> Option<&[u8]> {
        Some(b"-+*0123456789")
    }

    // Go: parser/list.go:listParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        let last = pc.last_opened_block().map(|b| b.node);
        let lok = matches!(last.map(|l| ast.value(l)), Some(NodeValue::List(_)));
        if lok || pc.get(*SKIP_LIST_PARSER_KEY).is_some() {
            pc.set(*SKIP_LIST_PARSER_KEY, None);
            return (None, State::NO_CHILDREN);
        }
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        let (m, typ) = matches_list_item(&line, true);
        if typ == ListItemType::NotList {
            return (None, State::NO_CHILDREN);
        }
        let mut start: i64 = -1;
        if typ == ListItemType::OrderedList {
            let number = &line[m[2] as usize..(m[3] - 1) as usize];
            start = go_strconv::internal::parse_int(number, 10, 0).0;
        }

        if ast.is_paragraph(last) && ast.parent(last.unwrap()) == Some(parent) {
            // we allow only lists starting with 1 to interrupt paragraphs.
            if typ == ListItemType::OrderedList && start != 1 {
                return (None, State::NO_CHILDREN);
            }
            //an empty list item cannot interrupt a paragraph:
            if m[4] < 0 || util::is_blank(&line[m[4] as usize..m[5] as usize]) {
                return (None, State::NO_CHILDREN);
            }
        }

        let marker = line[(m[3] - 1) as usize];
        let node = ast.new_list(marker);
        if start > -1 {
            ast.list_mut(node).unwrap().start = start;
        }
        pc.set(*EMPTY_LIST_ITEM_WITH_BLANK_LINES, None);
        (Some(node), State::HAS_CHILDREN)
    }

    // Go: parser/list.go:listParser.Continue
    fn continue_<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> State {
        let list = *ast.list(node).unwrap();
        let (line, _) = reader.peek_line();
        let line = line.unwrap_or_default();
        let last_child = ast.last_child(node).expect("list without items");
        if util::is_blank(&line) {
            if ast.child_count(last_child) == 0 {
                pc.set_value(*EMPTY_LIST_ITEM_WITH_BLANK_LINES, true);
            }
            return State::CONTINUE | State::HAS_CHILDREN;
        }

        // "offset" means a width that bar indicates.
        //    -  aaaaaaaa
        // |----|
        //
        // If the indent is less than the last offset like
        // - a
        //  - b          <--- current line
        // it maybe a new child of the list.
        //
        // Empty list items can have multiple blanklines
        //
        // -             <--- 1st item is an empty thus "offset" is unknown
        //
        //
        //   -           <--- current line
        //
        // -> 1 list with 2 blank items
        //
        // So if the last item is an empty, it maybe a new child of the list.
        //
        let offset = last_offset(ast, node);
        let last_is_empty = ast.child_count(last_child) == 0;
        let lo = reader.line_offset();
        let (indent, _) = util::indent_width(&line, lo);

        if indent < offset || last_is_empty {
            if indent < 4 {
                let (m, typ) = matches_list_item(&line, false); // may have a leading spaces more than 3
                if typ != ListItemType::NotList && m[1] - offset < 4 {
                    let marker = line[(m[3] - 1) as usize];
                    if !list.can_continue(marker, typ == ListItemType::OrderedList) {
                        return State::CLOSE;
                    }
                    // Thematic Breaks take precedence over lists
                    if is_thematic_break(&line[(m[3] - 1) as usize..], 0) {
                        let mut is_heading = false;
                        let last = pc.last_opened_block().map(|b| b.node);
                        if ast.is_paragraph(last)
                            && let Some(c) =
                                matches_setext_heading_bar(&line[(m[3] - 1) as usize..])
                            && c == b'-'
                        {
                            is_heading = true;
                        }
                        if !is_heading {
                            return State::CLOSE;
                        }
                    }
                    return State::CONTINUE | State::HAS_CHILDREN;
                }
            }
            if !last_is_empty {
                return State::CLOSE;
            }
        }

        if last_is_empty && indent < offset {
            return State::CLOSE;
        }

        // Non empty items can not exist next to an empty list item
        // with blank lines. So we need to close the current list
        //
        // -
        //
        //   foo
        //
        // -> 1 list with 1 blank items and 1 paragraph
        if pc.get(*EMPTY_LIST_ITEM_WITH_BLANK_LINES).is_some() {
            return State::CLOSE;
        }
        State::CONTINUE | State::HAS_CHILDREN
    }

    // Go: parser/list.go:listParser.Close
    fn close<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) {
        let mut c = ast.first_child(node);
        while let Some(cid) = c {
            if !ast.list(node).unwrap().is_tight {
                break;
            }
            let fc = ast.first_child(cid);
            if fc.is_some() && fc != ast.last_child(cid) {
                let mut c1 = ast.next_sibling(fc.unwrap());
                while let Some(c1id) = c1 {
                    if ast.has_blank_previous_lines(c1id) {
                        ast.list_mut(node).unwrap().is_tight = false;
                        break;
                    }
                    c1 = ast.next_sibling(c1id);
                }
            }
            if Some(cid) != ast.first_child(node) && ast.has_blank_previous_lines(cid) {
                ast.list_mut(node).unwrap().is_tight = false;
            }
            c = ast.next_sibling(cid);
        }

        if ast.list(node).unwrap().is_tight {
            let mut child = ast.first_child(node);
            while let Some(chid) = child {
                let mut gc = ast.first_child(chid);
                while let Some(gcid) = gc {
                    let is_paragraph = ast.is_paragraph(Some(gcid));
                    gc = ast.next_sibling(gcid);
                    if is_paragraph {
                        let text_block = ast.new_text_block();
                        let lines = ast.lines(gcid).clone();
                        ast.set_lines(text_block, &lines);
                        ast.replace_child(chid, gcid, text_block);
                    }
                }
                child = ast.next_sibling(chid);
            }
        }
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }

    fn can_accept_indented_line(&self) -> bool {
        false
    }
}
