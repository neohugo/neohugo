//! Input reader: BOM detection, UTF-8/UTF-16 decoding into the working buffer.
//!
//! Go: gopkg.in/yaml.v2@v2.4.0 readerc.go (+ the string read handler from apic.go).

use crate::yamlh::{Encoding, ErrorType, Parser};

// Go: readerc.go:yaml_parser_set_reader_error
fn yaml_parser_set_reader_error(
    parser: &mut Parser,
    problem: &str,
    offset: i64,
    value: i64,
) -> bool {
    parser.error = ErrorType::Reader;
    parser.problem = problem.to_string();
    parser.problem_offset = offset;
    parser.problem_value = value;
    false
}

const BOM_UTF8: &[u8] = b"\xef\xbb\xbf";
const BOM_UTF16LE: &[u8] = b"\xff\xfe";
const BOM_UTF16BE: &[u8] = b"\xfe\xff";

// Go: readerc.go:yaml_parser_determine_encoding
fn yaml_parser_determine_encoding(parser: &mut Parser) -> bool {
    // Ensure that we had enough bytes in the raw buffer.
    while !parser.eof && parser.raw_len - parser.raw_buffer_pos < 3 {
        if !yaml_parser_update_raw_buffer(parser) {
            return false;
        }
    }

    // Determine the encoding.
    let buf = &parser.raw_buffer;
    let pos = parser.raw_buffer_pos;
    let avail = parser.raw_len - pos;
    if avail >= 2 && buf[pos] == BOM_UTF16LE[0] && buf[pos + 1] == BOM_UTF16LE[1] {
        parser.encoding = Encoding::Utf16Le;
        parser.raw_buffer_pos += 2;
        parser.offset += 2;
    } else if avail >= 2 && buf[pos] == BOM_UTF16BE[0] && buf[pos + 1] == BOM_UTF16BE[1] {
        parser.encoding = Encoding::Utf16Be;
        parser.raw_buffer_pos += 2;
        parser.offset += 2;
    } else if avail >= 3
        && buf[pos] == BOM_UTF8[0]
        && buf[pos + 1] == BOM_UTF8[1]
        && buf[pos + 2] == BOM_UTF8[2]
    {
        parser.encoding = Encoding::Utf8;
        parser.raw_buffer_pos += 3;
        parser.offset += 3;
    } else {
        parser.encoding = Encoding::Utf8;
    }
    true
}

// Go: apic.go:yaml_string_read_handler. Returns (n, eof).
fn yaml_string_read_handler(parser: &mut Parser, start: usize, end: usize) -> (usize, bool) {
    if parser.input_pos == parser.input.len() {
        return (0, true);
    }
    let n = (end - start).min(parser.input.len() - parser.input_pos);
    parser.raw_buffer[start..start + n]
        .copy_from_slice(&parser.input[parser.input_pos..parser.input_pos + n]);
    parser.input_pos += n;
    (n, false)
}

// Go: readerc.go:yaml_parser_update_raw_buffer
fn yaml_parser_update_raw_buffer(parser: &mut Parser) -> bool {
    // Return if the raw buffer is full.
    if parser.raw_buffer_pos == 0 && parser.raw_len == parser.raw_buffer.len() {
        return true;
    }

    // Return on EOF.
    if parser.eof {
        return true;
    }

    // Move the remaining bytes in the raw buffer to the beginning.
    if parser.raw_buffer_pos > 0 && parser.raw_buffer_pos < parser.raw_len {
        parser
            .raw_buffer
            .copy_within(parser.raw_buffer_pos..parser.raw_len, 0);
    }
    parser.raw_len -= parser.raw_buffer_pos;
    parser.raw_buffer_pos = 0;

    // Call the read handler to fill the buffer.
    let cap = parser.raw_buffer.len();
    let (size_read, eof) = yaml_string_read_handler(parser, parser.raw_len, cap);
    parser.raw_len += size_read;
    if eof {
        parser.eof = true;
    }
    true
}

// Go: readerc.go:yaml_parser_update_buffer
//
// Ensure that the buffer contains at least `length` characters.
pub(crate) fn yaml_parser_update_buffer(parser: &mut Parser, length: i64) -> bool {
    // Return if the buffer contains enough characters.
    if parser.unread >= length {
        return true;
    }

    // Determine the input encoding if it is not known yet.
    if parser.encoding == Encoding::Any && !yaml_parser_determine_encoding(parser) {
        return false;
    }

    // Move the unread characters to the beginning of the buffer.
    let mut buffer_len = parser.buffer_len;
    if parser.buffer_pos > 0 && parser.buffer_pos < buffer_len {
        parser.buffer.copy_within(parser.buffer_pos..buffer_len, 0);
        buffer_len -= parser.buffer_pos;
        parser.buffer_pos = 0;
    } else if parser.buffer_pos == buffer_len {
        buffer_len = 0;
        parser.buffer_pos = 0;
    }

    // Open the whole buffer for writing, and cut it before returning.
    // (The backing Vec always has the full capacity.)

    // Fill the buffer until it has enough characters.
    let mut first = true;
    while parser.unread < length {
        // Fill the raw buffer if necessary.
        if (!first || parser.raw_buffer_pos == parser.raw_len)
            && !yaml_parser_update_raw_buffer(parser)
        {
            parser.buffer_len = buffer_len;
            return false;
        }
        first = false;

        // Decode the raw buffer.
        'inner: while parser.raw_buffer_pos != parser.raw_len {
            let mut value: u32;
            let width: usize;

            let raw_unread = parser.raw_len - parser.raw_buffer_pos;

            // Decode the next character.
            match parser.encoding {
                Encoding::Utf8 => {
                    // Determine the length of the UTF-8 sequence.
                    let mut octet = parser.raw_buffer[parser.raw_buffer_pos];
                    width = if octet & 0x80 == 0x00 {
                        1
                    } else if octet & 0xE0 == 0xC0 {
                        2
                    } else if octet & 0xF0 == 0xE0 {
                        3
                    } else if octet & 0xF8 == 0xF0 {
                        4
                    } else {
                        // The leading octet is invalid.
                        return yaml_parser_set_reader_error(
                            parser,
                            "invalid leading UTF-8 octet",
                            parser.offset,
                            octet as i64,
                        );
                    };

                    // Check if the raw buffer contains an incomplete character.
                    if width > raw_unread {
                        if parser.eof {
                            return yaml_parser_set_reader_error(
                                parser,
                                "incomplete UTF-8 octet sequence",
                                parser.offset,
                                -1,
                            );
                        }
                        break 'inner;
                    }

                    // Decode the leading octet.
                    value = if octet & 0x80 == 0x00 {
                        (octet & 0x7F) as u32
                    } else if octet & 0xE0 == 0xC0 {
                        (octet & 0x1F) as u32
                    } else if octet & 0xF0 == 0xE0 {
                        (octet & 0x0F) as u32
                    } else if octet & 0xF8 == 0xF0 {
                        (octet & 0x07) as u32
                    } else {
                        0
                    };

                    // Check and decode the trailing octets.
                    for k in 1..width {
                        octet = parser.raw_buffer[parser.raw_buffer_pos + k];

                        // Check if the octet is valid.
                        if (octet & 0xC0) != 0x80 {
                            return yaml_parser_set_reader_error(
                                parser,
                                "invalid trailing UTF-8 octet",
                                parser.offset + k as i64,
                                octet as i64,
                            );
                        }

                        // Decode the octet.
                        value = (value << 6) + (octet & 0x3F) as u32;
                    }

                    // Check the length of the sequence against the value.
                    let ok = width == 1
                        || (width == 2 && value >= 0x80)
                        || (width == 3 && value >= 0x800)
                        || (width == 4 && value >= 0x10000);
                    if !ok {
                        return yaml_parser_set_reader_error(
                            parser,
                            "invalid length of a UTF-8 sequence",
                            parser.offset,
                            -1,
                        );
                    }

                    // Check the range of the value.
                    if (0xD800..=0xDFFF).contains(&value) || value > 0x10FFFF {
                        return yaml_parser_set_reader_error(
                            parser,
                            "invalid Unicode character",
                            parser.offset,
                            value as i64,
                        );
                    }
                }
                Encoding::Utf16Le | Encoding::Utf16Be => {
                    let (low, high) = if parser.encoding == Encoding::Utf16Le {
                        (0, 1)
                    } else {
                        (1, 0)
                    };

                    // Check for incomplete UTF-16 character.
                    if raw_unread < 2 {
                        if parser.eof {
                            return yaml_parser_set_reader_error(
                                parser,
                                "incomplete UTF-16 character",
                                parser.offset,
                                -1,
                            );
                        }
                        break 'inner;
                    }

                    // Get the character.
                    value = parser.raw_buffer[parser.raw_buffer_pos + low] as u32
                        + ((parser.raw_buffer[parser.raw_buffer_pos + high] as u32) << 8);

                    // Check for unexpected low surrogate area.
                    if value & 0xFC00 == 0xDC00 {
                        return yaml_parser_set_reader_error(
                            parser,
                            "unexpected low surrogate area",
                            parser.offset,
                            value as i64,
                        );
                    }

                    // Check for a high surrogate area.
                    if value & 0xFC00 == 0xD800 {
                        width = 4;

                        // Check for incomplete surrogate pair.
                        if raw_unread < 4 {
                            if parser.eof {
                                return yaml_parser_set_reader_error(
                                    parser,
                                    "incomplete UTF-16 surrogate pair",
                                    parser.offset,
                                    -1,
                                );
                            }
                            break 'inner;
                        }

                        // Get the next character.
                        let value2 = parser.raw_buffer[parser.raw_buffer_pos + low + 2] as u32
                            + ((parser.raw_buffer[parser.raw_buffer_pos + high + 2] as u32) << 8);

                        // Check for a low surrogate area.
                        if value2 & 0xFC00 != 0xDC00 {
                            return yaml_parser_set_reader_error(
                                parser,
                                "expected low surrogate area",
                                parser.offset + 2,
                                value2 as i64,
                            );
                        }

                        // Generate the value of the surrogate pair.
                        value = 0x10000 + ((value & 0x3FF) << 10) + (value2 & 0x3FF);
                    } else {
                        width = 2;
                    }
                }
                Encoding::Any => {
                    // Go: panic("impossible")
                    return yaml_parser_set_reader_error(parser, "impossible", parser.offset, -1);
                }
            }

            // Check if the character is in the allowed range:
            //      #x9 | #xA | #xD | [#x20-#x7E]               (8 bit)
            //      | #x85 | [#xA0-#xD7FF] | [#xE000-#xFFFD]    (16 bit)
            //      | [#x10000-#x10FFFF]                        (32 bit)
            let allowed = value == 0x09
                || value == 0x0A
                || value == 0x0D
                || (0x20..=0x7E).contains(&value)
                || value == 0x85
                || (0xA0..=0xD7FF).contains(&value)
                || (0xE000..=0xFFFD).contains(&value)
                || (0x10000..=0x10FFFF).contains(&value);
            if !allowed {
                return yaml_parser_set_reader_error(
                    parser,
                    "control characters are not allowed",
                    parser.offset,
                    value as i64,
                );
            }

            // Move the raw pointers.
            parser.raw_buffer_pos += width;
            parser.offset += width as i64;

            // Guard against overflowing the fixed backing store (Go would
            // panic with an index error; this cannot happen with the
            // buffer sizes used).
            if buffer_len + 4 > parser.buffer.len() {
                let n = parser.buffer.len() * 2;
                parser.buffer.resize(n, 0);
            }

            // Finally put the character into the buffer.
            let b = &mut parser.buffer;
            if value <= 0x7F {
                // 0000 0000-0000 007F . 0xxxxxxx
                b[buffer_len] = value as u8;
                buffer_len += 1;
            } else if value <= 0x7FF {
                // 0000 0080-0000 07FF . 110xxxxx 10xxxxxx
                b[buffer_len] = (0xC0 + (value >> 6)) as u8;
                b[buffer_len + 1] = (0x80 + (value & 0x3F)) as u8;
                buffer_len += 2;
            } else if value <= 0xFFFF {
                // 0000 0800-0000 FFFF . 1110xxxx 10xxxxxx 10xxxxxx
                b[buffer_len] = (0xE0 + (value >> 12)) as u8;
                b[buffer_len + 1] = (0x80 + ((value >> 6) & 0x3F)) as u8;
                b[buffer_len + 2] = (0x80 + (value & 0x3F)) as u8;
                buffer_len += 3;
            } else {
                // 0001 0000-0010 FFFF . 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx
                b[buffer_len] = (0xF0 + (value >> 18)) as u8;
                b[buffer_len + 1] = (0x80 + ((value >> 12) & 0x3F)) as u8;
                b[buffer_len + 2] = (0x80 + ((value >> 6) & 0x3F)) as u8;
                b[buffer_len + 3] = (0x80 + (value & 0x3F)) as u8;
                buffer_len += 4;
            }

            parser.unread += 1;
        }

        // On EOF, put NUL into the buffer and return.
        if parser.eof {
            if buffer_len + 1 > parser.buffer.len() {
                let n = parser.buffer.len() * 2;
                parser.buffer.resize(n, 0);
            }
            parser.buffer[buffer_len] = 0;
            buffer_len += 1;
            parser.unread += 1;
            break;
        }
    }
    // [Go] To return true, we need to have the given length in the buffer.
    while (buffer_len as i64) < length {
        if buffer_len + 1 > parser.buffer.len() {
            let n = parser.buffer.len() * 2;
            parser.buffer.resize(n, 0);
        }
        parser.buffer[buffer_len] = 0;
        buffer_len += 1;
    }
    parser.buffer_len = buffer_len;
    true
}
