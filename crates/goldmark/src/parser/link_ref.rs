// Go: github.com/yuin/goldmark@v1.7.12/parser/link_ref.go

use super::link::{link_find_closure_options, parse_link_destination};
use super::{Context, ParagraphTransformer, new_reference};
use crate::ast::{Ast, NodeId};
use crate::text::{Reader, new_block_reader};
use crate::util;

/// LinkReferenceParagraphTransformer is a ParagraphTransformer implementation
/// that parses and extracts link reference from paragraphs.
pub struct LinkReferenceParagraphTransformer;

impl ParagraphTransformer for LinkReferenceParagraphTransformer {
    // Go: parser/link_ref.go:linkReferenceParagraphTransformer.Transform
    fn transform<'a>(
        &self,
        ast: &mut Ast,
        node: NodeId,
        reader: &mut dyn Reader<'a>,
        pc: &mut Context,
    ) {
        let mut lines = ast.lines(node).clone();
        let mut block = new_block_reader(reader.source(), Some(&lines));
        let mut removes: Vec<[i64; 2]> = Vec::new();
        loop {
            let (start, mut end) = parse_link_reference_definition(&mut block, pc);
            if start > -1 {
                if start == end {
                    end += 1;
                }
                removes.push([start, end]);
                continue;
            }
            break;
        }

        let mut offset = 0;
        for remove in &removes {
            if lines.is_empty() {
                break;
            }
            let s = lines.sliced(remove[1] - offset, lines.len());
            lines.set_sliced(0, remove[0] - offset);
            lines.append_all(&s);
            offset = remove[1];
        }

        if lines.is_empty() {
            let t = ast.new_text_block();
            let blank = ast.has_blank_previous_lines(node);
            ast.set_blank_previous_lines(t, blank);
            let parent = ast.parent(node).expect("paragraph has no parent");
            ast.replace_child(parent, node, t);
            return;
        }

        ast.set_lines(node, &lines);
    }
}

// Go: parser/link_ref.go:parseLinkReferenceDefinition
fn parse_link_reference_definition<'a>(block: &mut dyn Reader<'a>, pc: &mut Context) -> (i64, i64) {
    block.skip_spaces();
    let (line, _) = block.peek_line();
    let Some(line) = line else {
        return (-1, -1);
    };
    let (start_line, _) = block.position();
    let (width, mut pos) = util::indent_width(&line, 0);
    if width > 3 {
        return (-1, -1);
    }
    if width != 0 {
        pos += 1;
    }
    if line[pos as usize] != b'[' {
        return (-1, -1);
    }
    block.advance(pos + 1);
    let Some(segments) = block.find_closure(b'[', b']', link_find_closure_options()) else {
        return (-1, -1);
    };
    let mut label: Vec<u8>;
    if segments.len() == 1 {
        label = block.value(segments.at(0)).into_owned();
    } else {
        label = Vec::new();
        for i in 0..segments.len() {
            let s = segments.at(i);
            label.extend_from_slice(&block.value(s));
        }
    }
    if util::is_blank(&label) {
        return (-1, -1);
    }
    if block.peek() != b':' {
        return (-1, -1);
    }
    block.advance(1);
    block.skip_spaces();
    let (destination, ok) = parse_link_destination(block);
    if !ok {
        return (-1, -1);
    }
    let (line, _) = block.peek_line();
    let is_new_line = match &line {
        None => true,
        Some(l) => util::is_blank(l),
    };

    let (end_line, _) = block.position();
    let (_, spaces, _) = block.skip_spaces();
    let opener = block.peek();
    if opener != b'"' && opener != b'\'' && opener != b'(' {
        if !is_new_line {
            return (-1, -1);
        }
        let r = new_reference(label, destination, None);
        pc.add_reference(r);
        return (start_line, end_line + 1);
    }
    if spaces == 0 {
        return (-1, -1);
    }
    block.advance(1);
    let mut closer = opener;
    if opener == b'(' {
        closer = b')';
    }
    let Some(segments) = block.find_closure(opener, closer, link_find_closure_options()) else {
        if !is_new_line {
            return (-1, -1);
        }
        let r = new_reference(label, destination, None);
        pc.add_reference(r);
        block.advance_line();
        return (start_line, end_line + 1);
    };
    let mut title: Vec<u8>;
    if segments.len() == 1 {
        title = block.value(segments.at(0)).into_owned();
    } else {
        title = Vec::new();
        for i in 0..segments.len() {
            let s = segments.at(i);
            title.extend_from_slice(&block.value(s));
        }
    }

    let (line, _) = block.peek_line();
    if let Some(line) = &line
        && !util::is_blank(line)
    {
        if !is_new_line {
            return (-1, -1);
        }
        let r = new_reference(label, destination, Some(title));
        pc.add_reference(r);
        return (start_line, end_line);
    }

    let (end_line, _) = block.position();
    let r = new_reference(label, destination, Some(title));
    pc.add_reference(r);
    (start_line, end_line + 1)
}
