//! Port of `github.com/gobwas/glob@v0.2.3` `compiler/compiler.go`.
//!
//! Owner: Wave B task T02 (common-paths-text).

use go_unicode::utf8;

use super::ast::{Kind, Node, Value};
use super::lexer::Rune;
use super::matcher::{
    Matcher, new_any, new_any_of, new_btree, new_contains, new_every_of, new_list, new_max,
    new_min, new_nothing, new_prefix, new_prefix_any, new_prefix_suffix, new_range, new_row,
    new_single, new_suffix, new_suffix_any, new_super, new_text,
};

// Go: github.com/gobwas/glob compiler/compiler.go:optimizeMatcher
fn optimize_matcher(matcher: Matcher) -> Matcher {
    match matcher {
        Matcher::Any { ref separators } => {
            if separators.is_empty() {
                return new_super();
            }
            matcher
        }

        Matcher::AnyOf { mut matchers } => {
            if matchers.len() == 1 {
                return matchers.pop().expect("one matcher");
            }

            Matcher::AnyOf { matchers }
        }

        Matcher::List { ref list, not } => {
            if !not && list.len() == 1 {
                return new_text(&utf8::from_runes(list));
            }

            matcher
        }

        Matcher::BTree(mut t) => {
            t.left = t.left.take().map(optimize_matcher);
            t.right = t.right.take().map(optimize_matcher);

            let r = match &t.value {
                Matcher::Text { str, .. } => str.clone(),
                _ => return Matcher::BTree(t),
            };

            let left_nil = t.left.is_none();
            let right_nil = t.right.is_none();
            if left_nil && right_nil {
                return new_text(&r);
            }

            let left_super = matches!(t.left, Some(Matcher::Super));
            let lp = match &t.left {
                Some(Matcher::Prefix { prefix }) => Some(prefix.clone()),
                _ => None,
            };
            let la = match &t.left {
                Some(Matcher::Any { separators }) => Some(separators.clone()),
                _ => None,
            };

            let right_super = matches!(t.right, Some(Matcher::Super));
            let rs = match &t.right {
                Some(Matcher::Suffix { suffix }) => Some(suffix.clone()),
                _ => None,
            };
            let ra = match &t.right {
                Some(Matcher::Any { separators }) => Some(separators.clone()),
                _ => None,
            };

            if left_super && right_super {
                return new_contains(&r, false);
            }
            if left_super && right_nil {
                return new_suffix(&r);
            }
            if right_super && left_nil {
                return new_prefix(&r);
            }
            if left_nil && let Some(rs) = rs {
                return new_prefix_suffix(&r, &rs);
            }
            if right_nil && let Some(lp) = lp {
                return new_prefix_suffix(&lp, &r);
            }
            if right_nil && let Some(la) = la {
                return new_suffix_any(&r, &la);
            }
            if left_nil && let Some(ra) = ra {
                return new_prefix_any(&r, &ra);
            }

            Matcher::BTree(t)
        }

        _ => matcher,
    }
}

// Go: github.com/gobwas/glob compiler/compiler.go:compileMatchers
fn compile_matchers(matchers: &[Matcher]) -> Result<Matcher, String> {
    if matchers.is_empty() {
        return Err("compile error: need at least one matcher".to_string());
    }
    if matchers.len() == 1 {
        return Ok(matchers[0].clone());
    }
    if let Some(m) = glue_matchers(matchers) {
        return Ok(m);
    }

    let mut idx: isize = -1;
    let mut max_len: isize = -1;
    let mut val: Option<&Matcher> = None;
    for (i, matcher) in matchers.iter().enumerate() {
        let l = matcher.len();
        if l != -1 && l >= max_len {
            max_len = l;
            idx = i as isize;
            val = Some(matcher);
        }
    }

    let Some(val) = val else {
        // not found matcher with static length
        let r = compile_matchers(&matchers[1..])?;
        return Ok(new_btree(matchers[0].clone(), None, Some(r)));
    };

    let idx = idx as usize;
    let left = &matchers[..idx];
    let right: &[Matcher] = if matchers.len() > idx + 1 {
        &matchers[idx + 1..]
    } else {
        &[]
    };

    let mut l = None;
    let mut r = None;
    if !left.is_empty() {
        l = Some(compile_matchers(left)?);
    }

    if !right.is_empty() {
        r = Some(compile_matchers(right)?);
    }

    Ok(new_btree(val.clone(), l, r))
}

// Go: github.com/gobwas/glob compiler/compiler.go:glueMatchers
fn glue_matchers(matchers: &[Matcher]) -> Option<Matcher> {
    if let Some(m) = glue_matchers_as_every(matchers) {
        return Some(m);
    }
    if let Some(m) = glue_matchers_as_row(matchers) {
        return Some(m);
    }
    None
}

// Go: github.com/gobwas/glob compiler/compiler.go:glueMatchersAsRow
fn glue_matchers_as_row(matchers: &[Matcher]) -> Option<Matcher> {
    if matchers.len() <= 1 {
        return None;
    }

    let mut c = Vec::new();
    let mut l: isize = 0;
    for matcher in matchers {
        let ml = matcher.len();
        if ml == -1 {
            return None;
        }
        c.push(matcher.clone());
        l += ml;
    }
    Some(new_row(l, c))
}

// Go: github.com/gobwas/glob compiler/compiler.go:glueMatchersAsEvery
fn glue_matchers_as_every(matchers: &[Matcher]) -> Option<Matcher> {
    if matchers.len() <= 1 {
        return None;
    }

    let mut has_any = false;
    let mut has_super = false;
    let mut has_single = false;
    let mut min: isize = 0;
    let mut separator: Vec<Rune> = Vec::new();

    for (i, matcher) in matchers.iter().enumerate() {
        let sep: Vec<Rune> = match matcher {
            Matcher::Super => {
                has_super = true;
                Vec::new()
            }

            Matcher::Any { separators } => {
                has_any = true;
                separators.clone()
            }

            Matcher::Single { separators } => {
                has_single = true;
                min += 1;
                separators.clone()
            }

            Matcher::List { list, not } => {
                if !not {
                    return None;
                }
                has_single = true;
                min += 1;
                list.clone()
            }

            _ => return None,
        };

        // initialize
        if i == 0 {
            separator = sep.clone();
        }

        if sep == separator {
            continue;
        }

        return None;
    }

    if has_super && !has_any && !has_single {
        return Some(new_super());
    }

    if has_any && !has_super && !has_single {
        return Some(new_any(&separator));
    }

    if (has_any || has_super) && min > 0 && separator.is_empty() {
        return Some(new_min(min));
    }

    let mut every = Vec::new();

    if min > 0 {
        every.push(new_min(min));

        if !has_any && !has_super {
            every.push(new_max(min));
        }
    }

    if !separator.is_empty() {
        every.push(new_contains(&utf8::from_runes(&separator), true));
    }

    Some(new_every_of(every))
}

// Go: github.com/gobwas/glob compiler/compiler.go:minimizeMatchers
fn minimize_matchers(matchers: Vec<Matcher>) -> Vec<Matcher> {
    let mut done: Option<Matcher> = None;
    let (mut left, mut right, mut count) = (0usize, 0usize, 0usize);

    for l in 0..matchers.len() {
        let mut r = matchers.len();
        while r > l {
            if let Some(glued) = glue_matchers(&matchers[l..r]) {
                let swap = match &done {
                    None => true,
                    Some(d) => {
                        let (cl, gl) = (d.len(), glued.len());
                        let swap = cl > -1 && gl > -1 && gl > cl;
                        swap || count < r - l
                    }
                };

                if swap {
                    done = Some(glued);
                    left = l;
                    right = r;
                    count = r - l;
                }
            }
            r -= 1;
        }
    }

    let Some(done) = done else {
        return matchers;
    };

    let mut next: Vec<Matcher> = matchers[..left].to_vec();
    next.push(done);
    if right < matchers.len() {
        next.extend_from_slice(&matchers[right..]);
    }

    if next.len() == matchers.len() {
        return next;
    }

    minimize_matchers(next)
}

// Go: github.com/gobwas/glob compiler/compiler.go:minimizeTree
/// Tries to apply some heuristics to minimize number of nodes in given tree.
fn minimize_tree(tree: &Node) -> Option<Node> {
    match tree.kind {
        Kind::AnyOf => minimize_tree_any_of(tree),
        _ => None,
    }
}

// Go: github.com/gobwas/glob compiler/compiler.go:minimizeTreeAnyOf
/// Tries to find common children of given node of AnyOf pattern; it searches for common
/// children from left and from right. If any common children are found it returns a new
/// optimized ast tree, else None.
fn minimize_tree_any_of(tree: &Node) -> Option<Node> {
    if !are_of_same_kind(&tree.children, Kind::Pattern) {
        return None;
    }

    let (common_left, common_right) = common_children(&tree.children);
    let (common_left_count, common_right_count) = (common_left.len(), common_right.len());
    if common_left_count == 0 && common_right_count == 0 {
        // there are no common parts
        return None;
    }

    let mut result: Vec<Node> = Vec::new();
    if common_left_count > 0 {
        result.push(Node::new(Kind::Pattern, Value::Nil, common_left));
    }

    let mut any_of: Vec<Node> = Vec::new();
    for child in &tree.children {
        let reuse = &child.children[common_left_count..child.children.len() - common_right_count];
        let node = if reuse.is_empty() {
            // this pattern is completely reduced by commonLeft and commonRight patterns
            // so it become nothing
            Node::new(Kind::Nothing, Value::Nil, Vec::new())
        } else {
            Node::new(Kind::Pattern, Value::Nil, reuse.to_vec())
        };
        append_if_unique(&mut any_of, node);
    }
    if any_of.len() == 1 && any_of[0].kind != Kind::Nothing {
        result.push(any_of.pop().expect("one node"));
    } else if any_of.len() > 1 {
        result.push(Node::new(Kind::AnyOf, Value::Nil, any_of));
    }

    if common_right_count > 0 {
        result.push(Node::new(Kind::Pattern, Value::Nil, common_right));
    }

    Some(Node::new(Kind::Pattern, Value::Nil, result))
}

// Go: github.com/gobwas/glob compiler/compiler.go:commonChildren
fn common_children(nodes: &[Node]) -> (Vec<Node>, Vec<Node>) {
    let mut common_left: Vec<Node> = Vec::new();
    if nodes.len() <= 1 {
        return (common_left, Vec::new());
    }

    // find node that has least number of children
    let idx = least_children(nodes);
    if idx == -1 {
        return (common_left, Vec::new());
    }
    let idx = idx as usize;
    let tree = &nodes[idx];
    let tree_length = tree.children.len();

    // allocate max able size for rightCommon slice
    // to get ability insert elements in reverse order (from end to start)
    // without sorting
    let mut common_right: Vec<Option<Node>> = vec![None; tree_length];
    let mut last_right = tree_length; // will use this to get results as commonRight[lastRight:]

    let mut break_left = false;
    let mut break_right = false;
    let mut common_total = 0usize;
    let (mut i, mut j) = (0isize, tree_length as isize - 1);
    while common_total < tree_length && j >= 0 && !(break_left && break_right) {
        let tree_left = &tree.children[i as usize];
        let tree_right = &tree.children[j as usize];

        let mut k = 0;
        while k < nodes.len() && !(break_left && break_right) {
            // skip least children node
            if k == idx {
                k += 1;
                continue;
            }

            let rest_left = &nodes[k].children[i as usize];
            let rest_right = &nodes[k].children
                [(j + nodes[k].children.len() as isize - tree_length as isize) as usize];

            break_left = break_left || tree_left != rest_left;

            // disable searching for right common parts, if left part is already overlapping
            break_right = break_right || (!break_left && j <= i);
            break_right = break_right || tree_right != rest_right;
            k += 1;
        }

        if !break_left {
            common_total += 1;
            common_left.push(tree_left.clone());
        }
        if !break_right {
            common_total += 1;
            last_right = j as usize;
            common_right[j as usize] = Some(tree_right.clone());
        }
        i += 1;
        j -= 1;
    }

    let common_right: Vec<Node> = common_right
        .drain(last_right..)
        .map(|n| n.expect("contiguous common right nodes"))
        .collect();

    (common_left, common_right)
}

// Go: github.com/gobwas/glob compiler/compiler.go:appendIfUnique
fn append_if_unique(target: &mut Vec<Node>, val: Node) {
    for n in target.iter() {
        if *n == val {
            return;
        }
    }
    target.push(val);
}

// Go: github.com/gobwas/glob compiler/compiler.go:areOfSameKind
fn are_of_same_kind(nodes: &[Node], kind: Kind) -> bool {
    nodes.iter().all(|n| n.kind == kind)
}

// Go: github.com/gobwas/glob compiler/compiler.go:leastChildren
fn least_children(nodes: &[Node]) -> isize {
    let mut min: isize = -1;
    let mut idx: isize = -1;
    for (i, n) in nodes.iter().enumerate() {
        if idx == -1 || (n.children.len() as isize) < min {
            min = n.children.len() as isize;
            idx = i as isize;
        }
    }
    idx
}

// Go: github.com/gobwas/glob compiler/compiler.go:compileTreeChildren
fn compile_tree_children(tree: &Node, sep: &[Rune]) -> Result<Vec<Matcher>, String> {
    let mut matchers = Vec::new();
    for desc in &tree.children {
        let m = compile(desc, sep)?;
        matchers.push(optimize_matcher(m));
    }
    Ok(matchers)
}

// Go: github.com/gobwas/glob compiler/compiler.go:compile
fn compile(tree: &Node, sep: &[Rune]) -> Result<Matcher, String> {
    let m = match tree.kind {
        Kind::AnyOf => {
            // todo this could be faster on pattern_alternatives_combine_lite (see glob_test.go)
            if let Some(n) = minimize_tree(tree) {
                return compile(&n, sep);
            }
            let matchers = compile_tree_children(tree, sep)?;
            return Ok(new_any_of(matchers));
        }

        Kind::Pattern => {
            if tree.children.is_empty() {
                return Ok(new_nothing());
            }
            let matchers = compile_tree_children(tree, sep)?;
            compile_matchers(&minimize_matchers(matchers))?
        }

        Kind::Any => new_any(sep),

        Kind::Super => new_super(),

        Kind::Single => new_single(sep),

        Kind::Nothing => new_nothing(),

        Kind::List => match &tree.value {
            Value::List { not, chars } => new_list(utf8::to_runes(chars), *not),
            _ => return Err("could not compile tree: unknown node type".to_string()),
        },

        Kind::Range => match &tree.value {
            Value::Range { not, lo, hi } => new_range(*lo, *hi, *not),
            _ => return Err("could not compile tree: unknown node type".to_string()),
        },

        Kind::Text => match &tree.value {
            Value::Text(t) => new_text(t),
            _ => return Err("could not compile tree: unknown node type".to_string()),
        },
    };

    Ok(optimize_matcher(m))
}

/// Go: `compiler.Compile(tree, sep)`.
// Go: github.com/gobwas/glob compiler/compiler.go:Compile
pub fn compile_tree(tree: &Node, sep: &[Rune]) -> Result<Matcher, String> {
    compile(tree, sep)
}
