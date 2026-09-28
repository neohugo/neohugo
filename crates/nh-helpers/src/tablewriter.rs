//! `github.com/olekukonko/tablewriter@v1.0.8` subset used by `helpers.ProcessingStatsTable`
//! (with `pkg/twwidth`, `pkg/twwarp`, `tw` helpers, the Blueprint renderer, go-runewidth
//! v0.0.16 and uniseg v0.2.0).
//!
//! Only the configuration Hugo builds is ported: a batch table (no streaming, captions or
//! merges), `MaxWidth: 70`, header auto-format and truncation, row word wrapping, right-aligned
//! rows with a left-aligned first column, the default one-space padding, no borders, column
//! separators and the header line of `StyleLight`. The functions keep tablewriter's names and
//! logic for that configuration; the branches the configuration never takes are left out (see
//! PORTING.md).

mod runewidth;
mod runewidth_tables;
mod uniseg;
mod uniseg_tables;

use runewidth::Condition;
use uniseg::Graphemes;

/// `tw.CharEllipsis`.
const CHAR_ELLIPSIS: &str = "…";
/// `tw.StyleLight`: `Column()`, `Row()`, `Center()`.
const SYM_COLUMN: &str = "│";
const SYM_ROW: &str = "─";
const SYM_CENTER: &str = "┼";
/// `tw.PaddingDefault`.
const PAD_LEFT: &str = " ";
const PAD_RIGHT: &str = " ";
/// The table's `MaxWidth` (Hugo's `WithConfig`).
const MAX_WIDTH: i64 = 70;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Align {
    Left,
    Right,
    Center,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AutoWrap {
    Normal,
    Truncate,
}

/// The section configuration Hugo's table ends up with (after `mergeConfig` and `Header()`).
struct CellConfig {
    auto_wrap: AutoWrap,
    auto_format: bool,
    global_align: Align,
    per_column_align: &'static [Align],
}

const HEADER: CellConfig = CellConfig {
    auto_wrap: AutoWrap::Truncate,
    // mergeConfig resets AutoFormat to the zero value; Header() then sets it On.
    auto_format: true,
    global_align: Align::Center,
    per_column_align: &[],
};

const ROW: CellConfig = CellConfig {
    auto_wrap: AutoWrap::Normal,
    auto_format: false,
    global_align: Align::Right,
    per_column_align: &[Align::Left],
};

// ---------------------------------------------------------------------------
// pkg/twwidth

fn condition() -> Condition {
    Condition {
        east_asian_width: runewidth::east_asian_width(),
    }
}

/// Go twwidth `ansi.ReplaceAllLiteralString(str, "")`: removes CSI (`ESC [ params
/// intermediates final`) and OSC (`ESC ] ... ST`, non-greedy, no newline) sequences.
// Go: pkg/twwidth/width.go:Filter
fn strip_ansi(s: &str) -> std::borrow::Cow<'_, str> {
    if !s.contains('\x1b') {
        return std::borrow::Cow::Borrowed(s);
    }
    let b = s.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == 0x1b && i + 1 < b.len() {
            if b[i + 1] == b'[' {
                let mut j = i + 2;
                while j < b.len() && (0x30..=0x3f).contains(&b[j]) {
                    j += 1;
                }
                while j < b.len() && (0x20..=0x2f).contains(&b[j]) {
                    j += 1;
                }
                if j < b.len() && (0x40..=0x7e).contains(&b[j]) {
                    i = j + 1;
                    continue;
                }
            } else if b[i + 1] == b']' {
                let mut j = i + 2;
                let mut end = None;
                while j < b.len() && b[j] != b'\n' {
                    if b[j] == 0x07 {
                        end = Some(j + 1);
                        break;
                    }
                    if b[j] == 0x1b && j + 1 < b.len() && b[j + 1] == b'\\' {
                        end = Some(j + 2);
                        break;
                    }
                    j += 1;
                }
                if let Some(e) = end {
                    i = e;
                    continue;
                }
            }
        }
        out.push(b[i]);
        i += 1;
    }
    std::borrow::Cow::Owned(String::from_utf8(out).expect("only whole ASCII sequences removed"))
}

/// Go: `twwidth.Width(str)`.
// Go: pkg/twwidth/width.go:Width
fn width(s: &str) -> i64 {
    condition().string_width(&strip_ansi(s))
}

/// Go: `twwidth.Truncate(s, maxWidth, suffix...)`.
// Go: pkg/twwidth/width.go:Truncate
fn truncate(s: &str, max_width: i64, suffix: &str) -> String {
    if max_width < 0 {
        return String::new();
    }

    let s_display_width = width(s);
    let suffix_display_width = width(suffix);

    if s_display_width == 0 {
        if !suffix.is_empty() && suffix_display_width <= max_width {
            return suffix.to_string();
        }
        if max_width == 0 && !s.is_empty() {
            return String::new();
        }
        return s.to_string();
    }

    if max_width == 0 {
        return String::new();
    }

    if s_display_width <= max_width {
        if suffix.is_empty() {
            return s.to_string();
        }
        if s_display_width + suffix_display_width <= max_width {
            return format!("{s}{suffix}");
        }
        return s.to_string();
    }

    let ea = runewidth::east_asian_width();
    if !suffix.is_empty() && ea {
        let provisional_content_width = max_width - suffix_display_width;
        if provisional_content_width == 0 {
            return suffix.to_string();
        }
    }

    let mut target = max_width;
    if !suffix.is_empty() {
        target -= suffix_display_width;
    }

    if target < 0 {
        if !suffix.is_empty() && suffix_display_width <= max_width {
            if s.contains("\x1b[") {
                return format!("\x1b[0m{suffix}");
            }
            return suffix.to_string();
        }
        return String::new();
    }

    let cond = condition();
    let mut content = String::new();
    let mut current_width = 0i64;
    let mut ansi_seq: Vec<char> = Vec::new();
    let mut in_ansi = false;
    let mut ansi_written = false;
    for r in s.chars() {
        if r == '\x1b' {
            in_ansi = true;
            ansi_seq.clear();
            ansi_seq.push(r);
        } else if in_ansi {
            ansi_seq.push(r);
            let seq: String = ansi_seq.iter().collect();
            let sb = seq.as_bytes();
            let n = sb.len();
            let mut terminated = false;
            if n >= 2 {
                let intro = sb[1];
                if intro == b'[' {
                    if n >= 3 && (0x40..=0x7e).contains(&(r as u32)) {
                        terminated = true;
                    }
                } else if intro == b']'
                    && (r == '\x07' || (n > 1 && sb[n - 2] == 0x1b && r == '\\'))
                {
                    terminated = true;
                }
            }
            if terminated {
                in_ansi = false;
                content.push_str(&seq);
                ansi_written = true;
                ansi_seq.clear();
            }
        } else {
            let rw = cond.rune_width(r as i32);
            if target == 0 {
                break;
            }
            if current_width + rw > target {
                break;
            }
            content.push(r);
            current_width += rw;
        }
    }

    let mut result = content;
    if !suffix.is_empty() {
        let mut needs_reset = false;
        // Go: `(ansiWritten || (inAnsi && strings.Contains(s, "\x1b["))) && (currentWidth > 0 || ansiWritten)`.
        if ansi_written || (in_ansi && s.contains("\x1b[") && current_width > 0) {
            if !result.ends_with("\x1b[0m") {
                needs_reset = true;
            }
        } else if current_width > 0
            && result.contains("\x1b[")
            && !result.ends_with("\x1b[0m")
            && s.contains("\x1b[")
        {
            needs_reset = true;
        }
        if needs_reset {
            result.push_str("\x1b[0m");
        }
        result.push_str(suffix);
    }
    result
}

// ---------------------------------------------------------------------------
// tw helpers

// Go: tw/fn.go:IsIsNumericOrSpace
fn is_numeric_or_space(r: char) -> bool {
    r.is_ascii_digit() || r == ' '
}

/// Go: `tw.Title(name)`.
// Go: tw/fn.go:Title
fn title(name: &str) -> String {
    let orig_len = name.len();
    let mut rs: Vec<char> = name.chars().collect();
    for i in 0..rs.len() {
        match rs[i] {
            '_' => rs[i] = ' ',
            '.' => {
                if (i != 0 && !is_numeric_or_space(rs[i - 1]))
                    || (i != rs.len() - 1 && !is_numeric_or_space(rs[i + 1]))
                {
                    rs[i] = ' ';
                }
            }
            _ => {}
        }
    }
    let name: String = rs.into_iter().collect();
    let mut name = go_unicode::strings::trim_space_str(&name).to_string();
    if name.is_empty() && orig_len > 0 {
        name = " ".to_string();
    }
    go_unicode::strings::to_upper_str(&name).into_owned()
}

/// Go: `tw.SplitCamelCase(src)`.
// Go: tw/fn.go:SplitCamelCase
fn split_camel_case(src: &str) -> Vec<String> {
    let mut runes: Vec<Vec<char>> = Vec::new();
    let mut last_class = 0;
    for r in src.chars() {
        let ri = r as i32;
        let class = if go_unicode::is_lower(ri) {
            1
        } else if go_unicode::is_upper(ri) {
            2
        } else if go_unicode::is_digit(ri) {
            3
        } else {
            4
        };
        if class == last_class {
            runes.last_mut().expect("a previous run").push(r);
        } else {
            runes.push(vec![r]);
        }
        last_class = class;
    }
    // handle upper case -> lower case sequences, e.g. "PDFL", "oader" -> "PDF", "Loader"
    let mut i = 0;
    while i + 1 < runes.len() {
        if go_unicode::is_upper(runes[i][0] as i32) && go_unicode::is_lower(runes[i + 1][0] as i32)
        {
            let last = runes[i].pop().expect("non-empty run");
            runes[i + 1].insert(0, last);
        }
        i += 1;
    }
    let mut entries = Vec::new();
    for s in &runes {
        let s: String = s.iter().collect();
        if !s.is_empty() && !go_unicode::strings::trim_space_str(&s).is_empty() && s != "_" {
            entries.push(s);
        }
    }
    entries
}

/// Go: `tw.PadRight(s, pad, width)`.
// Go: tw/fn.go:PadRight
fn pad_right(s: &str, pad: &str, w: i64) -> String {
    let gap = w - width(s);
    if gap > 0 {
        return format!("{s}{}", pad.repeat(gap as usize));
    }
    s.to_string()
}

/// Go: `tw.PadLeft(s, pad, width)`.
// Go: tw/fn.go:PadLeft
fn pad_left(s: &str, pad: &str, w: i64) -> String {
    let gap = w - width(s);
    if gap > 0 {
        return format!("{}{s}", pad.repeat(gap as usize));
    }
    s.to_string()
}

// ---------------------------------------------------------------------------
// pkg/twwarp

/// Go: `twwarp.SplitWords(s)` (words separated by `unicode.IsSpace`).
// Go: pkg/twwarp/wrap.go:SplitWords
fn split_words(s: &str) -> Vec<&str> {
    let mut words = Vec::new();
    let mut word_begin = 0;
    let mut word_pending = false;
    for (i, c) in s.char_indices() {
        if go_unicode::is_space(c as i32) {
            if word_pending {
                words.push(&s[word_begin..i]);
                word_pending = false;
            }
            continue;
        }
        if !word_pending {
            word_begin = i;
            word_pending = true;
        }
    }
    if word_pending {
        words.push(&s[word_begin..]);
    }
    words
}

/// Go: `twwarp.WrapStringWithSpaces(s, lim)` (the lines only).
// Go: pkg/twwarp/wrap.go:WrapStringWithSpaces
fn wrap_string_with_spaces(s: &str, lim: i64) -> Vec<String> {
    if s.is_empty() {
        return vec![String::new()];
    }
    let is_space = |c: char| go_unicode::is_space(c as i32);
    if go_unicode::strings::trim_space_str(s).is_empty() {
        // All spaces
        if width(s) <= lim {
            return vec![s.to_string()];
        }
        if lim > 0 {
            return vec![string_to_display_width(s, lim)];
        }
        return vec![String::new()];
    }

    let first_non_space = s.find(|c: char| !is_space(c)).expect("not all spaces");
    let leading_spaces = &s[..first_non_space];
    let (last_idx, last_char) = s
        .char_indices()
        .rev()
        .find(|&(_, c)| !is_space(c))
        .expect("not all spaces");
    let last_non_space_end = last_idx + last_char.len_utf8();
    let trailing_spaces = &s[last_non_space_end..];
    let core_content = &s[first_non_space..last_non_space_end];

    if core_content.is_empty() {
        return vec![format!("{leading_spaces}{trailing_spaces}")];
    }

    let words = split_words(core_content);
    if words.is_empty() {
        return vec![format!("{leading_spaces}{trailing_spaces}")];
    }

    let mut current_lim = lim;
    let max_core_word_width = words.iter().map(|w| width(w)).max().unwrap_or(0);
    if max_core_word_width > current_lim {
        current_lim = max_core_word_width;
    }

    let wrapped = wrap_words(&words, 1, current_lim, 100_000);
    let n = wrapped.len();
    wrapped
        .into_iter()
        .enumerate()
        .map(|(i, line_words)| {
            let mut final_line = format!("{leading_spaces}{}", line_words.join(" "));
            if i == n - 1 {
                // Last line
                final_line.push_str(trailing_spaces);
            }
            final_line
        })
        .collect()
}

/// Go: `stringToDisplayWidth(s, targetWidth)` (the substring).
// Go: pkg/twwarp/wrap.go:stringToDisplayWidth
fn string_to_display_width(s: &str, target_width: i64) -> String {
    if target_width <= 0 {
        return String::new();
    }
    let mut current_width = 0;
    let mut end_index = 0;
    let mut g = Graphemes::new(s);
    while g.next() {
        let (start, end) = g.positions();
        let grapheme_width = width(&s[start..end]);
        if current_width + grapheme_width > target_width {
            break;
        }
        current_width += grapheme_width;
        end_index = end;
    }
    s[..end_index].to_string()
}

/// Go: `twwarp.WrapWords(words, spc, lim, pen)` — minimal raggedness line breaking.
// Go: pkg/twwarp/wrap.go:WrapWords
fn wrap_words<'a>(words: &[&'a str], spc: i64, lim: i64, pen: i64) -> Vec<Vec<&'a str>> {
    let n = words.len();
    if n == 0 {
        return Vec::new();
    }
    let lengths: Vec<i64> = words.iter().map(|w| width(w)).collect();
    let mut nbrk = vec![0usize; n];
    let mut cost = vec![i32::MAX as i64; n];
    let mut remainder_len = lengths[n - 1];
    for i in (0..n).rev() {
        if i < n - 1 {
            remainder_len += spc + lengths[i];
        }
        if remainder_len <= lim {
            cost[i] = 0;
            nbrk[i] = n;
            continue;
        }
        let mut phrase_len = lengths[i];
        for j in i + 1..n {
            if j > i + 1 {
                phrase_len += spc + lengths[j - 1];
            }
            let d = lim - phrase_len;
            let mut c = d.wrapping_mul(d).wrapping_add(cost[j]);
            if phrase_len > lim {
                c = c.wrapping_add(pen); // too-long lines get a worse penalty
            }
            if c < cost[i] {
                cost[i] = c;
                nbrk[i] = j;
            }
        }
    }
    let mut lines = Vec::new();
    let mut i = 0;
    while i < n {
        lines.push(words[i..nbrk[i]].to_vec());
        i = nbrk[i];
    }
    lines
}

// ---------------------------------------------------------------------------
// Table

/// Go: `(*Table).prepareContent(cells, config)` for a batch table with `MaxWidth` set and no
/// `Widths`: each cell auto-formatted (header), then wrapped or truncated to
/// `floor(MaxWidth/numCols) - padding`.
// Go: tablewriter.go:prepareContent
fn prepare_content(cells: &[String], config: &CellConfig) -> Vec<Vec<String>> {
    let effective_num_cols = cells.len();
    let mut result: Vec<Vec<String>> = Vec::new();

    let mut col_max_width_global = 0i64;
    if effective_num_cols > 0 {
        col_max_width_global = (MAX_WIDTH as f64 / effective_num_cols as f64).floor() as i64;
    }

    for (i, cell) in cells.iter().enumerate() {
        // t.Trimmer: TrimSpace is not enabled (mergeConfig resets it to the zero value).
        let mut cell_content = cell.clone();

        let pad_left_width = width(PAD_LEFT);
        let pad_right_width = width(PAD_RIGHT);

        let effective_content_max_width =
            calculate_content_max_width(col_max_width_global, pad_left_width, pad_right_width);

        if config.auto_format {
            cell_content = title(&split_camel_case(&cell_content).join(" "));
        }

        let mut final_lines: Vec<String> = Vec::new();
        for line in cell_content.split('\n') {
            if effective_content_max_width > 0 {
                match config.auto_wrap {
                    AutoWrap::Normal => {
                        final_lines
                            .extend(wrap_string_with_spaces(line, effective_content_max_width));
                    }
                    AutoWrap::Truncate => {
                        if width(line) > effective_content_max_width {
                            let ellipsis_width = width(CHAR_ELLIPSIS);
                            if effective_content_max_width >= ellipsis_width {
                                final_lines.push(truncate(
                                    line,
                                    effective_content_max_width - ellipsis_width,
                                    CHAR_ELLIPSIS,
                                ));
                            } else {
                                final_lines.push(truncate(line, effective_content_max_width, ""));
                            }
                        } else {
                            final_lines.push(line.to_string());
                        }
                    }
                }
            } else {
                final_lines.push(line.to_string());
            }
        }

        while result.len() < final_lines.len() {
            result.push(vec![String::new(); effective_num_cols]);
        }

        for (j, row) in result.iter_mut().enumerate() {
            row[i] = final_lines.get(j).cloned().unwrap_or_default();
        }
    }

    result
}

/// Go: `calculateContentMaxWidth` with only the legacy `ColMaxWidths.Global` constraint (which
/// `prepareContent` derives from `MaxWidth`).
// Go: zoo.go:calculateContentMaxWidth
fn calculate_content_max_width(col_max_width_global: i64, pad_left: i64, pad_right: i64) -> i64 {
    if col_max_width_global > 0 {
        let mut w = col_max_width_global - pad_left - pad_right;
        if w < 1 && col_max_width_global > pad_left + pad_right {
            w = 1;
        } else if w < 0 {
            w = 0;
        }
        return w;
    }
    0
}

/// Go: `(*Table).updateWidths(row, widths, padding)`.
// Go: zoo.go:updateWidths
fn update_widths(row: &[String], widths: &mut Vec<i64>) {
    for (i, cell) in row.iter().enumerate() {
        let pad_left_width = width(PAD_LEFT);
        let pad_right_width = width(PAD_RIGHT);

        let content_width = cell.split('\n').map(width).max().unwrap_or(0);

        let mut total_width = content_width + pad_left_width + pad_right_width;
        let min_required = pad_left_width + pad_right_width;
        if content_width == 0 && total_width < min_required {
            total_width = min_required;
        }
        if total_width < 1 {
            total_width = 1;
        }
        if widths.len() <= i {
            widths.resize(i + 1, 0);
        }
        if total_width > widths[i] {
            widths[i] = total_width;
        }
    }
}

/// Go: `padLine(line, numCols)`.
// Go: option.go:padLine
fn pad_line(line: &[String], num_cols: usize) -> Vec<String> {
    let mut l = line.to_vec();
    if l.len() < num_cols {
        l.resize(num_cols, String::new());
    }
    l
}

/// Go: `(*Table).buildAligns(config)`.
// Go: tablewriter.go:buildAligns
fn build_aligns(config: &CellConfig, num_cols: usize) -> Vec<Align> {
    (0..num_cols)
        .map(|i| {
            config
                .per_column_align
                .get(i)
                .copied()
                .unwrap_or(config.global_align)
        })
        .collect()
}

/// Go: the Blueprint renderer's `formatCell(content, width, padding, align)`.
// Go: renderer/blueprint.go:formatCell
fn format_cell(content: &str, w: i64, align: Align) -> String {
    if w <= 0 {
        return String::new();
    }

    let mut content = content.to_string();
    let mut rune_width = width(&content);

    let left_pad_char = PAD_LEFT;
    let right_pad_char = PAD_RIGHT;
    let pad_left_width = width(left_pad_char);
    let pad_right_width = width(right_pad_char);

    let available = (w - pad_left_width - pad_right_width).max(0);

    if rune_width > available {
        content = truncate(&content, available, "");
        rune_width = width(&content);
    }

    let total_padding_width = (w - rune_width).max(0);

    let mut result = String::new();
    match align {
        Align::Left => {
            result.push_str(left_pad_char);
            result.push_str(&content);
            let right_padding_width = total_padding_width - pad_left_width;
            if right_padding_width > 0 {
                result.push_str(&pad_right("", right_pad_char, right_padding_width));
            }
        }
        Align::Right => {
            let left_padding_width = total_padding_width - pad_right_width;
            if left_padding_width > 0 {
                result.push_str(&pad_left("", left_pad_char, left_padding_width));
            }
            result.push_str(&content);
            result.push_str(right_pad_char);
        }
        Align::Center => {
            let left_padding_width =
                (total_padding_width - pad_left_width - pad_right_width) / 2 + pad_left_width;
            let right_padding_width = total_padding_width - left_padding_width;
            if left_padding_width > pad_left_width {
                result.push_str(&pad_left(
                    "",
                    left_pad_char,
                    left_padding_width - pad_left_width,
                ));
            }
            result.push_str(left_pad_char);
            result.push_str(&content);
            result.push_str(right_pad_char);
            if right_padding_width > pad_right_width {
                result.push_str(&pad_right(
                    "",
                    right_pad_char,
                    right_padding_width - pad_right_width,
                ));
            }
        }
    }

    let final_width = width(&result);
    if final_width > w {
        result = truncate(&result, w, "");
    } else if final_width < w {
        result = pad_right(&result, " ", w);
    }
    result
}

/// Go: the Blueprint renderer's `renderLine(ctx)` without borders or merges.
// Go: renderer/blueprint.go:renderLine
fn render_line(out: &mut Vec<u8>, line: &[String], widths: &[i64], aligns: &[Align]) {
    let mut output = String::new();
    let num_cols = widths.len();
    for col in 0..num_cols {
        let visual_width = widths[col];
        if visual_width == 0 {
            continue;
        }
        // Separators::BetweenColumns is on.
        if col > 0 && widths[col - 1] > 0 {
            output.push_str(SYM_COLUMN);
        }
        let Some(data) = line.get(col) else {
            // Go: no cell context.
            output.push_str(&" ".repeat(visual_width as usize));
            continue;
        };
        output.push_str(&format_cell(data, visual_width, aligns[col]));
    }
    output.push('\n');
    out.extend_from_slice(output.as_bytes());
}

/// Go: the Blueprint renderer's `Line(ctx)` for the line below the header (no borders, no
/// merges: `─` segments joined by `┼`).
// Go: renderer/blueprint.go:Line
fn render_separator_line(out: &mut Vec<u8>, widths: &[i64]) {
    let mut line = String::new();
    let visible: Vec<i64> = widths.iter().copied().filter(|&w| w > 0).collect();
    for (k, &col_width) in visible.iter().enumerate() {
        let segment_width = width(SYM_ROW).max(1);
        let mut repeat = col_width / segment_width;
        if repeat < 1 && col_width > 0 {
            repeat = 1;
        }
        line.push_str(&SYM_ROW.repeat(repeat as usize));
        if k != visible.len() - 1 {
            line.push_str(SYM_CENTER);
        }
    }
    line.push('\n');
    out.extend_from_slice(line.as_bytes());
}

/// `table.Bulk(data); table.Header(names); table.Render()` with Hugo's configuration.
// Go: tablewriter.go:Render
pub(crate) fn render_processing_stats(
    w: &mut dyn std::io::Write,
    names: &[String],
    data: &[Vec<String>],
) {
    // Bulk: each row through prepareContent.
    let rows: Vec<Vec<Vec<String>>> = data.iter().map(|r| prepare_content(r, &ROW)).collect();
    // Header: prepareContent with the header config.
    let headers = prepare_content(names, &HEADER);

    // maxColumns.
    let mut num_cols = 0;
    if let Some(h) = headers.first() {
        num_cols = num_cols.max(h.len());
    }
    for row in &rows {
        if let Some(l) = row.first() {
            num_cols = num_cols.max(l.len());
        }
    }

    // calculateAndNormalizeWidths.
    let mut header_widths = Vec::new();
    for line in &headers {
        update_widths(line, &mut header_widths);
    }
    let mut row_widths: Vec<i64> = Vec::new();
    for row in &rows {
        let mut cache = Vec::new();
        for line in row {
            update_widths(line, &mut cache);
            for (col, &wd) in cache.iter().enumerate() {
                if row_widths.len() <= col {
                    row_widths.resize(col + 1, 0);
                }
                if wd > row_widths[col] {
                    row_widths[col] = wd;
                }
            }
        }
    }
    let get = |v: &Vec<i64>, i: usize| v.get(i).copied().unwrap_or(0);
    let widths: Vec<i64> = (0..num_cols)
        .map(|i| {
            let wd = get(&header_widths, i).max(get(&row_widths, i));
            if wd == 0 { 1 } else { wd }
        })
        .collect();

    // prepareWithMerges pads each line to the width of its section's first line.
    let pad_section = |lines: &[Vec<String>]| -> Vec<Vec<String>> {
        let n = lines.first().map(|l| l.len()).unwrap_or(0);
        lines.iter().map(|l| pad_line(l, n)).collect()
    };
    let headers = pad_section(&headers);
    let rows: Vec<Vec<Vec<String>>> = rows.iter().map(|r| pad_section(r)).collect();

    let mut out = Vec::new();

    // renderHeader.
    if !headers.is_empty() {
        let aligns = build_aligns(&HEADER, num_cols);
        for line in &headers {
            render_line(&mut out, &pad_line(line, num_cols), &widths, &aligns);
        }
        // ShowHeaderLine, when there are rows (or a footer).
        if !rows.is_empty() {
            render_separator_line(&mut out, &widths);
        }
    }

    // renderRow.
    let aligns = build_aligns(&ROW, num_cols);
    for lines in &rows {
        for (j, line) in lines.iter().enumerate() {
            let line = pad_line(line, num_cols);
            // Visual lines after the first are skipped when they have no content.
            if j > 0 && line.iter().all(|c| c.is_empty()) {
                continue;
            }
            render_line(&mut out, &line, &widths, &aligns);
        }
    }

    // renderFooter: no footer and no bottom border.
    let _ = w.write_all(&out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn widths() {
        assert_eq!(width("abc"), 3);
        assert_eq!(width("ไทย"), 3);
        assert_eq!(width("日本語"), 6);
        assert_eq!(width("\x1b[31mred\x1b[0m"), 3);
        assert_eq!(truncate("abcdef", 3, "…"), "ab…");
        assert_eq!(title("zh - cn"), "ZH - CN");
        assert_eq!(split_camel_case("PDFLoader"), vec!["PDF", "Loader"]);
    }

    fn fixture() -> serde_json::Value {
        use std::io::Read;
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/general/widths.json.gz");
        let mut s = String::new();
        flate2::read::GzDecoder::new(std::fs::File::open(p).unwrap())
            .read_to_string(&mut s)
            .unwrap();
        serde_json::from_str(&s).unwrap()
    }

    /// go-runewidth's RuneWidth for every code point (the oracle's run-length encoding).
    #[test]
    fn rune_width_matches_go() {
        let fx = fixture();
        for ea in [false, true] {
            let runs = fx[format!("runs_{ea}")].as_array().unwrap();
            let c = Condition {
                east_asian_width: ea,
            };
            for (i, run) in runs.iter().enumerate() {
                let start = run[0].as_i64().unwrap() as i32;
                let w = run[1].as_i64().unwrap();
                let end = runs
                    .get(i + 1)
                    .map(|r| r[0].as_i64().unwrap() as i32)
                    .unwrap_or(0x110001);
                for r in start..end {
                    assert_eq!(c.rune_width(r), w, "U+{r:X} ea={ea}");
                }
            }
        }
    }

    /// uniseg's grapheme clusters and go-runewidth's StringWidth.
    #[test]
    fn graphemes_match_go() {
        let fx = fixture();
        let mut n = 0;
        for g in fx["graphemes"].as_array().unwrap() {
            let s = g[0].as_str().unwrap();
            let mut pos = Vec::new();
            let mut it = Graphemes::new(s);
            while it.next() {
                let (a, b) = it.positions();
                pos.push(serde_json::json!([a, b]));
            }
            let want = if g[1].is_null() {
                serde_json::json!([])
            } else {
                g[1].clone()
            };
            assert_eq!(serde_json::Value::Array(pos), want, "{s:?}");
            for (k, ea) in [(2, false), (3, true)] {
                let c = Condition {
                    east_asian_width: ea,
                };
                assert_eq!(c.string_width(s), g[k].as_i64().unwrap(), "{s:?} ea={ea}");
            }
            n += 1;
        }
        assert!(n > 3000);
    }
}
