//! Port of Go's `encoding/csv` reader (go1.27.1 `encoding/csv/reader.go`): the subset
//! `metadecoders.Decoder.unmarshalCSV` uses (`NewReader`, `Comma`, `Comment`, `LazyQuotes`,
//! `ReadAll`; `FieldsPerRecord` stays 0, `TrimLeadingSpace` and `ReuseRecord` false).
//!
//! Owner: gaps follow-up of Wave B task T03 (parser-langs).
//!
//! The input is the whole document in memory (Go reads it through a `bufio.Reader` over a
//! `bytes.Reader`; `ReadSlice`'s buffer size does not change the records).

use go_unicode::Rune;
use go_unicode::utf8;

/// Go: `csv.ParseError` (`Err` is one of [`CsvErr`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    pub start_line: i64,
    pub line: i64,
    pub column: i64,
    pub err: CsvErr,
}

/// Go: the errors a `ParseError` wraps, and `errInvalidDelim`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CsvErr {
    BareQuote,
    Quote,
    FieldCount,
}

impl CsvErr {
    fn text(self) -> &'static str {
        match self {
            CsvErr::BareQuote => "bare \" in non-quoted-field",
            CsvErr::Quote => "extraneous or missing \" in quoted-field",
            CsvErr::FieldCount => "wrong number of fields",
        }
    }
}

/// A `ReadAll` error.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Parse(ParseError),
    /// Go: `errInvalidDelim`.
    InvalidDelim,
}

impl std::fmt::Display for Error {
    // Go: encoding/csv/reader.go:(*ParseError).Error
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidDelim => f.write_str("csv: invalid field or comment delimiter"),
            Error::Parse(e) => {
                if e.err == CsvErr::FieldCount {
                    return write!(f, "record on line {}: {}", e.line, e.err.text());
                }
                if e.start_line != e.line {
                    return write!(
                        f,
                        "record on line {}; parse error on line {}, column {}: {}",
                        e.start_line,
                        e.line,
                        e.column,
                        e.err.text()
                    );
                }
                write!(
                    f,
                    "parse error on line {}, column {}: {}",
                    e.line,
                    e.column,
                    e.err.text()
                )
            }
        }
    }
}

// Go: encoding/csv/reader.go:validDelim
fn valid_delim(r: Rune) -> bool {
    r != 0
        && r != '"' as Rune
        && r != '\r' as Rune
        && r != '\n' as Rune
        && utf8::valid_rune(r)
        && r != utf8::RUNE_ERROR
}

/// Go: `csv.Reader` (the fields `metadecoders` sets).
pub struct Reader<'a> {
    /// Comma is the field delimiter.
    pub comma: Rune,
    /// Comment, if not 0, is the comment character.
    pub comment: Rune,
    /// If LazyQuotes is true, a quote may appear in an unquoted field and a
    /// non-doubled quote may appear in a quoted field.
    pub lazy_quotes: bool,
    fields_per_record: i64,
    data: &'a [u8],
    pos: usize,
    num_line: i64,
    record_buffer: Vec<u8>,
    field_indexes: Vec<usize>,
}

/// `io.EOF` from `readLine` (the other read errors cannot happen on a byte slice).
struct Eof;

impl<'a> Reader<'a> {
    /// Go: `csv.NewReader(bytes.NewReader(data))`.
    // Go: encoding/csv/reader.go:NewReader
    pub fn new(data: &'a [u8]) -> Reader<'a> {
        Reader {
            comma: ',' as Rune,
            comment: 0,
            lazy_quotes: false,
            fields_per_record: 0,
            data,
            pos: 0,
            num_line: 0,
            record_buffer: Vec::new(),
            field_indexes: Vec::new(),
        }
    }

    /// ReadAll reads all the remaining records. `Ok(None)` is Go's nil `[][]string` (no
    /// records).
    // Go: encoding/csv/reader.go:(*Reader).ReadAll
    #[allow(clippy::type_complexity)]
    pub fn read_all(&mut self) -> Result<Option<Vec<Vec<Vec<u8>>>>, Error> {
        let mut records: Option<Vec<Vec<Vec<u8>>>> = None;
        loop {
            match self.read_record() {
                Ok(None) => return Ok(records),
                Err(e) => return Err(e),
                Ok(Some(record)) => records.get_or_insert_with(Vec::new).push(record),
            }
        }
    }

    // Go: encoding/csv/reader.go:(*Reader).readLine
    fn read_line(&mut self) -> (Vec<u8>, Option<Eof>) {
        // bufio.Reader.ReadSlice('\n') over the remaining input.
        let rest = &self.data[self.pos..];
        let (mut line, mut err) = match memchr_nl(rest) {
            Some(i) => (rest[..=i].to_vec(), None),
            None => (rest.to_vec(), Some(Eof)),
        };
        self.pos += line.len();
        let read_size = line.len();
        if read_size > 0 && err.is_some() {
            err = None;
            // For backwards compatibility, drop trailing \r before EOF.
            if line[read_size - 1] == b'\r' {
                line.truncate(read_size - 1);
            }
        }
        self.num_line += 1;
        // Normalize \r\n to \n on all input lines.
        let n = line.len();
        if n >= 2 && line[n - 2] == b'\r' && line[n - 1] == b'\n' {
            line[n - 2] = b'\n';
            line.truncate(n - 1);
        }
        (line, err)
    }

    /// One record; `Ok(None)` is Go's `io.EOF`.
    // Go: encoding/csv/reader.go:(*Reader).readRecord
    fn read_record(&mut self) -> Result<Option<Vec<Vec<u8>>>, Error> {
        if self.comma == self.comment
            || !valid_delim(self.comma)
            || (self.comment != 0 && !valid_delim(self.comment))
        {
            return Err(Error::InvalidDelim);
        }

        // Read line (automatically skipping past empty lines and any comments).
        let mut line: Vec<u8> = Vec::new();
        let mut err_read: Option<Eof> = None;
        while err_read.is_none() {
            let (l, e) = self.read_line();
            line = l;
            err_read = e;
            if self.comment != 0 && next_rune(&line) == self.comment {
                line = Vec::new();
                continue; // Skip comment lines
            }
            if err_read.is_none() && line.len() == length_nl(&line) {
                line = Vec::new();
                continue; // Skip empty lines
            }
            break;
        }
        if err_read.is_some() {
            return Ok(None);
        }

        // Parse each field in the record.
        let mut err: Option<ParseError> = None;
        const QUOTE_LEN: usize = 1;
        let comma_len = utf8::rune_len(self.comma) as usize;
        let rec_line = self.num_line; // Starting line for record
        self.record_buffer.clear();
        self.field_indexes.clear();
        let mut pos_line = self.num_line;
        let mut pos_col: i64 = 1;
        let mut comma_buf = [0u8; 4];
        let comma_n = utf8::encode_rune(&mut comma_buf, self.comma);
        let comma_bytes = &comma_buf[..comma_n];
        let mut line: &[u8] = &line;
        let mut owned_line: Vec<u8>;
        'parse_field: loop {
            if line.is_empty() || line[0] != b'"' {
                // Non-quoted string field
                let i = find(line, comma_bytes);
                let field = match i {
                    Some(i) => &line[..i],
                    None => &line[..line.len() - length_nl(line)],
                };
                // Check to make sure a quote does not appear in field.
                if !self.lazy_quotes
                    && let Some(j) = field.iter().position(|&b| b == b'"')
                {
                    let col = pos_col + j as i64;
                    err = Some(ParseError {
                        start_line: rec_line,
                        line: self.num_line,
                        column: col,
                        err: CsvErr::BareQuote,
                    });
                    break 'parse_field;
                }
                self.record_buffer.extend_from_slice(field);
                self.field_indexes.push(self.record_buffer.len());
                if let Some(i) = i {
                    line = &line[i + comma_len..];
                    pos_col += (i + comma_len) as i64;
                    continue 'parse_field;
                }
                break 'parse_field;
            } else {
                // Quoted string field
                line = &line[QUOTE_LEN..];
                pos_col += QUOTE_LEN as i64;
                loop {
                    let i = line.iter().position(|&b| b == b'"');
                    if let Some(i) = i {
                        // Hit next quote.
                        self.record_buffer.extend_from_slice(&line[..i]);
                        line = &line[i + QUOTE_LEN..];
                        pos_col += (i + QUOTE_LEN) as i64;
                        let rn = next_rune(line);
                        if rn == '"' as Rune {
                            // `""` sequence (append quote).
                            self.record_buffer.push(b'"');
                            line = &line[QUOTE_LEN..];
                            pos_col += QUOTE_LEN as i64;
                        } else if rn == self.comma {
                            // `",` sequence (end of field).
                            line = &line[comma_len..];
                            pos_col += comma_len as i64;
                            self.field_indexes.push(self.record_buffer.len());
                            continue 'parse_field;
                        } else if length_nl(line) == line.len() {
                            // `"\n` sequence (end of line).
                            self.field_indexes.push(self.record_buffer.len());
                            break 'parse_field;
                        } else if self.lazy_quotes {
                            // `"` sequence (bare quote).
                            self.record_buffer.push(b'"');
                        } else {
                            // `"*` sequence (invalid non-escaped quote).
                            err = Some(ParseError {
                                start_line: rec_line,
                                line: self.num_line,
                                column: pos_col - QUOTE_LEN as i64,
                                err: CsvErr::Quote,
                            });
                            break 'parse_field;
                        }
                    } else if !line.is_empty() {
                        // Hit end of line (copy all data so far).
                        self.record_buffer.extend_from_slice(line);
                        if err_read.is_some() {
                            break 'parse_field;
                        }
                        pos_col += line.len() as i64;
                        let (l, e) = self.read_line();
                        owned_line = l;
                        line = &owned_line;
                        if !line.is_empty() {
                            pos_line += 1;
                            pos_col = 1;
                        }
                        // Go: `if errRead == io.EOF { errRead = nil }` (the only read error).
                        let _ = e;
                        err_read = None;
                    } else {
                        // Abrupt end of file (EOF or error).
                        if !self.lazy_quotes && err_read.is_none() {
                            err = Some(ParseError {
                                start_line: rec_line,
                                line: pos_line,
                                column: pos_col,
                                err: CsvErr::Quote,
                            });
                            break 'parse_field;
                        }
                        self.field_indexes.push(self.record_buffer.len());
                        break 'parse_field;
                    }
                }
            }
        }
        // (err = errRead: errRead is nil here; a read error cannot happen.)

        // Create a single string and create slices out of it.
        let mut dst = Vec::with_capacity(self.field_indexes.len());
        let mut pre_idx = 0;
        for &idx in &self.field_indexes {
            dst.push(self.record_buffer[pre_idx..idx].to_vec());
            pre_idx = idx;
        }

        // Check or update the expected fields per record.
        if self.fields_per_record > 0 {
            if dst.len() as i64 != self.fields_per_record && err.is_none() {
                err = Some(ParseError {
                    start_line: rec_line,
                    line: rec_line,
                    column: 1,
                    err: CsvErr::FieldCount,
                });
            }
        } else if self.fields_per_record == 0 {
            self.fields_per_record = dst.len() as i64;
        }
        match err {
            Some(e) => Err(Error::Parse(e)),
            None => Ok(Some(dst)),
        }
    }
}

fn memchr_nl(b: &[u8]) -> Option<usize> {
    b.iter().position(|&c| c == b'\n')
}

/// Go: `bytes.IndexRune(line, r.Comma)` for a valid rune (its UTF-8 bytes).
fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() == 1 {
        return hay.iter().position(|&c| c == needle[0]);
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

// Go: encoding/csv/reader.go:lengthNL
fn length_nl(b: &[u8]) -> usize {
    if !b.is_empty() && b[b.len() - 1] == b'\n' {
        1
    } else {
        0
    }
}

// Go: encoding/csv/reader.go:nextRune
fn next_rune(b: &[u8]) -> Rune {
    utf8::decode_rune(b).0
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (go1.27.1 encoding/csv/reader.go, the subset metadecoders uses)
// OK (*ParseError).Error, validDelim, NewReader, ReadAll, readLine, lengthNL, nextRune,
//    readRecord (TrimLeadingSpace, ReuseRecord and FieldPos are not used by neohugo)
// ---------------------------------------------------------------------------
