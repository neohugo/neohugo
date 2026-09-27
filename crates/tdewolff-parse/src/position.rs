//! Go: parse/position.go

use crate::input::{GoReader, Input};
use crate::utf8::{is_graphic, runes_to_string, to_runes};

// Go: parse/position.go:Position
/// Returns the line and column number for a certain position in a file, and
/// the context (the line itself with a caret). Only `\n`, `\r`, `\r\n`,
/// U+2028 and U+2029 are newlines.
pub fn position(r: Option<&mut dyn GoReader>, offset: isize) -> (isize, isize, Vec<u8>) {
    let l = Input::new(r);
    position_input(&l, offset)
}

/// [`position`] on an already constructed `Input` (it is consumed: its
/// position is moved).
pub fn position_input(l: &Input, mut offset: isize) -> (isize, isize, Vec<u8>) {
    let mut line: isize = 1;
    while (l.pos() as isize) < offset {
        let c = l.peek(0);
        let mut n: usize = 1;
        let mut newline = false;
        if c == b'\n' {
            newline = true;
        } else if c == b'\r' {
            if l.peek(1) == b'\n' {
                newline = true;
                n = 2;
            } else {
                newline = true;
            }
        } else if c >= 0xC0 {
            let (r, rn) = l.peek_rune(0);
            n = rn;
            if r == 0x2028 || r == 0x2029 {
                newline = true;
            }
        } else if c == 0 && l.has_err() {
            break;
        }

        if 1 < n && offset < (l.pos() + n) as isize {
            break;
        }
        l.move_(n as isize);

        if newline {
            line += 1;
            offset -= l.pos() as isize;
            l.skip();
        }
    }

    let col = to_runes(&l.lexeme()).len() as isize + 1;
    let context = position_context(l, line, col);
    (line, col, context)
}

// Go: parse/position.go:positionContext
fn position_context(l: &Input, line: isize, mut col: isize) -> Vec<u8> {
    loop {
        let c = l.peek(0);
        if c == 0 && l.has_err() || c == b'\n' || c == b'\r' {
            break;
        }
        l.move_(1);
    }
    let mut rs = to_runes(&l.lexeme());

    // cut off front or rear of context to stay between 60 characters
    let limit: isize = 60;
    let offset: isize = 20;
    let mut ellipsis_front: &[u8] = b"";
    let mut ellipsis_rear: &[u8] = b"";
    let n = rs.len() as isize;
    if limit < n {
        if col <= limit - offset {
            ellipsis_rear = b"...";
            rs.truncate((limit - 3) as usize);
        } else if col >= n - offset - 3 {
            ellipsis_front = b"...";
            col -= n - offset - offset - 7;
            rs = rs[(n - offset - offset - 4) as usize..].to_vec();
        } else {
            ellipsis_front = b"...";
            ellipsis_rear = b"...";
            rs = rs[(col - offset - 1) as usize..(col + offset) as usize].to_vec();
            col = offset + 4;
        }
    }

    // replace unprintable characters by a space
    for r in rs.iter_mut() {
        if !is_graphic(*r) {
            *r = 0xB7; // '·'
        }
    }

    let mut context = Vec::new();
    context.extend_from_slice(format!("{:5}: ", line).as_bytes());
    context.extend_from_slice(ellipsis_front);
    context.extend_from_slice(&runes_to_string(&rs));
    context.extend_from_slice(ellipsis_rear);
    context.push(b'\n');
    assert!(6 + col >= 0, "strings: negative Repeat count");
    context.extend(std::iter::repeat_n(b' ', (6 + col) as usize));
    context.push(b'^');
    context
}
