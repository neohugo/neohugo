//! Port of Go's `regexp/syntax/simplify.go` (go1.27.1).
//!
//! Go returns the receiver (or shares sub-nodes) where nothing changes; the port returns an
//! owned tree with the same structure.

use super::regexp::{Flags, NON_GREEDY, Op, Regexp};

impl Regexp {
    /// Simplify returns a regexp equivalent to re but without counted repetitions and with
    /// various other simplifications, such as rewriting /(?:a+)+/ to /a+/.
    // Go: regexp/syntax/simplify.go:Simplify
    pub fn simplify(&self) -> Regexp {
        let re = self;
        match re.op {
            Op::Capture | Op::Concat | Op::Alternate => re.simplify_children(),
            Op::Star | Op::Plus | Op::Quest => {
                let sub = re.sub[0].simplify();
                simplify1(re.op, re.flags, sub)
            }
            Op::Repeat => re.simplify_repeat(),
            _ => re.clone(),
        }
    }

    /// The OpCapture, OpConcat and OpAlternate case of Simplify: simplify children, building
    /// a new Regexp. (Separate functions keep the recursive frame small: trees can be ~2,000
    /// levels deep.)
    #[inline(never)]
    fn simplify_children(&self) -> Regexp {
        let re = self;
        let mut nre = Regexp::with_op(re.op);
        nre.flags = re.flags;
        nre.min = re.min;
        nre.max = re.max;
        nre.cap = re.cap;
        nre.name = re.name.clone();
        nre.rune = re.rune.clone();
        let mut sub = Vec::with_capacity(re.sub.len());
        for s in &re.sub {
            sub.push(s.simplify());
        }
        nre.sub = sub;
        nre
    }

    /// The OpRepeat case of Simplify.
    #[inline(never)]
    fn simplify_repeat(&self) -> Regexp {
        let re = self;
        // Special special case: x{0} matches the empty string and doesn't even need
        // to consider x.
        if re.min == 0 && re.max == 0 {
            return Regexp::with_op(Op::EmptyMatch);
        }

        // The fun begins.
        let sub = re.sub[0].simplify();

        // x{n,} means at least n matches of x.
        if re.max == -1 {
            // Special case: x{0,} is x*.
            if re.min == 0 {
                return simplify1(Op::Star, re.flags, sub);
            }

            // Special case: x{1,} is x+.
            if re.min == 1 {
                return simplify1(Op::Plus, re.flags, sub);
            }

            // General case: x{4,} is xxxx+.
            let mut nre = Regexp::with_op(Op::Concat);
            for _ in 0..re.min - 1 {
                nre.sub.push(sub.clone());
            }
            nre.sub.push(simplify1(Op::Plus, re.flags, sub));
            return nre;
        }

        // Special case x{0} handled above.

        // Special case: x{1} is just x.
        if re.min == 1 && re.max == 1 {
            return sub;
        }

        // General case: x{n,m} means n copies of x and m copies of x?
        // The machine will do less work if we nest the final m copies,
        // so that x{2,5} = xx(x(x(x)?)?)?

        // Build leading prefix: xx.
        let mut prefix: Option<Regexp> = None;
        if re.min > 0 {
            let mut p = Regexp::with_op(Op::Concat);
            for _ in 0..re.min {
                p.sub.push(sub.clone());
            }
            prefix = Some(p);
        }

        // Build and attach suffix: (x(x(x)?)?)?
        if re.max > re.min {
            let mut suffix = simplify1(Op::Quest, re.flags, sub.clone());
            for _ in re.min + 1..re.max {
                let mut nre2 = Regexp::with_op(Op::Concat);
                nre2.sub = vec![sub.clone(), suffix];
                suffix = simplify1(Op::Quest, re.flags, nre2);
            }
            match prefix.as_mut() {
                None => return suffix,
                Some(p) => p.sub.push(suffix),
            }
        }
        if let Some(p) = prefix {
            return p;
        }

        // Some degenerate case like min > max or min < max < 0. Handle as impossible
        // match.
        Regexp::with_op(Op::NoMatch)
    }
}

/// simplify1 implements Simplify for the unary OpStar, OpPlus, and OpQuest operators. It
/// returns the simple regexp equivalent to Regexp{Op: op, Flags: flags, Sub: {sub}}.
// Go: regexp/syntax/simplify.go:simplify1
fn simplify1(op: Op, flags: Flags, sub: Regexp) -> Regexp {
    // Special case: repeat the empty string as much as you want, but it's going to match only
    // once.
    if sub.op == Op::EmptyMatch {
        return sub;
    }
    // The operators are idempotent if the flags match.
    if op == sub.op && flags & NON_GREEDY == sub.flags & NON_GREEDY {
        return sub;
    }
    // (Go returns `re` itself when it is `Regexp{Op: op, Flags: flags, Sub: {sub}}` already.)
    let mut re = Regexp::with_op(op);
    re.flags = flags;
    re.sub = vec![sub];
    re
}
