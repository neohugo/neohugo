// Go: github.com/yuin/goldmark@v1.7.12/parser/atx_heading.go

use std::sync::Arc;

use super::attribute::{ATTR_NAME_ID, parse_attributes};
use super::{BlockParser, Config, Context, OPT_ATTRIBUTE, ParserOption, State};
use crate::ast::{self, Ast, AttrValue, NodeId};
use crate::text::{EOF, Reader, Segment, new_reader, new_segment};
use crate::util;

/// A HeadingConfig struct is a data structure that holds configuration of the renderers related to headings.
#[derive(Debug, Clone, Copy, Default)]
pub struct HeadingConfig {
    pub auto_heading_id: bool,
    pub attribute: bool,
}

impl HeadingConfig {
    // Go: parser/atx_heading.go:HeadingConfig.SetOption
    /// SetOption implements SetOptioner.
    pub fn set_option(&mut self, name: &str) {
        match name {
            OPT_AUTO_HEADING_ID => self.auto_heading_id = true,
            OPT_ATTRIBUTE => self.attribute = true,
            _ => {}
        }
    }
}

/// A HeadingOption interface sets options for heading parsers.
pub trait HeadingOption: ParserOption {
    /// SetHeadingOption sets the heading config.
    fn set_heading_option(&self, c: &mut HeadingConfig);
}

/// AutoHeadingID is an option name that enables auto IDs for headings.
pub const OPT_AUTO_HEADING_ID: &str = "AutoHeadingID";

/// Go: `withAutoHeadingID`.
pub struct WithAutoHeadingID;

impl ParserOption for WithAutoHeadingID {
    fn set_parser_option(self: Box<Self>, c: &mut Config) {
        c.options
            .insert(OPT_AUTO_HEADING_ID.to_string(), Arc::new(true));
    }
}

impl HeadingOption for WithAutoHeadingID {
    fn set_heading_option(&self, p: &mut HeadingConfig) {
        p.auto_heading_id = true;
    }
}

// Go: parser/atx_heading.go:WithAutoHeadingID
/// WithAutoHeadingID is a functional option that enables custom heading ids and
/// auto generated heading ids.
pub fn with_auto_heading_id() -> WithAutoHeadingID {
    WithAutoHeadingID
}

/// Go: `withHeadingAttribute`.
pub struct WithHeadingAttribute;

impl ParserOption for WithHeadingAttribute {
    fn set_parser_option(self: Box<Self>, c: &mut Config) {
        super::with_attribute().set_parser_option(c);
    }
}

impl HeadingOption for WithHeadingAttribute {
    fn set_heading_option(&self, p: &mut HeadingConfig) {
        p.attribute = true;
    }
}

// Go: parser/atx_heading.go:WithHeadingAttribute
/// WithHeadingAttribute is a functional option that enables custom heading attributes.
pub fn with_heading_attribute() -> WithHeadingAttribute {
    WithHeadingAttribute
}

struct AtxHeadingParser {
    config: HeadingConfig,
}

// Go: parser/atx_heading.go:NewATXHeadingParser
/// NewATXHeadingParser return a new BlockParser that can parse ATX headings.
pub fn new_atx_heading_parser(opts: Vec<Box<dyn HeadingOption>>) -> Box<dyn BlockParser> {
    let mut p = AtxHeadingParser {
        config: HeadingConfig::default(),
    };
    for o in opts {
        o.set_heading_option(&mut p.config);
    }
    Box::new(p)
}

impl BlockParser for AtxHeadingParser {
    fn trigger(&self) -> Option<&[u8]> {
        Some(b"#")
    }

    // Go: parser/atx_heading.go:atxHeadingParser.Open
    fn open<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> (Option<NodeId>, State) {
        let (line, segment) = reader.peek_line();
        let line = line.unwrap_or_default();
        let pos = _pc.block_offset();
        if pos < 0 {
            return (None, State::NO_CHILDREN);
        }
        let pos = pos as usize;
        let mut i = pos;
        while i < line.len() && line[i] == b'#' {
            i += 1;
        }
        let level = (i - pos) as i64;
        if i == pos || level > 6 {
            return (None, State::NO_CHILDREN);
        }
        if i == line.len() {
            // alone '#' (without a new line character)
            return (Some(ast.new_heading(level)), State::NO_CHILDREN);
        }
        let l = util::trim_left_space_length(&line[i..]);
        if l == 0 {
            return (None, State::NO_CHILDREN);
        }
        let mut start = (i + l) as i64;
        if start >= line.len() as i64 {
            start = line.len() as i64 - 1;
        }
        let origstart = start;
        let stop = line.len() as i64 - util::trim_right_space_length(&line) as i64;

        let node = ast.new_heading(level);
        let mut parsed = false;
        if self.config.attribute {
            // handles special case like ### heading ### {#id}
            start -= 1;
            let mut closure_close: i64 = -1;
            let mut closure_open: i64 = -1;
            let mut j = start;
            while j < stop {
                let c = line[j as usize];
                if util::is_escaped_punctuation(&line, j as usize) {
                    j += 2;
                } else if util::is_space(c) && j < stop - 1 && line[(j + 1) as usize] == b'#' {
                    closure_open = j + 1;
                    let mut k = j + 1;
                    while k < stop && line[k as usize] == b'#' {
                        k += 1;
                    }
                    closure_close = k;
                    break;
                } else {
                    j += 1;
                }
            }
            if closure_close > 0 {
                reader.advance(closure_close);
                let attrs = parse_attributes(reader);
                let (rest, _) = reader.peek_line();
                parsed = attrs.is_some() && util::is_blank(rest.as_deref().unwrap_or(&[]));
                if parsed {
                    for attr in attrs.unwrap() {
                        ast.set_attribute(node, &attr.name, attr.value);
                    }
                    ast.lines_mut(node).append(new_segment(
                        segment.start + start + 1 - segment.padding,
                        segment.start + closure_open - segment.padding,
                    ));
                }
            }
        }
        if !parsed {
            start = origstart;
            let mut stop = line.len() as i64 - util::trim_right_space_length(&line) as i64;
            if stop <= start {
                // empty headings like '##[space]'
                stop = start;
            } else {
                let mut i = stop - 1;
                while line[i as usize] == b'#' && i >= start {
                    i -= 1;
                }
                if i != stop - 1 && !util::is_space(line[i as usize]) {
                    i = stop - 1;
                }
                i += 1;
                stop = i;
            }

            if !util::trim_right(&line[start as usize..stop as usize], b"#").is_empty() {
                // empty heading like '### ###'
                ast.lines_mut(node).append(new_segment(
                    segment.start + start - segment.padding,
                    segment.start + stop - segment.padding,
                ));
            }
        }
        (Some(node), State::NO_CHILDREN)
    }

    fn continue_<'a>(
        &self,
        _ast: &mut Ast,
        _node: NodeId,
        _reader: &mut dyn Reader<'a>,
        _pc: &mut Context,
    ) -> State {
        State::CLOSE
    }

    // Go: parser/atx_heading.go:atxHeadingParser.Close
    fn close<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        if self.config.attribute && ast.attribute_string(node, "id").is_none() {
            parse_last_line_attributes(ast, node, reader, pc);
        }

        if self.config.auto_heading_id {
            match ast.attribute_string(node, "id") {
                None => generate_auto_heading_id(ast, node, reader, pc),
                Some(id) => {
                    let id = attr_bytes(id);
                    pc.ids().put(&id);
                }
            }
        }
    }

    fn can_interrupt_paragraph(&self) -> bool {
        true
    }

    fn can_accept_indented_line(&self) -> bool {
        false
    }

    fn set_option(&mut self, name: &str, _value: &super::OptionValue) {
        self.config.set_option(name);
    }
}

/// Go: `id.([]byte)` (panics for other types).
pub(crate) fn attr_bytes(v: &AttrValue) -> Vec<u8> {
    match v {
        AttrValue::Bytes(b) => b.clone(),
        other => panic!("interface conversion: interface {{}} is {other:?}, not []uint8"),
    }
}

// Go: parser/atx_heading.go:generateAutoHeadingID
pub(crate) fn generate_auto_heading_id<'a>(
    ast: &mut Ast,
    node: NodeId,
    reader: &mut dyn Reader<'a>,
    pc: &mut Context,
) {
    let mut line: Vec<u8> = Vec::new();
    let last_index = ast.lines(node).len() - 1;
    if last_index > -1 {
        let last_line = ast.lines(node).at(last_index);
        line = last_line.value(reader.source()).into_owned();
    }
    let heading_id = pc.ids().generate(&line, ast::KIND_HEADING);
    ast.set_attribute(node, ATTR_NAME_ID, AttrValue::Bytes(heading_id));
}

// Go: parser/atx_heading.go:parseLastLineAttributes
pub(crate) fn parse_last_line_attributes<'a>(
    ast: &mut Ast,
    node: NodeId,
    reader: &mut dyn Reader<'a>,
    _pc: &mut Context,
) {
    let last_index = ast.lines(node).len() - 1;
    if last_index < 0 {
        // empty headings
        return;
    }
    let mut last_line = ast.lines(node).at(last_index);
    let line = last_line.value(reader.source()).into_owned();
    let mut lr = new_reader(&line);
    let mut attrs = None;
    let mut ok = false;
    let mut start = Segment::default();
    let mut end = Segment::default();
    loop {
        let c = lr.peek();
        if c == EOF {
            break;
        }
        if c == b'\\' {
            lr.advance(1);
            if lr.peek() == b'{' {
                lr.advance(1);
            }
            continue;
        }
        if c == b'{' {
            let sl;
            (sl, start) = lr.position();
            attrs = parse_attributes(&mut lr);
            ok = attrs.is_some();
            (_, end) = lr.position();
            lr.set_position(sl, start);
        }
        lr.advance(1);
    }
    if ok && util::is_blank(&line[end.start as usize..]) {
        for attr in attrs.unwrap() {
            ast.set_attribute(node, &attr.name, attr.value);
        }
        last_line.stop = last_line.start + start.start;
        ast.lines_mut(node).set(last_index, last_line);
    }
}
