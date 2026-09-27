//! Go: minify/svg/pathdata.go

use tdewolff_parse::strconv::parse_float;
use tdewolff_parse::{GoBytes, SubView};

use crate::common::number;

/// Go: svg.PathDataState — the state of the current path.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PathDataState {
    cmd: u8,
    prev_digit: bool,
    prev_digit_is_int: bool,
    prev_flag: bool,
}

/// Go: svg.PathData — represents a path data string.
pub struct PathData {
    /// `p.o.Precision`
    precision: i64,
    /// `p.o.newPrecision`
    new_precision: i64,

    x: f64,
    y: f64,
    x0: f64,
    y0: f64,
    coords: Vec<GoBytes>,
    coord_floats: Vec<f64>,
    cx: f64, // last control point for cubic bezier
    cy: f64,
    qx: f64, // last control point for quadratic bezier
    qy: f64,

    state: PathDataState,
    cur_buffer: Vec<u8>,
    alt_buffer: Vec<u8>,
}

// Go: svg/pathdata.go:pathCmds
fn path_cmds(c: u8) -> bool {
    matches!(
        c,
        b'M' | b'm'
            | b'L'
            | b'l'
            | b'H'
            | b'h'
            | b'V'
            | b'v'
            | b'Q'
            | b'q'
            | b'T'
            | b't'
            | b'C'
            | b'c'
            | b'S'
            | b's'
            | b'A'
            | b'a'
            | b'Z'
            | b'z'
    )
}

impl PathData {
    // Go: svg/pathdata.go:NewPathData
    /// Returns a new PathData for the given minifier options. As in Go, a
    /// `Minifier` that did not go through `Minify` has `newPrecision == 0`.
    pub fn new(o: &super::Minifier) -> PathData {
        PathData::with_precisions(o.precision, 0)
    }

    /// `NewPathData(o)` where `o.Precision`/`o.newPrecision` are given
    /// explicitly (as set up by `Minifier.Minify`).
    pub fn with_precisions(precision: i64, new_precision: i64) -> PathData {
        PathData {
            precision,
            new_precision,
            x: 0.0,
            y: 0.0,
            x0: 0.0,
            y0: 0.0,
            coords: Vec::new(),
            coord_floats: Vec::new(),
            cx: f64::NAN,
            cy: f64::NAN,
            qx: f64::NAN,
            qy: f64::NAN,
            state: PathDataState::default(),
            cur_buffer: Vec::new(),
            alt_buffer: Vec::new(),
        }
    }

    // Go: svg/pathdata.go:PathData.ShortenPathData
    /// Takes a full pathdata string and returns a shortened version. The
    /// original string is overwritten. It parses all commands (M, A, Z, ...)
    /// and coordinates (numbers) and calls copyInstruction for each command.
    pub fn shorten_path_data(&mut self, b: GoBytes) -> GoBytes {
        if 100000 < b.len() {
            // prevent extremely long paths for being too costly (OSS-Fuzz)
            return b;
        }

        let mut cmd: u8 = 0;
        self.x = 0.0;
        self.y = 0.0;
        self.coords.clear();
        self.coord_floats.clear();
        self.state = PathDataState::default();

        let mut j = 0usize;
        let mut i = 0usize;
        while i < b.len() {
            let c = b.at(i);
            if c == b' ' || c == b',' || c == b'\n' || c == b'\r' || c == b'\t' {
                i += 1;
                continue;
            } else if path_cmds(c) && (cmd == 0 || cmd != c || c == b'M' || c == b'm') {
                // any command
                if cmd != 0 {
                    j += self.copy_instruction(b.slice_from(j), cmd);
                }
                cmd = c;
                self.coords.clear();
                self.coord_floats.clear();
            } else if (cmd == b'A' || cmd == b'a')
                && (self.coord_floats.len() % 7 == 3 || self.coord_floats.len() % 7 == 4)
            {
                // boolean flags for arc command
                if c == b'1' {
                    self.coords.push(b.slice(i, i + 1));
                    self.coord_floats.push(1.0);
                } else if c == b'0' {
                    self.coords.push(b.slice(i, i + 1));
                    self.coord_floats.push(0.0);
                } else {
                    cmd = 0; // bad format, don't minify
                }
            } else {
                let n = tdewolff_parse::number(&SubView::new(&b, i));
                if n > 0 {
                    let (f, _) = parse_float(&b.slice(i, i + n));
                    self.coords.push(b.slice(i, i + n));
                    self.coord_floats.push(f);
                    i += n - 1;
                }
            }
            i += 1;
        }
        if cmd == 0 {
            return b;
        }
        j += self.copy_instruction(b.slice_from(j), cmd);
        b.slice_to(j)
    }

    // Go: svg/pathdata.go:PathData.copyInstruction
    /// Copies pathdata of a single command, but may be comprised of multiple
    /// sets for that command. `b` is the destination and is not read from.
    fn copy_instruction(&mut self, b: GoBytes, mut cmd: u8) -> usize {
        let n = self.coords.len();
        if n == 0 {
            if cmd == b'Z' || cmd == b'z' {
                self.x = self.x0;
                self.y = self.y0;
                b.set(0, b'z');
                return 1;
            }
            return 0;
        }
        let is_rel_cmd = cmd >= b'a';

        // get new cursor coordinates
        let di: usize;
        if (cmd == b'M' || cmd == b'm' || cmd == b'L' || cmd == b'l' || cmd == b'T' || cmd == b't')
            && n % 2 == 0
        {
            di = 2;
            // reprint M always, as the first pair is a move but subsequent pairs are L
            if cmd == b'M' || cmd == b'm' {
                self.state.cmd = 0;
            }
        } else if cmd == b'H' || cmd == b'h' || cmd == b'V' || cmd == b'v' {
            di = 1;
        } else if (cmd == b'S' || cmd == b's' || cmd == b'Q' || cmd == b'q') && n % 4 == 0 {
            di = 4;
        } else if (cmd == b'C' || cmd == b'c') && n % 6 == 0 {
            di = 6;
        } else if (cmd == b'A' || cmd == b'a') && n % 7 == 0 {
            di = 7;
        } else {
            return 0;
        }

        let mut j = 0usize;
        let mut orig_cmd = cmd;
        let mut i = 0usize;
        while i < n {
            // subsequent coordinate pairs for M are really L
            if i > 0 && (orig_cmd == b'M' || orig_cmd == b'm') {
                orig_cmd = b'L'.wrapping_add(orig_cmd.wrapping_sub(b'M'));
            }

            cmd = orig_cmd;
            // coords := p.coords[i : i+di]; coordFloats := p.coordFloats[i : i+di]
            let (mut lo, mut hi) = (i, i + di);

            // set next coordinate
            let ax: f64;
            let ay: f64;
            if cmd == b'H' || cmd == b'h' {
                let mut x = self.coord_floats[lo + di - 1];
                if is_rel_cmd {
                    x += self.x;
                }
                ax = x;
                ay = self.y;
            } else if cmd == b'V' || cmd == b'v' {
                ax = self.x;
                let mut y = self.coord_floats[lo + di - 1];
                if is_rel_cmd {
                    y += self.y;
                }
                ay = y;
            } else {
                let mut x = self.coord_floats[lo + di - 2];
                let mut y = self.coord_floats[lo + di - 1];
                if is_rel_cmd {
                    x += self.x;
                    y += self.y;
                }
                ax = x;
                ay = y;
            }

            // switch from C to S whenever possible
            if cmd == b'C' || cmd == b'c' || cmd == b'S' || cmd == b's' {
                if self.cx.is_nan() {
                    self.cx = self.x;
                    self.cy = self.y;
                } else {
                    self.cx = 2.0 * self.x - self.cx;
                    self.cy = 2.0 * self.y - self.cy;
                }

                let cp1x: f64;
                let cp1y: f64;
                let mut cp2x = self.coord_floats[lo + di - 4];
                let mut cp2y = self.coord_floats[lo + di - 3];
                if is_rel_cmd {
                    cp2x += self.x;
                    cp2y += self.y;
                }
                if cmd == b'C' || cmd == b'c' {
                    let mut x1 = self.coord_floats[lo + di - 6];
                    let mut y1 = self.coord_floats[lo + di - 5];
                    if is_rel_cmd {
                        x1 += self.x;
                        y1 += self.y;
                    }
                    cp1x = x1;
                    cp1y = y1;
                    if cp1x == self.cx && cp1y == self.cy {
                        if is_rel_cmd {
                            cmd = b's';
                        } else {
                            cmd = b'S';
                        }
                        lo += 2;
                    }
                } else {
                    cp1x = self.cx;
                    cp1y = self.cy;
                }

                // if control points overlap begin/end points, this is a straight line
                // even though if the control points would be along the straight line, we won't minify that as the control points influence the speed along the curve (important for dashes for example)
                // only change to a lines if we start with s or S and none follow
                if (cmd == b'C' || cmd == b'c' || i == 0 && i + di >= n)
                    && (cp1x == self.x && cp1y == self.y || cp1x == ax && cp1y == ay)
                    && (cp2x == self.x && cp2y == self.y || cp2x == ax && cp2y == ay)
                {
                    if is_rel_cmd {
                        cmd = b'l';
                    } else {
                        cmd = b'L';
                    }
                    lo = hi - 2;
                    cp2x = f64::NAN;
                    cp2y = f64::NAN;
                }
                self.cx = cp2x;
                self.cy = cp2y;
            } else {
                self.cx = f64::NAN;
                self.cy = f64::NAN;
            }

            // switch from Q to T whenever possible
            if cmd == b'Q' || cmd == b'q' || cmd == b'T' || cmd == b't' {
                if self.qx.is_nan() {
                    self.qx = self.x;
                    self.qy = self.y;
                } else {
                    self.qx = 2.0 * self.x - self.qx;
                    self.qy = 2.0 * self.y - self.qy;
                }

                let mut cpx: f64;
                let mut cpy: f64;
                if cmd == b'Q' || cmd == b'q' {
                    cpx = self.coord_floats[lo + di - 4];
                    cpy = self.coord_floats[lo + di - 3];
                    if is_rel_cmd {
                        cpx += self.x;
                        cpy += self.y;
                    }
                    if cpx == self.qx && cpy == self.qy {
                        if is_rel_cmd {
                            cmd = b't';
                        } else {
                            cmd = b'T';
                        }
                        lo += 2;
                    }
                } else {
                    cpx = self.qx;
                    cpy = self.qy;
                }

                // if control point overlaps begin/end points, this is a straight line
                // even if the control point would be along the straight line, we won't minify that as the control point influences the speed along the curve (important for dashes for example)
                // only change to line if we start with t or T and none follow
                if (cmd == b'Q' || cmd == b'q' || i == 0 && i + di >= n)
                    && (cpx == self.x && cpy == self.y || cpx == ax && cpy == ay)
                {
                    if is_rel_cmd {
                        cmd = b'l';
                    } else {
                        cmd = b'L';
                    }
                    lo = hi - 2;
                    cpx = f64::NAN;
                    cpy = f64::NAN;
                }
                self.qx = cpx;
                self.qy = cpy;
            } else {
                self.qx = f64::NAN;
                self.qy = f64::NAN;
            }

            // switch from L to H or V whenever possible
            if cmd == b'L' || cmd == b'l' {
                if ax == self.x && ay == self.y {
                    i += di;
                    continue;
                } else if ax == self.x {
                    if is_rel_cmd {
                        cmd = b'v';
                    } else {
                        cmd = b'V';
                    }
                    lo += 1;
                } else if ay == self.y {
                    if is_rel_cmd {
                        cmd = b'h';
                    } else {
                        cmd = b'H';
                    }
                    hi = lo + 1;
                }
            }

            // make a current and alternated path with absolute/relative altered
            let cur_state = self.shorten_cur_pos_instruction(cmd, lo, hi);
            let alt_state = if is_rel_cmd {
                self.shorten_alt_pos_instruction(
                    cmd.wrapping_sub(b'a').wrapping_add(b'A'),
                    lo,
                    hi,
                    self.x,
                    self.y,
                )
            } else {
                self.shorten_alt_pos_instruction(
                    cmd.wrapping_sub(b'A').wrapping_add(b'a'),
                    lo,
                    hi,
                    -self.x,
                    -self.y,
                )
            };

            // choose shortest, relative or absolute path?
            if self.alt_buffer.len() < self.cur_buffer.len() {
                j += b.slice_from(j).copy_from_slice(&self.alt_buffer);
                self.state = alt_state;
            } else {
                j += b.slice_from(j).copy_from_slice(&self.cur_buffer);
                self.state = cur_state;
            }

            self.x = ax;
            self.y = ay;
            if i == 0 && (orig_cmd == b'M' || orig_cmd == b'm') {
                self.x0 = self.x;
                self.y0 = self.y;
            }
            i += di;
        }
        j
    }

    // Go: svg/pathdata.go:PathData.shortenCurPosInstruction
    /// Only minifies the coordinates `p.coords[lo:hi]`.
    fn shorten_cur_pos_instruction(&mut self, cmd: u8, lo: usize, hi: usize) -> PathDataState {
        let mut state = self.state;
        self.cur_buffer.clear();
        if cmd != state.cmd
            && !(state.cmd == b'M' && cmd == b'L' || state.cmd == b'm' && cmd == b'l')
        {
            self.cur_buffer.push(cmd);
            state.cmd = cmd;
            state.prev_digit = false;
            state.prev_digit_is_int = false;
        }
        for (i, k) in (lo..hi).enumerate() {
            let coord = self.coords[k].clone();
            // Arc has boolean flags that can only be 0 or 1. copyFlag prevents from adding a dot before a zero (instead of a space). However, when the dot already was there, the command is malformed and could make the path longer than before, introducing bugs.
            if (cmd == b'A' || cmd == b'a') && (i % 7 == 3 || i % 7 == 4) {
                state.copy_flag(&mut self.cur_buffer, coord.at(0) == b'1');
                continue;
            }

            let coord = number(coord, self.precision);
            state.copy_number(&mut self.cur_buffer, &coord);
        }
        state
    }

    // Go: svg/pathdata.go:PathData.shortenAltPosInstruction
    /// Toggles the command between absolute / relative coordinates and
    /// minifies the coordinates `p.coordFloats[lo:hi]`.
    fn shorten_alt_pos_instruction(
        &mut self,
        cmd: u8,
        lo: usize,
        hi: usize,
        x: f64,
        y: f64,
    ) -> PathDataState {
        let mut state = self.state;
        self.alt_buffer.clear();
        if cmd != state.cmd
            && !(state.cmd == b'M' && cmd == b'L' || state.cmd == b'm' && cmd == b'l')
        {
            self.alt_buffer.push(cmd);
            state.cmd = cmd;
            state.prev_digit = false;
            state.prev_digit_is_int = false;
        }
        for (i, k) in (lo..hi).enumerate() {
            let mut f = self.coord_floats[k];
            if cmd == b'L'
                || cmd == b'l'
                || cmd == b'C'
                || cmd == b'c'
                || cmd == b'S'
                || cmd == b's'
                || cmd == b'Q'
                || cmd == b'q'
                || cmd == b'T'
                || cmd == b't'
                || cmd == b'M'
                || cmd == b'm'
            {
                if i % 2 == 0 {
                    f += x;
                } else {
                    f += y;
                }
            } else if cmd == b'H' || cmd == b'h' {
                f += x;
            } else if cmd == b'V' || cmd == b'v' {
                f += y;
            } else if cmd == b'A' || cmd == b'a' {
                if i % 7 == 5 {
                    f += x;
                } else if i % 7 == 6 {
                    f += y;
                } else if i % 7 == 3 || i % 7 == 4 {
                    state.copy_flag(&mut self.alt_buffer, f == 1.0);
                    continue;
                }
            }

            let mut coord_buffer = Vec::new();
            go_strconv::append_float(&mut coord_buffer, f, b'g', -1, 64);
            let coord = number(GoBytes::from_vec(coord_buffer), self.new_precision);
            state.copy_number(&mut self.alt_buffer, &coord);
        }
        state
    }
}

impl PathDataState {
    // Go: svg/pathdata.go:PathDataState.copyNumber
    /// Copies a number to the destination buffer, taking into account space
    /// or dot insertion to guarantee the shortest pathdata. May rewrite
    /// `coord` in place (`…00` → `…e2`).
    fn copy_number(&mut self, buffer: &mut Vec<u8>, coord: &GoBytes) {
        if self.prev_digit
            && (coord.at(0).is_ascii_digit() || coord.at(0) == b'.' && self.prev_digit_is_int)
        {
            if coord.at(0) == b'0' && !self.prev_digit_is_int {
                buffer.extend_from_slice(b".0"); // aggresively add dot so subsequent numbers could drop leading space
                // prevDigit stays true and prevDigitIsInt stays false
                return;
            }
            buffer.push(b' ');
        }
        self.prev_digit = true;
        self.prev_digit_is_int = true;
        let n = coord.len();
        if n > 2 && coord.at(n - 2) == b'0' && coord.at(n - 1) == b'0' {
            coord.set(n - 2, b'e');
            coord.set(n - 1, b'2');
            self.prev_digit_is_int = false;
        } else {
            for c in coord.iter() {
                if c == b'.' || c == b'e' || c == b'E' {
                    self.prev_digit_is_int = false;
                    break;
                }
            }
        }
        coord.write_to(buffer);
        self.prev_flag = false;
    }

    // Go: svg/pathdata.go:PathDataState.copyFlag
    fn copy_flag(&mut self, buffer: &mut Vec<u8>, flag: bool) {
        if !self.prev_flag {
            if flag {
                buffer.extend_from_slice(b" 1");
            } else {
                buffer.extend_from_slice(b" 0");
            }
        } else {
            if flag {
                buffer.push(b'1');
            } else {
                buffer.push(b'0');
            }
        }
        self.prev_flag = true;
        self.prev_digit = false;
        self.prev_digit_is_int = false;
    }
}
