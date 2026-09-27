// Go: github.com/yuin/goldmark@v1.7.12/parser/link.go

use std::sync::LazyLock;

use super::{
    Context, ContextKey, DelimiterBottom, InlineParser, new_context_key, process_delimiters,
};
use crate::ast::{self, Ast, NodeId, NodeValue};
use crate::text::{FindClosureOptions, Reader, Segment, new_segment};
use crate::util;

pub(crate) static LINK_LABEL_STATE_KEY: LazyLock<ContextKey> = LazyLock::new(new_context_key);

/// Go: `linkLabelState`, an inline node that marks an opened `[` / `![`.
#[derive(Debug, Clone, Default)]
pub struct LinkLabelState {
    pub segment: Segment,
    pub is_image: bool,
    pub prev: Option<NodeId>,
    pub next: Option<NodeId>,
    pub first: Option<NodeId>,
    pub last: Option<NodeId>,
}

impl Ast {
    fn lls(&self, n: NodeId) -> &LinkLabelState {
        match self.value(n) {
            NodeValue::LinkLabelState(s) => s,
            _ => panic!("not a linkLabelState"),
        }
    }

    fn lls_mut(&mut self, n: NodeId) -> &mut LinkLabelState {
        match self.value_mut(n) {
            NodeValue::LinkLabelState(s) => s,
            _ => panic!("not a linkLabelState"),
        }
    }
}

// Go: parser/link.go:newLinkLabelState
fn new_link_label_state(ast: &mut Ast, segment: Segment, is_image: bool) -> NodeId {
    ast.new_node(NodeValue::LinkLabelState(LinkLabelState {
        segment,
        is_image,
        ..Default::default()
    }))
}

// Go: parser/link.go:linkLabelStateLength
fn link_label_state_length(ast: &Ast, v: Option<NodeId>) -> i64 {
    let Some(v) = v else {
        return 0;
    };
    let s = ast.lls(v);
    let (Some(last), Some(first)) = (s.last, s.first) else {
        return 0;
    };
    ast.lls(last).segment.stop - ast.lls(first).segment.start
}

fn get_list(pc: &Context) -> Option<NodeId> {
    pc.get_as::<NodeId>(*LINK_LABEL_STATE_KEY).copied()
}

// Go: parser/link.go:pushLinkLabelState
fn push_link_label_state(ast: &mut Ast, pc: &mut Context, v: NodeId) {
    match get_list(pc) {
        None => {
            let list = v;
            let s = ast.lls_mut(v);
            s.first = Some(v);
            s.last = Some(v);
            pc.set_value(*LINK_LABEL_STATE_KEY, list);
        }
        Some(list) => {
            let l = ast.lls(list).last.expect("last");
            ast.lls_mut(list).last = Some(v);
            ast.lls_mut(l).next = Some(v);
            ast.lls_mut(v).prev = Some(l);
        }
    }
}

// Go: parser/link.go:removeLinkLabelState
fn remove_link_label_state(ast: &mut Ast, pc: &mut Context, d: NodeId) {
    let Some(mut list) = get_list(pc) else {
        return;
    };
    let mut list_opt = Some(list);

    let (d_prev, d_next, d_last) = {
        let s = ast.lls(d);
        (s.prev, s.next, s.last)
    };
    match d_prev {
        None => {
            list_opt = d_next;
            if let Some(l) = list_opt {
                list = l;
                let s = ast.lls_mut(list);
                s.first = Some(d);
                s.last = d_last;
                s.prev = None;
                pc.set_value(*LINK_LABEL_STATE_KEY, list);
            } else {
                pc.set(*LINK_LABEL_STATE_KEY, None);
            }
        }
        Some(p) => {
            ast.lls_mut(p).next = d_next;
            if let Some(n) = d_next {
                ast.lls_mut(n).prev = Some(p);
            }
        }
    }
    if let Some(list) = list_opt
        && d_next.is_none()
    {
        ast.lls_mut(list).last = d_prev;
    }
    let s = ast.lls_mut(d);
    s.next = None;
    s.prev = None;
    s.first = None;
    s.last = None;
}

struct LinkParser;

// Go: parser/link.go:NewLinkParser
/// NewLinkParser return a new InlineParser that parses links.
pub fn new_link_parser() -> Box<dyn InlineParser> {
    Box::new(LinkParser)
}

static LINK_BOTTOM: LazyLock<ContextKey> = LazyLock::new(new_context_key);

/// The value stored under `linkBottom`: Go stores either one `ast.Node`
/// (possibly a nil `*Delimiter`) or a `[]ast.Node`.
enum LinkBottoms {
    One(Option<NodeId>),
    Many(Vec<Option<NodeId>>),
}

impl InlineParser for LinkParser {
    fn trigger(&self) -> &[u8] {
        b"![]"
    }

    // Go: parser/link.go:linkParser.Parse
    fn parse<'a>(
        &self,
        ast: &mut Ast,
        parent: NodeId,
        block: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) -> Option<NodeId> {
        let (line, segment) = block.peek_line();
        let line = line.unwrap_or_default();
        if line[0] == b'!' {
            if line.len() > 1 && line[1] == b'[' {
                block.advance(1);
                push_link_bottom(pc);
                return Some(process_link_label_open(
                    ast,
                    block,
                    segment.start + 1,
                    true,
                    pc,
                ));
            }
            return None;
        }
        if line[0] == b'[' {
            push_link_bottom(pc);
            return Some(process_link_label_open(
                ast,
                block,
                segment.start,
                false,
                pc,
            ));
        }

        // line[0] == ']'
        let tlist = get_list(pc)?;
        let Some(last) = ast.lls(tlist).last else {
            let _ = pop_link_bottom(pc);
            return None;
        };
        block.advance(1);
        remove_link_label_state(ast, pc, last);
        // CommonMark spec says:
        //  > A link label can have at most 999 characters inside the square brackets.
        if link_label_state_length(ast, Some(tlist)) > 998 {
            let (lp, ls) = (ast.parent(last).unwrap(), ast.lls(last).segment);
            ast::merge_or_replace_text_segment(ast, lp, last, ls);
            let _ = pop_link_bottom(pc);
            return None;
        }

        if !ast.lls(last).is_image && contains_link(ast, Some(last)) {
            // a link in a link text is not allowed
            let (lp, ls) = (ast.parent(last).unwrap(), ast.lls(last).segment);
            ast::merge_or_replace_text_segment(ast, lp, last, ls);
            let _ = pop_link_bottom(pc);
            return None;
        }

        let c = block.peek();
        let (l, pos) = block.position();
        let mut link: Option<NodeId> = None;
        if c == b'(' {
            // normal link
            link = parse_link(ast, parent, last, block, pc);
        } else if c == b'[' {
            // reference link
            let has_value;
            (link, has_value) = parse_reference_link(ast, parent, last, block, pc);
            if link.is_none() && has_value {
                let (lp, ls) = (ast.parent(last).unwrap(), ast.lls(last).segment);
                ast::merge_or_replace_text_segment(ast, lp, last, ls);
                let _ = pop_link_bottom(pc);
                return None;
            }
        }

        let link = match link {
            Some(link) => link,
            None => {
                // maybe shortcut reference link
                block.set_position(l, pos);
                let ssegment = new_segment(ast.lls(last).segment.stop, segment.start);
                let maybe_reference = block.value(ssegment);
                // CommonMark spec says:
                //  > A link label can have at most 999 characters inside the square brackets.
                if maybe_reference.len() > 999 {
                    let (lp, ls) = (ast.parent(last).unwrap(), ast.lls(last).segment);
                    ast::merge_or_replace_text_segment(ast, lp, last, ls);
                    let _ = pop_link_bottom(pc);
                    return None;
                }

                let Some(r) = pc
                    .reference(&util::to_link_reference(&maybe_reference))
                    .cloned()
                else {
                    let (lp, ls) = (ast.parent(last).unwrap(), ast.lls(last).segment);
                    ast::merge_or_replace_text_segment(ast, lp, last, ls);
                    let _ = pop_link_bottom(pc);
                    return None;
                };
                let link = ast.new_link();
                process_link_label(ast, parent, link, last, pc);
                let lk = ast.link_mut(link).unwrap();
                lk.title = r.title().map(|t| t.to_vec());
                lk.destination = r.destination().to_vec();
                link
            }
        };
        if ast.lls(last).is_image {
            let lp = ast.parent(last).unwrap();
            ast.remove_child(lp, last);
            return Some(ast.new_image(link));
        }
        let lp = ast.parent(last).unwrap();
        ast.remove_child(lp, last);
        Some(link)
    }

    fn is_close_blocker(&self) -> bool {
        true
    }

    // Go: parser/link.go:linkParser.CloseBlock
    fn close_block<'a>(
        &self,
        ast: &mut Ast,
        _parent: NodeId,
        _block: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        pc.set(*LINK_BOTTOM, None);
        let Some(tlist) = get_list(pc) else {
            return;
        };
        let mut s = Some(tlist);
        while let Some(sid) = s {
            let next = ast.lls(sid).next;
            remove_link_label_state(ast, pc, sid);
            let sp = ast.parent(sid).expect("linkLabelState has no parent");
            let t = ast.new_text_segment(ast.lls(sid).segment);
            ast.replace_child(sp, sid, t);
            s = next;
        }
    }
}

// Go: parser/link.go:linkParser.containsLink (pre-order search of n, its
// siblings and their descendants; an explicit stack instead of recursion)
fn contains_link(ast: &Ast, n: Option<NodeId>) -> bool {
    let mut stack: Vec<NodeId> = Vec::new();
    if let Some(n) = n {
        stack.push(n);
    }
    while let Some(c) = stack.pop() {
        if ast.link(c).is_some() {
            return true;
        }
        if let Some(next) = ast.next_sibling(c) {
            stack.push(next);
        }
        if let Some(fc) = ast.first_child(c) {
            stack.push(fc);
        }
    }
    false
}

// Go: parser/link.go:processLinkLabelOpen
fn process_link_label_open<'a>(
    ast: &mut Ast,
    block: &mut dyn Reader<'a>,
    pos: i64,
    is_image: bool,
    pc: &mut Context,
) -> NodeId {
    let mut start = pos;
    if is_image {
        start -= 1;
    }
    let state = new_link_label_state(ast, new_segment(start, pos + 1), is_image);
    push_link_label_state(ast, pc, state);
    block.advance(1);
    state
}

// Go: parser/link.go:linkParser.processLinkLabel
fn process_link_label(ast: &mut Ast, parent: NodeId, link: NodeId, last: NodeId, pc: &mut Context) {
    let bottom = pop_link_bottom(pc);
    process_delimiters(ast, bottom, pc);
    let mut c = ast.next_sibling(last);
    while let Some(cid) = c {
        let next = ast.next_sibling(cid);
        ast.remove_child(parent, cid);
        ast.append_child(link, cid);
        c = next;
    }
}

const LINK_FIND_CLOSURE_OPTIONS: FindClosureOptions = FindClosureOptions {
    code_span: false,
    nesting: false,
    newline: true,
    advance: true,
};

pub(crate) fn link_find_closure_options() -> FindClosureOptions {
    LINK_FIND_CLOSURE_OPTIONS
}

// Go: parser/link.go:linkParser.parseReferenceLink
fn parse_reference_link<'a>(
    ast: &mut Ast,
    parent: NodeId,
    last: NodeId,
    block: &mut dyn Reader<'a>,
    pc: &mut Context,
) -> (Option<NodeId>, bool) {
    let (_, orgpos) = block.position();
    block.advance(1); // skip '['
    let Some(segments) = block.find_closure(b'[', b']', LINK_FIND_CLOSURE_OPTIONS) else {
        return (None, false);
    };

    let mut maybe_reference: Vec<u8>;
    if segments.len() == 1 {
        // avoid allocate a new byte slice
        maybe_reference = block.value(segments.at(0)).into_owned();
    } else {
        maybe_reference = Vec::new();
        for i in 0..segments.len() {
            let s = segments.at(i);
            maybe_reference.extend_from_slice(&block.value(s));
        }
    }
    if util::is_blank(&maybe_reference) {
        // collapsed reference link
        let s = new_segment(ast.lls(last).segment.stop, orgpos.start - 1);
        maybe_reference = block.value(s).into_owned();
    }
    // CommonMark spec says:
    //  > A link label can have at most 999 characters inside the square brackets.
    if maybe_reference.len() > 999 {
        return (None, true);
    }

    let Some(r) = pc
        .reference(&util::to_link_reference(&maybe_reference))
        .cloned()
    else {
        return (None, true);
    };

    let link = ast.new_link();
    process_link_label(ast, parent, link, last, pc);
    let lk = ast.link_mut(link).unwrap();
    lk.title = r.title().map(|t| t.to_vec());
    lk.destination = r.destination().to_vec();
    (Some(link), true)
}

// Go: parser/link.go:linkParser.parseLink
fn parse_link<'a>(
    ast: &mut Ast,
    parent: NodeId,
    last: NodeId,
    block: &mut dyn Reader<'a>,
    pc: &mut Context,
) -> Option<NodeId> {
    block.advance(1); // skip '('
    block.skip_spaces();
    let mut title: Option<Vec<u8>> = None;
    let mut destination: Vec<u8> = Vec::new();
    if block.peek() == b')' {
        // empty link like '[link]()'
        block.advance(1);
    } else {
        let (d, ok) = parse_link_destination(block);
        if !ok {
            return None;
        }
        destination = d;
        block.skip_spaces();
        if block.peek() == b')' {
            block.advance(1);
        } else {
            let (t, ok) = parse_link_title(block);
            if !ok {
                return None;
            }
            title = t;
            block.skip_spaces();
            if block.peek() == b')' {
                block.advance(1);
            } else {
                return None;
            }
        }
    }

    let link = ast.new_link();
    process_link_label(ast, parent, link, last, pc);
    let lk = ast.link_mut(link).unwrap();
    lk.destination = destination;
    lk.title = title;
    Some(link)
}

// Go: parser/link.go:parseLinkDestination
pub(crate) fn parse_link_destination<'a>(block: &mut dyn Reader<'a>) -> (Vec<u8>, bool) {
    block.skip_spaces();
    let (line, _) = block.peek_line();
    let line = line.unwrap_or_default();
    if block.peek() == b'<' {
        let mut i = 1;
        while i < line.len() {
            let c = line[i];
            if c == b'\\' && i < line.len() - 1 && util::is_punct(line[i + 1]) {
                i += 2;
                continue;
            } else if c == b'>' {
                block.advance((i + 1) as i64);
                return (line[1..i].to_vec(), true);
            }
            i += 1;
        }
        return (Vec::new(), false);
    }
    let mut opened = 0;
    let mut i = 0;
    while i < line.len() {
        let c = line[i];
        if c == b'\\' && i < line.len() - 1 && util::is_punct(line[i + 1]) {
            i += 2;
            continue;
        } else if c == b'(' {
            opened += 1;
        } else if c == b')' {
            opened -= 1;
            if opened < 0 {
                break;
            }
        } else if util::is_space(c) {
            break;
        }
        i += 1;
    }
    block.advance(i as i64);
    (line[..i].to_vec(), i != 0)
}

// Go: parser/link.go:parseLinkTitle
fn parse_link_title<'a>(block: &mut dyn Reader<'a>) -> (Option<Vec<u8>>, bool) {
    block.skip_spaces();
    let opener = block.peek();
    if opener != b'"' && opener != b'\'' && opener != b'(' {
        return (None, false);
    }
    let mut closer = opener;
    if opener == b'(' {
        closer = b')';
    }
    block.advance(1);
    if let Some(segments) = block.find_closure(opener, closer, LINK_FIND_CLOSURE_OPTIONS) {
        if segments.len() == 1 {
            return (Some(block.value(segments.at(0)).into_owned()), true);
        }
        let mut title: Vec<u8> = Vec::new();
        for i in 0..segments.len() {
            let s = segments.at(i);
            title.extend_from_slice(&block.value(s));
        }
        return (Some(title), true);
    }
    (None, false)
}

// Go: parser/link.go:pushLinkBottom
fn push_link_bottom(pc: &mut Context) {
    let b = pc.last_delimiter();
    let bottoms = pc.get_as_mut::<LinkBottoms>(*LINK_BOTTOM);
    match bottoms {
        None => pc.set_value(*LINK_BOTTOM, LinkBottoms::One(b)),
        Some(LinkBottoms::Many(s)) => s.push(b),
        Some(LinkBottoms::One(one)) => {
            let one = *one;
            pc.set_value(*LINK_BOTTOM, LinkBottoms::Many(vec![one, b]));
        }
    }
}

// Go: parser/link.go:popLinkBottom
fn pop_link_bottom(pc: &mut Context) -> DelimiterBottom {
    let to_bottom = |v: Option<NodeId>| match v {
        Some(n) => DelimiterBottom::Node(n),
        None => DelimiterBottom::TypedNil,
    };
    let Some(bottoms) = pc.get_as_mut::<LinkBottoms>(*LINK_BOTTOM) else {
        return DelimiterBottom::Nil;
    };
    match bottoms {
        LinkBottoms::One(v) => {
            let v = *v;
            pc.set(*LINK_BOTTOM, None);
            to_bottom(v)
        }
        LinkBottoms::Many(s) => {
            let v = s.pop().expect("linkBottom stack");
            match s.len() {
                0 => pc.set(*LINK_BOTTOM, None),
                1 => {
                    let n0 = s[0];
                    pc.set_value(*LINK_BOTTOM, LinkBottoms::One(n0));
                }
                _ => {}
            }
            to_bottom(v)
        }
    }
}
