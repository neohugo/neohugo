//! Module `pflag`.
//!
//! NEW: the subset of `github.com/spf13/pflag` v1.0.6 that cobra uses to parse the neohugo command
//! line, plus the `encoding/csv` record reader/writer behind its string slice flags.
//!
//! Owner: Wave B task T25 (commands-cli).
//!
//! Only the flag value types neohugo declares are supported (`bool`, `string`, `stringSlice`).
//! Parsing follows `(*FlagSet).parseArgs` exactly: interspersed arguments, `--` terminator,
//! `--flag=value` / `--flag value` / `-f value` / `-fvalue` / `-f=value`, combined boolean
//! shorthands (`-DFE`), the bool `NoOptDefVal` ("true"), and Go's error texts.

use std::collections::BTreeMap;

use go_strconv::quote;

/// The value types neohugo's flags use (Go: `pflag.Value.Type()`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FlagType {
    Bool,
    String,
    StringSlice,
}

impl FlagType {
    /// Go: `Value.Type()`.
    pub fn go_name(&self) -> &'static str {
        match self {
            FlagType::Bool => "bool",
            FlagType::String => "string",
            FlagType::StringSlice => "stringSlice",
        }
    }
}

/// The current value of a flag.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Value {
    Bool(bool),
    String(String),
    /// `changed` is the string slice value's own "set once" flag (Go `stringSliceValue.changed`).
    StringSlice {
        value: Vec<String>,
        changed: bool,
    },
}

/// Go: `pflag.Flag`.
#[derive(Clone, Debug)]
pub struct Flag {
    pub name: String,
    /// One ASCII letter, or "".
    pub shorthand: String,
    pub usage: String,
    pub value: Value,
    pub def_value: String,
    /// Go `NoOptDefVal`: the value of `--flag` without an argument ("true" for bools).
    pub no_opt_def_val: String,
    pub changed: bool,
    pub hidden: bool,
}

impl Flag {
    /// Go: `Value.Type()`.
    pub fn flag_type(&self) -> FlagType {
        match self.value {
            Value::Bool(_) => FlagType::Bool,
            Value::String(_) => FlagType::String,
            Value::StringSlice { .. } => FlagType::StringSlice,
        }
    }

    /// Go: `Value.String()`.
    // Go: pflag bool.go/string.go/string_slice.go:String
    pub fn value_string(&self) -> String {
        match &self.value {
            Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
            Value::String(s) => s.clone(),
            Value::StringSlice { value, .. } => format!("[{}]", write_as_csv(value)),
        }
    }

    /// Go: `Value.Set(s)`.
    // Go: pflag bool.go/string.go/string_slice.go:Set
    fn set_value(&mut self, s: &str) -> Result<(), String> {
        match &mut self.value {
            Value::Bool(b) => {
                // Go: `v, err := strconv.ParseBool(s); *b.value = v; return err`.
                match go_strconv::parse_bool(s) {
                    Ok(v) => {
                        *b = v;
                        Ok(())
                    }
                    Err(e) => {
                        *b = false;
                        Err(e.to_string())
                    }
                }
            }
            Value::String(v) => {
                *v = s.to_string();
                Ok(())
            }
            Value::StringSlice { value, changed } => {
                let v = read_as_csv(s)?;
                if !*changed {
                    *value = v;
                } else {
                    value.extend(v);
                }
                *changed = true;
                Ok(())
            }
        }
    }
}

/// Go: `pflag.FlagSet` (ContinueOnError, interspersed, sorted flags).
#[derive(Clone, Debug, Default)]
pub struct FlagSet {
    /// Go `formal`, keyed by name (sorted, as `VisitAll` with `SortFlags`).
    pub formal: BTreeMap<String, Flag>,
    /// Go `shorthands`: shorthand letter -> flag name.
    pub shorthands: BTreeMap<u8, String>,
    /// Go `args`: the arguments that are not flags.
    pub args: Vec<String>,
    /// Go `argsLenAtDash` (-1 when there was no `--`).
    pub args_len_at_dash: i64,
}

impl FlagSet {
    pub fn new() -> Self {
        FlagSet {
            args_len_at_dash: -1,
            ..Default::default()
        }
    }

    // Go: pflag flag.go:(*FlagSet).AddFlag
    fn add_flag(&mut self, flag: Flag) {
        if self.formal.contains_key(&flag.name) {
            // Go panics ("flag redefined"); cobra never redefines a flag.
            return;
        }
        if !flag.shorthand.is_empty() {
            self.shorthands
                .insert(flag.shorthand.as_bytes()[0], flag.name.clone());
        }
        self.formal.insert(flag.name.clone(), flag);
    }

    /// Adds `flag` unless a flag with its name exists (cobra's `AddFlagSet` when merging the
    /// persistent flags of the parents).
    pub fn add_flag_if_absent(&mut self, flag: &Flag) {
        if !self.formal.contains_key(&flag.name) {
            self.add_flag(flag.clone());
        }
    }

    /// Go: `BoolP(name, shorthand, value, usage)`.
    pub fn bool_p(&mut self, name: &str, shorthand: &str, value: bool, usage: &str) {
        self.add_flag(Flag {
            name: name.to_string(),
            shorthand: shorthand.to_string(),
            usage: usage.to_string(),
            value: Value::Bool(value),
            def_value: if value { "true" } else { "false" }.to_string(),
            no_opt_def_val: "true".to_string(),
            changed: false,
            hidden: false,
        });
    }

    /// Go: `StringP(name, shorthand, value, usage)`.
    pub fn string_p(&mut self, name: &str, shorthand: &str, value: &str, usage: &str) {
        self.add_flag(Flag {
            name: name.to_string(),
            shorthand: shorthand.to_string(),
            usage: usage.to_string(),
            value: Value::String(value.to_string()),
            def_value: value.to_string(),
            no_opt_def_val: String::new(),
            changed: false,
            hidden: false,
        });
    }

    /// Go: `StringSliceP(name, shorthand, value, usage)`.
    pub fn string_slice_p(&mut self, name: &str, shorthand: &str, value: &[&str], usage: &str) {
        let value: Vec<String> = value.iter().map(|s| s.to_string()).collect();
        self.add_flag(Flag {
            name: name.to_string(),
            shorthand: shorthand.to_string(),
            usage: usage.to_string(),
            def_value: format!("[{}]", write_as_csv(&value)),
            value: Value::StringSlice {
                value,
                changed: false,
            },
            no_opt_def_val: String::new(),
            changed: false,
            hidden: false,
        });
    }

    /// Go: `MarkHidden(name)`.
    pub fn mark_hidden(&mut self, name: &str) {
        if let Some(f) = self.formal.get_mut(name) {
            f.hidden = true;
        }
    }

    /// Go: `Lookup(name)`.
    pub fn lookup(&self, name: &str) -> Option<&Flag> {
        self.formal.get(name)
    }

    /// Go: `Changed(name)`.
    pub fn changed(&self, name: &str) -> bool {
        self.formal.get(name).map(|f| f.changed).unwrap_or(false)
    }

    /// Go: `GetBool(name)` (false for a missing or non-bool flag).
    pub fn get_bool(&self, name: &str) -> bool {
        matches!(
            self.formal.get(name).map(|f| &f.value),
            Some(Value::Bool(true))
        )
    }

    /// Go: `GetString(name)`.
    pub fn get_string(&self, name: &str) -> String {
        match self.formal.get(name).map(|f| &f.value) {
            Some(Value::String(s)) => s.clone(),
            _ => String::new(),
        }
    }

    /// Go: `GetStringSlice(name)`.
    pub fn get_string_slice(&self, name: &str) -> Vec<String> {
        match self.formal.get(name).map(|f| &f.value) {
            Some(Value::StringSlice { value, .. }) => value.clone(),
            _ => Vec::new(),
        }
    }

    /// Go: `VisitAll(fn)` (sorted by name).
    pub fn visit_all(&self) -> impl Iterator<Item = &Flag> {
        self.formal.values()
    }

    /// Go: `hasNoOptDefVal(name, fs)` (cobra).
    pub fn has_no_opt_def_val(&self, name: &str) -> bool {
        match self.formal.get(name) {
            Some(f) => !f.no_opt_def_val.is_empty(),
            None => false,
        }
    }

    /// Go: `shortHasNoOptDefVal(name, fs)` (cobra).
    pub fn short_has_no_opt_def_val(&self, name: &str) -> bool {
        if name.is_empty() {
            return false;
        }
        let c = name.as_bytes()[0];
        match self.shorthands.get(&c).and_then(|n| self.formal.get(n)) {
            Some(f) => !f.no_opt_def_val.is_empty(),
            None => false,
        }
    }

    /// Go: `(*FlagSet).Set(name, value)`.
    // Go: pflag flag.go:(*FlagSet).Set
    pub fn set(&mut self, name: &str, value: &str) -> Result<(), String> {
        let Some(flag) = self.formal.get_mut(name) else {
            return Err(format!("no such flag -{name}"));
        };
        if let Err(err) = flag.set_value(value) {
            let flag_name = if !flag.shorthand.is_empty() {
                format!("-{}, --{}", flag.shorthand, flag.name)
            } else {
                format!("--{}", flag.name)
            };
            return Err(format!(
                "invalid argument {} for {} flag: {}",
                quote(value),
                quote(&flag_name),
                err
            ));
        }
        flag.changed = true;
        Ok(())
    }

    /// Go: `(*FlagSet).Parse(arguments)` (ContinueOnError). `Err("help")` is Go's `ErrHelp`
    /// (`--help`/`-h` when no such flag is defined; cobra defines one).
    // Go: pflag flag.go:(*FlagSet).Parse
    pub fn parse(&mut self, arguments: &[String]) -> Result<(), ParseError> {
        self.args = Vec::with_capacity(arguments.len());
        self.parse_args(arguments)
    }

    // Go: pflag flag.go:(*FlagSet).parseArgs
    fn parse_args(&mut self, args: &[String]) -> Result<(), ParseError> {
        let mut args: Vec<String> = args.to_vec();
        while !args.is_empty() {
            let s = args.remove(0);
            let b = s.as_bytes();
            if b.is_empty() || b[0] != b'-' || b.len() == 1 {
                // Interspersed.
                self.args.push(s);
                continue;
            }
            if b[1] == b'-' {
                if b.len() == 2 {
                    // "--" terminates the flags
                    self.args_len_at_dash = self.args.len() as i64;
                    self.args.append(&mut args);
                    break;
                }
                args = self.parse_long_arg(&s, args)?;
            } else {
                args = self.parse_short_arg(&s, args)?;
            }
        }
        Ok(())
    }

    // Go: pflag flag.go:(*FlagSet).parseLongArg
    fn parse_long_arg(&mut self, s: &str, args: Vec<String>) -> Result<Vec<String>, ParseError> {
        let mut a = args;
        let name = &s[2..];
        if name.is_empty() || name.starts_with('-') || name.starts_with('=') {
            return Err(ParseError::Msg(format!("bad flag syntax: {s}")));
        }
        let (name, split_value) = match name.split_once('=') {
            Some((n, v)) => (n, Some(v.to_string())),
            None => (name, None),
        };
        let Some(flag) = self.formal.get(name) else {
            if name == "help" {
                return Err(ParseError::Help);
            }
            return Err(ParseError::Msg(format!("unknown flag: --{name}")));
        };
        let value = if let Some(v) = split_value {
            // '--flag=arg'
            v
        } else if !flag.no_opt_def_val.is_empty() {
            // '--flag' (arg was optional)
            flag.no_opt_def_val.clone()
        } else if !a.is_empty() {
            // '--flag arg'
            a.remove(0)
        } else {
            // '--flag' (arg was required)
            return Err(ParseError::Msg(format!("flag needs an argument: {s}")));
        };
        let name = name.to_string();
        self.set(&name, &value).map_err(ParseError::Msg)?;
        Ok(a)
    }

    // Go: pflag flag.go:(*FlagSet).parseSingleShortArg
    fn parse_single_short_arg(
        &mut self,
        shorthands: &str,
        args: &[String],
    ) -> Result<(String, Vec<String>), ParseError> {
        let mut out_args = args.to_vec();
        if shorthands.starts_with("test.") {
            return Ok((String::new(), out_args));
        }
        let sb = shorthands.as_bytes();
        let mut out_shorts = String::from_utf8_lossy(&sb[1..]).into_owned();
        let c = sb[0];
        let Some(name) = self.shorthands.get(&c).cloned() else {
            if c == b'h' {
                return Err(ParseError::Help);
            }
            return Err(ParseError::Msg(format!(
                "unknown shorthand flag: {} in -{}",
                go_quote_byte_rune(c),
                shorthands
            )));
        };
        let flag = &self.formal[&name];
        let value = if sb.len() > 2 && sb[1] == b'=' {
            // '-f=arg'
            out_shorts = String::new();
            String::from_utf8_lossy(&sb[2..]).into_owned()
        } else if !flag.no_opt_def_val.is_empty() {
            // '-f' (arg was optional)
            flag.no_opt_def_val.clone()
        } else if sb.len() > 1 {
            // '-farg'
            out_shorts = String::new();
            String::from_utf8_lossy(&sb[1..]).into_owned()
        } else if !args.is_empty() {
            // '-f arg'
            out_args = args[1..].to_vec();
            args[0].clone()
        } else {
            // '-f' (arg was required)
            return Err(ParseError::Msg(format!(
                "flag needs an argument: {} in -{}",
                go_quote_byte_rune(c),
                shorthands
            )));
        };
        self.set(&name, &value).map_err(ParseError::Msg)?;
        Ok((out_shorts, out_args))
    }

    // Go: pflag flag.go:(*FlagSet).parseShortArg
    fn parse_short_arg(&mut self, s: &str, args: Vec<String>) -> Result<Vec<String>, ParseError> {
        let mut a = args.clone();
        let mut shorthands = s[1..].to_string();
        // "shorthands" can be a series of shorthand letters of flags (e.g. "-vvv").
        // Go passes the original args to every parseSingleShortArg call.
        while !shorthands.is_empty() {
            let (sh, out) = self.parse_single_short_arg(&shorthands, &args)?;
            shorthands = sh;
            a = out;
        }
        Ok(a)
    }
}

/// A flag parse failure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    /// Go `pflag.ErrHelp` ("pflag: help requested").
    Help,
    Msg(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::Help => f.write_str("pflag: help requested"),
            ParseError::Msg(m) => f.write_str(m),
        }
    }
}

/// Go `%q` of a `byte` (a rune literal: `'x'`).
fn go_quote_byte_rune(c: u8) -> String {
    go_strconv::quote_rune(c as go_strconv::Rune)
}

/// Go: pflag `readAsCSV(val)`: `csv.NewReader(strings.NewReader(val)).Read()`.
// Go: pflag string_slice.go:readAsCSV
pub fn read_as_csv(val: &str) -> Result<Vec<String>, String> {
    if val.is_empty() {
        return Ok(Vec::new());
    }
    csv_read_record(val.as_bytes())
}

/// Go: pflag `writeAsCSV(vals)`: one `csv.Writer` record without the trailing newline.
// Go: pflag string_slice.go:writeAsCSV
pub fn write_as_csv(vals: &[String]) -> String {
    let mut out = String::new();
    for (n, field) in vals.iter().enumerate() {
        if n > 0 {
            out.push(',');
        }
        if !field_needs_quotes(field) {
            out.push_str(field);
            continue;
        }
        out.push('"');
        for c in field.chars() {
            match c {
                '"' => out.push_str("\"\""),
                _ => out.push(c),
            }
        }
        out.push('"');
    }
    out
}

// Go: encoding/csv writer.go:(*Writer).fieldNeedsQuotes (Comma ',').
fn field_needs_quotes(field: &str) -> bool {
    if field.is_empty() {
        return false;
    }
    if field == "\\." {
        return true;
    }
    if field.contains([',', '"', '\r', '\n']) {
        return true;
    }
    let (r1, _) = go_unicode::utf8::decode_rune(field.as_bytes());
    go_unicode::is_space(r1)
}

/// Go: `(*csv.Reader).readRecord` for the first record of `input` (Comma ',', no comments,
/// strict quotes). Errors are Go's texts (`parse error on line 1, column 2: bare " in
/// non-quoted-field`, `EOF`).
// Go: encoding/csv reader.go:(*Reader).readRecord
fn csv_read_record(input: &[u8]) -> Result<Vec<String>, String> {
    struct LineReader<'a> {
        rest: &'a [u8],
        num_line: i64,
    }
    impl<'a> LineReader<'a> {
        // Go: encoding/csv reader.go:(*Reader).readLine. `None` is io.EOF with no data.
        fn read_line(&mut self) -> (Vec<u8>, bool) {
            if self.rest.is_empty() {
                self.num_line += 1;
                return (Vec::new(), true);
            }
            let (mut line, eof) = match self.rest.iter().position(|&b| b == b'\n') {
                Some(i) => {
                    let l = self.rest[..=i].to_vec();
                    self.rest = &self.rest[i + 1..];
                    (l, false)
                }
                None => {
                    let l = self.rest.to_vec();
                    self.rest = &[];
                    (l, true)
                }
            };
            let mut eof_err = false;
            if !line.is_empty() && eof {
                // For backwards compatibility, drop trailing \r before EOF.
                if line[line.len() - 1] == b'\r' {
                    line.pop();
                }
            } else if eof {
                eof_err = true;
            }
            self.num_line += 1;
            // Normalize \r\n to \n on all input lines.
            let n = line.len();
            if n >= 2 && line[n - 2] == b'\r' && line[n - 1] == b'\n' {
                line[n - 2] = b'\n';
                line.pop();
            }
            (line, eof_err)
        }
    }
    fn length_nl(b: &[u8]) -> usize {
        if !b.is_empty() && b[b.len() - 1] == b'\n' {
            1
        } else {
            0
        }
    }
    fn parse_error(start_line: i64, line: i64, col: i64, err: &str) -> String {
        if start_line != line {
            format!("record on line {start_line}; parse error on line {line}, column {col}: {err}")
        } else {
            format!("parse error on line {line}, column {col}: {err}")
        }
    }
    const ERR_BARE_QUOTE: &str = "bare \" in non-quoted-field";
    const ERR_QUOTE: &str = "extraneous or missing \" in quoted-field";

    let mut r = LineReader {
        rest: input,
        num_line: 0,
    };
    // Read line (automatically skipping past empty lines).
    let mut line: Vec<u8>;
    let mut err_read: bool; // io.EOF
    loop {
        let (l, e) = r.read_line();
        line = l;
        err_read = e;
        if !err_read && line.len() == length_nl(&line) {
            continue; // Skip empty lines
        }
        break;
    }
    if err_read {
        return Err("EOF".to_string());
    }

    let rec_line = r.num_line;
    let mut record_buffer: Vec<u8> = Vec::new();
    let mut field_indexes: Vec<usize> = Vec::new();
    let mut pos_line = r.num_line;
    let mut pos_col: i64 = 1;
    let mut err: Option<String> = None;
    let mut line: &[u8] = &line;
    let mut owned_line: Vec<u8>;
    'parse_field: loop {
        if line.is_empty() || line[0] != b'"' {
            // Non-quoted string field
            let i = line.iter().position(|&b| b == b',');
            let field = match i {
                Some(i) => &line[..i],
                None => &line[..line.len() - length_nl(line)],
            };
            // Check to make sure a quote does not appear in field.
            if let Some(j) = field.iter().position(|&b| b == b'"') {
                let col = pos_col + j as i64;
                err = Some(parse_error(rec_line, r.num_line, col, ERR_BARE_QUOTE));
                break 'parse_field;
            }
            record_buffer.extend_from_slice(field);
            field_indexes.push(record_buffer.len());
            if let Some(i) = i {
                line = &line[i + 1..];
                pos_col += i as i64 + 1;
                continue 'parse_field;
            }
            break 'parse_field;
        } else {
            // Quoted string field
            line = &line[1..];
            pos_col += 1;
            loop {
                if let Some(i) = line.iter().position(|&b| b == b'"') {
                    // Hit next quote.
                    record_buffer.extend_from_slice(&line[..i]);
                    line = &line[i + 1..];
                    pos_col += i as i64 + 1;
                    let (rn, _) = go_unicode::utf8::decode_rune(line);
                    if rn == '"' as i32 {
                        // `""` sequence (append quote).
                        record_buffer.push(b'"');
                        line = &line[1..];
                        pos_col += 1;
                    } else if rn == ',' as i32 {
                        // `",` sequence (end of field).
                        line = &line[1..];
                        pos_col += 1;
                        field_indexes.push(record_buffer.len());
                        continue 'parse_field;
                    } else if length_nl(line) == line.len() {
                        // `"\n` sequence (end of line).
                        field_indexes.push(record_buffer.len());
                        break 'parse_field;
                    } else {
                        // `"*` sequence (invalid non-escaped quote).
                        err = Some(parse_error(rec_line, r.num_line, pos_col - 1, ERR_QUOTE));
                        break 'parse_field;
                    }
                } else if !line.is_empty() {
                    // Hit end of line (copy all data so far).
                    record_buffer.extend_from_slice(line);
                    if err_read {
                        break 'parse_field;
                    }
                    pos_col += line.len() as i64;
                    let (l, e) = r.read_line();
                    owned_line = l;
                    line = &owned_line;
                    err_read = e;
                    if !line.is_empty() {
                        pos_line += 1;
                        pos_col = 1;
                    }
                    if err_read {
                        err_read = false;
                    }
                } else {
                    // Abrupt end of file (EOF or error).
                    if !err_read {
                        err = Some(parse_error(rec_line, pos_line, pos_col, ERR_QUOTE));
                        break 'parse_field;
                    }
                    field_indexes.push(record_buffer.len());
                    break 'parse_field;
                }
            }
        }
    }
    if let Some(e) = err {
        return Err(e);
    }
    if err_read {
        return Err("EOF".to_string());
    }
    let mut out = Vec::with_capacity(field_indexes.len());
    let mut pre = 0;
    for idx in field_indexes {
        out.push(String::from_utf8_lossy(&record_buffer[pre..idx]).into_owned());
        pre = idx;
    }
    Ok(out)
}

// ---------------------------------------------------------------------------
// GO PORTING CHECKLIST (github.com/spf13/pflag v1.0.6 as cobra uses it for neohugo's flags)
// OK flag.go: (*FlagSet).Parse, parseArgs, parseLongArg, parseShortArg, parseSingleShortArg,
//    Set, Lookup, Changed, VisitAll (sorted), AddFlag, MarkHidden
// OK bool.go / string.go / string_slice.go: Set, String, Type, BoolP, StringP, StringSliceP,
//    readAsCSV, writeAsCSV
// OK encoding/csv: (*Reader).readRecord (first record, strict quotes), readLine, lengthNL,
//    (*ParseError).Error, (*Writer).fieldNeedsQuotes
//    Other value types (int, duration, ...), normalization funcs, deprecation, usage: not used
// ---------------------------------------------------------------------------
