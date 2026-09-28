//! Port of `github.com/rivo/uniseg@v0.2.0` `grapheme.go` (grapheme cluster boundaries, the
//! version go-runewidth v0.0.16 is built with in neohugo's go.mod).

use super::uniseg_tables::CODE_POINTS;

// The unicode properties (Go: prAny, prPreprend, ...).
pub(crate) const PR_ANY: u8 = 0;
pub(crate) const PR_PREPREND: u8 = 1;
pub(crate) const PR_CR: u8 = 2;
pub(crate) const PR_LF: u8 = 3;
pub(crate) const PR_CONTROL: u8 = 4;
pub(crate) const PR_EXTEND: u8 = 5;
pub(crate) const PR_REGIONAL_INDICATOR: u8 = 6;
pub(crate) const PR_SPACING_MARK: u8 = 7;
pub(crate) const PR_L: u8 = 8;
pub(crate) const PR_V: u8 = 9;
pub(crate) const PR_T: u8 = 10;
pub(crate) const PR_LV: u8 = 11;
pub(crate) const PR_LVT: u8 = 12;
pub(crate) const PR_ZWJ: u8 = 13;
pub(crate) const PR_EXTENDED_PICTOGRAPHIC: u8 = 14;

// The states of the grapheme cluster parser.
const GR_ANY: u8 = 0;
const GR_CR: u8 = 1;
const GR_CONTROL_LF: u8 = 2;
const GR_L: u8 = 3;
const GR_LVV: u8 = 4;
const GR_LVTT: u8 = 5;
const GR_PREPEND: u8 = 6;
const GR_EXTENDED_PICTOGRAPHIC: u8 = 7;
const GR_EXTENDED_PICTOGRAPHIC_ZWJ: u8 = 8;
const GR_RI_ODD: u8 = 9;
const GR_RI_EVEN: u8 = 10;

/// Go: `grTransitions` — (state, property) -> (new state, boundary, rule number).
// Go: grapheme.go:grTransitions
fn gr_transition(state: u8, prop: u8) -> Option<(u8, bool, i32)> {
    Some(match (state, prop) {
        // GB5
        (GR_ANY, PR_CR) => (GR_CR, true, 50),
        (GR_ANY, PR_LF) => (GR_CONTROL_LF, true, 50),
        (GR_ANY, PR_CONTROL) => (GR_CONTROL_LF, true, 50),
        // GB4
        (GR_CR, PR_ANY) => (GR_ANY, true, 40),
        (GR_CONTROL_LF, PR_ANY) => (GR_ANY, true, 40),
        // GB3
        (GR_CR, PR_LF) => (GR_ANY, false, 30),
        // GB6
        (GR_ANY, PR_L) => (GR_L, true, 9990),
        (GR_L, PR_L) => (GR_L, false, 60),
        (GR_L, PR_V) => (GR_LVV, false, 60),
        (GR_L, PR_LV) => (GR_LVV, false, 60),
        (GR_L, PR_LVT) => (GR_LVTT, false, 60),
        // GB7
        (GR_ANY, PR_LV) => (GR_LVV, true, 9990),
        (GR_ANY, PR_V) => (GR_LVV, true, 9990),
        (GR_LVV, PR_V) => (GR_LVV, false, 70),
        (GR_LVV, PR_T) => (GR_LVTT, false, 70),
        // GB8
        (GR_ANY, PR_LVT) => (GR_LVTT, true, 9990),
        (GR_ANY, PR_T) => (GR_LVTT, true, 9990),
        (GR_LVTT, PR_T) => (GR_LVTT, false, 80),
        // GB9
        (GR_ANY, PR_EXTEND) => (GR_ANY, false, 90),
        (GR_ANY, PR_ZWJ) => (GR_ANY, false, 90),
        // GB9a
        (GR_ANY, PR_SPACING_MARK) => (GR_ANY, false, 91),
        // GB9b
        (GR_ANY, PR_PREPREND) => (GR_PREPEND, true, 9990),
        (GR_PREPEND, PR_ANY) => (GR_ANY, false, 92),
        // GB11
        (GR_ANY, PR_EXTENDED_PICTOGRAPHIC) => (GR_EXTENDED_PICTOGRAPHIC, true, 9990),
        (GR_EXTENDED_PICTOGRAPHIC, PR_EXTEND) => (GR_EXTENDED_PICTOGRAPHIC, false, 110),
        (GR_EXTENDED_PICTOGRAPHIC, PR_ZWJ) => (GR_EXTENDED_PICTOGRAPHIC_ZWJ, false, 110),
        (GR_EXTENDED_PICTOGRAPHIC_ZWJ, PR_EXTENDED_PICTOGRAPHIC) => {
            (GR_EXTENDED_PICTOGRAPHIC, false, 110)
        }
        // GB12 / GB13
        (GR_ANY, PR_REGIONAL_INDICATOR) => (GR_RI_ODD, true, 9990),
        (GR_RI_ODD, PR_REGIONAL_INDICATOR) => (GR_RI_EVEN, false, 120),
        (GR_RI_EVEN, PR_REGIONAL_INDICATOR) => (GR_RI_ODD, true, 120),
        _ => return None,
    })
}

/// Go: `property(r)` — binary search in `codePoints`.
// Go: properties.go:property
fn property(r: i32) -> u8 {
    let mut from = 0usize;
    let mut to = CODE_POINTS.len();
    while to > from {
        let middle = (from + to) / 2;
        let cp = CODE_POINTS[middle];
        if r < cp.0 {
            to = middle;
            continue;
        }
        if r > cp.1 {
            from = middle + 1;
            continue;
        }
        return cp.2;
    }
    PR_ANY
}

/// Go: `uniseg.Graphemes` — iterates over the grapheme clusters of a string.
pub(crate) struct Graphemes {
    code_points: Vec<i32>,
    indices: Vec<usize>,
    start: usize,
    end: usize,
    pos: usize,
    state: u8,
}

impl Graphemes {
    /// Go: `NewGraphemes(s)` (`for pos, r := range s`: invalid UTF-8 bytes are U+FFFD).
    // Go: grapheme.go:NewGraphemes
    pub(crate) fn new(s: &str) -> Graphemes {
        let mut code_points = Vec::new();
        let mut indices = Vec::new();
        for (pos, r) in go_unicode::utf8::runes(s.as_bytes()) {
            code_points.push(r);
            indices.push(pos);
        }
        indices.push(s.len());
        let mut g = Graphemes {
            code_points,
            indices,
            start: 0,
            end: 0,
            pos: 0,
            state: GR_ANY,
        };
        g.next(); // Parse ahead.
        g
    }

    /// Go: `Next()` — advances to the next grapheme cluster.
    // Go: grapheme.go:Next
    pub(crate) fn next(&mut self) -> bool {
        self.start = self.end;

        // The state transition gives us a boundary instruction BEFORE the next code point so we
        // always need to stay ahead by one code point.

        // Parse the next code point.
        while self.pos <= self.code_points.len() {
            // GB2.
            if self.pos == self.code_points.len() {
                self.end = self.pos;
                self.pos += 1;
                break;
            }

            // Determine the property of the next character.
            let next_property = property(self.code_points[self.pos]);
            self.pos += 1;

            // Find the applicable transition.
            let boundary;
            if let Some(t) = gr_transition(self.state, next_property) {
                // We have a specific transition. We'll use it.
                self.state = t.0;
                boundary = t.1;
            } else {
                // No specific transition found. Try the less specific ones.
                let trans_any_prop = gr_transition(self.state, PR_ANY);
                let trans_any_state = gr_transition(GR_ANY, next_property);
                match (trans_any_prop, trans_any_state) {
                    (Some(p), Some(s)) => {
                        // Both apply. We'll use a mix (see comments for grTransitions).
                        self.state = s.0;
                        let mut b = s.1;
                        if p.2 < s.2 {
                            self.state = p.0;
                            b = p.1;
                        }
                        boundary = b;
                    }
                    (Some(p), None) => {
                        // We only have a specific state.
                        self.state = p.0;
                        boundary = p.1;
                        // This branch will probably never be reached because okAnyState will
                        // always be true given the current transition map. But we keep it here
                        // for future modifications to the transition map where this may not be
                        // true anymore.
                    }
                    (None, Some(s)) => {
                        // We only have a specific property.
                        self.state = s.0;
                        boundary = s.1;
                    }
                    (None, None) => {
                        // No known transition. GB999: Any x Any.
                        self.state = GR_ANY;
                        boundary = true;
                    }
                }
            }

            // If we found a cluster boundary, let's stop here. The current cluster will be the
            // one that just ended.
            if self.pos - 1 == 0 /* GB1 */ || boundary {
                self.end = self.pos - 1;
                break;
            }
        }

        self.start != self.end
    }

    /// Go: `Runes()`.
    // Go: grapheme.go:Runes
    pub(crate) fn runes(&self) -> &[i32] {
        &self.code_points[self.start..self.end]
    }

    /// Go: `Positions()` — the byte interval of the current cluster.
    // Go: grapheme.go:Positions
    pub(crate) fn positions(&self) -> (usize, usize) {
        (self.indices[self.start], self.indices[self.end])
    }
}
