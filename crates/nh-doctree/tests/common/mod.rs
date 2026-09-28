//! Shared by the oracle tests: fixture loading and the test shifter of
//! `tools/go-oracle/nh-doctree/dtcommon` (Hugo's `contentNodeShifter` over test values).

#![allow(dead_code)]

use std::io::Read;
use std::path::PathBuf;
use std::sync::Arc;

use nh_doctree::dimensions::{DIMENSION_LANGUAGE, Dimension, DimensionFlag};
use nh_doctree::nodeshifttree::Shifter;
use serde_json::Value;

/// Reads `tests/fixtures/<rel>` (gzip-compressed JSON).
pub fn fixture(rel: &str) -> Value {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(rel);
    let f = std::fs::File::open(&p).unwrap_or_else(|e| panic!("{}: {e}", p.display()));
    let mut s = String::new();
    flate2::read::GzDecoder::new(f)
        .read_to_string(&mut s)
        .expect("gzip");
    serde_json::from_str(&s).expect("json")
}

pub fn s(v: &Value) -> &str {
    v.as_str().unwrap_or_else(|| panic!("not a string: {v}"))
}

pub fn u(v: &Value) -> usize {
    v.as_u64().unwrap_or_else(|| panic!("not a uint: {v}")) as usize
}

pub fn i(v: &Value) -> i64 {
    v.as_i64().unwrap_or_else(|| panic!("not an int: {v}"))
}

pub fn b(v: &Value) -> bool {
    v.as_bool().unwrap_or_else(|| panic!("not a bool: {v}"))
}

pub fn arr(v: &Value) -> &Vec<Value> {
    v.as_array().unwrap_or_else(|| panic!("not an array: {v}"))
}

pub fn strs(v: &Value) -> Vec<String> {
    arr(v).iter().map(|x| s(x).to_string()).collect()
}

pub const NUM_LANGUAGES: usize = 2;

/// Go `*TV`: one language version of a page (`*pageState`) or resource (`*resourceSource`).
#[derive(Debug)]
pub struct Tv {
    pub id: String,
    pub lang: usize,
    pub res: bool,
    pub is_page: bool,
    pub branch: bool,
}

/// A tree value: Go `*TV`, `Pages` (contentNodeIs) or `Ress` (resourceSources).
#[derive(Clone, Debug)]
pub enum Tn {
    One(Arc<Tv>),
    Pages(Vec<Option<Arc<Tv>>>),
    Ress(Vec<Option<Arc<Tv>>>),
}

impl Tn {
    /// From the fixture spelling `[id, lang, flags]`.
    pub fn from_def(v: &Value) -> Tn {
        let a = arr(v);
        let flags = u(&a[2]);
        Tn::One(Arc::new(Tv {
            id: s(&a[0]).to_string(),
            lang: u(&a[1]),
            res: flags & 1 != 0,
            is_page: flags & 2 != 0,
            branch: flags & 4 != 0,
        }))
    }

    fn one(&self) -> &Arc<Tv> {
        match self {
            Tn::One(v) => v,
            _ => panic!("unknown type"),
        }
    }
}

fn reprs(vs: &[Option<Arc<Tv>>]) -> String {
    vs.iter()
        .map(|v| v.as_ref().map_or("-".to_string(), |v| v.id.clone()))
        .collect::<Vec<_>>()
        .join(",")
}

/// Go `dtcommon.Repr`.
pub fn repr(n: Option<&Tn>) -> String {
    match n {
        None => "-".to_string(),
        Some(Tn::One(v)) => v.id.clone(),
        Some(Tn::Pages(vs)) => format!("P[{}]", reprs(vs)),
        Some(Tn::Ress(vs)) => format!("R[{}]", reprs(vs)),
    }
}

pub fn is_branch(n: &Tn) -> bool {
    matches!(n, Tn::One(v) if v.branch)
}

/// Go `dtcommon.Shifter` (Hugo's `contentNodeShifter`).
pub struct TestShifter;

fn delete_slot(v: &mut [Option<Arc<Tv>>], lidx: usize) -> (Option<Tn>, bool, bool) {
    let deleted = v[lidx].take();
    let was_deleted = deleted.is_some();
    let is_empty = v.iter().all(|x| x.is_none());
    (deleted.map(Tn::One), was_deleted, is_empty)
}

impl Shifter<Tn> for TestShifter {
    fn for_each_in_dimension(&self, n: &Tn, d: usize, f: &mut dyn FnMut(&Tn) -> bool) {
        assert_eq!(d, DIMENSION_LANGUAGE, "only language dimension supported");
        match n {
            Tn::Pages(vv) => {
                for v in vv.iter().flatten() {
                    if f(&Tn::One(v.clone())) {
                        return;
                    }
                }
            }
            _ => {
                f(n);
            }
        }
    }

    fn insert(&self, old: Tn, new: Tn) -> (Tn, Option<Tn>, bool) {
        let newp = new.one().clone();
        match old {
            Tn::One(vv) => {
                assert_eq!(newp.res, vv.res, "unknown type");
                if vv.lang == newp.lang {
                    return (new, Some(Tn::One(vv)), true);
                }
                let mut is = vec![None; NUM_LANGUAGES];
                let lang = newp.lang;
                is[lang] = Some(newp);
                is[vv.lang] = Some(vv.clone());
                let is = if vv.res { Tn::Ress(is) } else { Tn::Pages(is) };
                (is, Some(Tn::One(vv)), false)
            }
            Tn::Pages(mut vv) => {
                let oldp = vv[newp.lang].replace(newp.clone());
                let updated = oldp.is_some();
                (Tn::Pages(vv), oldp.map(Tn::One), updated)
            }
            Tn::Ress(mut vv) => {
                let oldp = vv[newp.lang].replace(newp.clone());
                let updated = oldp.is_some();
                (Tn::Ress(vv), oldp.map(Tn::One), updated)
            }
        }
    }

    fn insert_into(&self, old: Tn, new: Tn, dimension: Dimension) -> (Tn, Option<Tn>, bool) {
        let langi = dimension[DIMENSION_LANGUAGE];
        let newp = new.one().clone();
        match old {
            Tn::One(vv) => {
                assert_eq!(newp.res, vv.res, "unknown type");
                if vv.lang == newp.lang && newp.lang == langi {
                    return (new, Some(Tn::One(vv)), true);
                }
                let mut is = vec![None; NUM_LANGUAGES];
                is[vv.lang] = Some(vv.clone());
                is[langi] = Some(newp);
                let is = if vv.res { Tn::Ress(is) } else { Tn::Pages(is) };
                (is, Some(Tn::One(vv)), false)
            }
            Tn::Pages(mut vv) => {
                let oldv = vv[langi].replace(newp);
                let updated = oldv.is_some();
                (Tn::Pages(vv), oldv.map(Tn::One), updated)
            }
            Tn::Ress(mut vv) => {
                let oldv = vv[langi].replace(newp);
                let updated = oldv.is_some();
                (Tn::Ress(vv), oldv.map(Tn::One), updated)
            }
        }
    }

    fn delete(&self, _v: Tn, _dimension: Dimension) -> (Option<Tn>, bool, bool) {
        unreachable!("the trees call delete_in_place")
    }

    fn delete_in_place(&self, v: &mut Tn, dimension: Dimension) -> (Option<Tn>, bool, bool) {
        let lidx = dimension[0];
        match v {
            Tn::Pages(vv) | Tn::Ress(vv) => delete_slot(vv, lidx),
            Tn::One(vv) => {
                if lidx != vv.lang {
                    return (None, false, false);
                }
                (Some(Tn::One(vv.clone())), true, true)
            }
        }
    }

    fn shift(
        &self,
        n: &Tn,
        dimension: Dimension,
        exact: bool,
    ) -> (Option<Tn>, bool, DimensionFlag) {
        let lidx = dimension[0];
        let accuracy = DimensionFlag::LANGUAGE;
        match n {
            Tn::Pages(v) => {
                assert!(!v.is_empty(), "empty contentNodeIs");
                if let Some(vv) = &v[lidx] {
                    return (Some(Tn::One(vv.clone())), true, accuracy);
                }
                (None, false, DimensionFlag(0))
            }
            Tn::Ress(v) => {
                if let Some(vv) = &v[lidx] {
                    return (Some(Tn::One(vv.clone())), true, DimensionFlag::LANGUAGE);
                }
                if exact {
                    return (None, false, DimensionFlag(0));
                }
                // For non content resources, pick the first match.
                if let Some(vv) = v.iter().flatten().next() {
                    if vv.is_page {
                        return (None, false, DimensionFlag(0));
                    }
                    return (Some(Tn::One(vv.clone())), true, DimensionFlag(0));
                }
                (None, false, DimensionFlag(0))
            }
            Tn::One(v) => {
                if v.res {
                    if v.lang == lidx {
                        return (Some(n.clone()), true, DimensionFlag::LANGUAGE);
                    }
                    if !v.is_page && !exact {
                        return (Some(n.clone()), true, DimensionFlag(0));
                    }
                } else if v.lang == lidx {
                    return (Some(n.clone()), true, DimensionFlag::LANGUAGE);
                }
                (None, false, DimensionFlag(0))
            }
        }
    }
}
