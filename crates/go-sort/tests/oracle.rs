//! Differential tests against fixtures produced by tools/go-oracle/go-sort.
//!
//! The input generator (splitmix64 + patterns) mirrors the oracle exactly;
//! each fixture line records Go's comparator call count, a hash of the full
//! less/swap call trace, and a hash of the resulting order.

use std::cell::{Cell, RefCell};
use std::cmp::Ordering;

use go_sort::{slices, sort};

struct SplitMix(u64);

impl SplitMix {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn intn(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

const FNV_PRIME: u64 = 0x100000001b3;
const FNV_BASIS: u64 = 0xcbf29ce484222325;

fn mix(h: &mut u64, v: u64) {
    *h ^= v;
    *h = h.wrapping_mul(FNV_PRIME);
}

/// Mirrors the oracle's pairHash: a deterministic but inconsistent
/// comparator source depending only on the two element identities.
fn pair_hash(a: i64, b: i64) -> u64 {
    let mut h =
        (a as u64).wrapping_mul(0x9e3779b97f4a7c15) ^ (b as u64).wrapping_mul(0xc2b2ae3d27d4eb4f);
    h ^= h >> 29;
    h = h.wrapping_mul(0xbf58476d1ce4e5b9);
    h ^= h >> 32;
    h
}

#[derive(Clone, Copy, Debug)]
struct Elem {
    key: i64,
    id: i64,
}

fn gen_keys(p: usize, n: usize, distinct: usize, r: &mut SplitMix) -> Vec<i64> {
    let mut keys = vec![0i64; n];
    for i in 0..n {
        keys[i] = match p {
            0 => r.intn(distinct) as i64,
            1 => (i * distinct / n) as i64,
            2 => ((n - 1 - i) * distinct / n) as i64,
            3 => (i % distinct) as i64,
            4 => (i.min(n - 1 - i) % distinct) as i64,
            5 => 0,
            6 => (i * distinct / n) as i64,
            7 => {
                if i == 0 || r.intn(8) == 0 {
                    r.intn(distinct) as i64
                } else {
                    keys[i - 1]
                }
            }
            8 => {
                if i < n / 2 {
                    ((n / 2 - i) % distinct) as i64
                } else {
                    ((i - n / 2) % distinct) as i64
                }
            }
            _ => unreachable!(),
        };
    }
    if p == 6 && n > 1 {
        for _ in 0..1 + n / 50 {
            let a = r.intn(n);
            let b = r.intn(n);
            keys.swap(a, b);
        }
    }
    keys
}

struct Traced<'a> {
    d: Vec<Elem>,
    kind: u32,
    r: &'a mut SplitMix,
    trace: u64,
    ncalls: u64,
    nsolid: i64,
    candidate: usize,
    gas: i64,
}

impl sort::Interface for Traced<'_> {
    fn len(&self) -> usize {
        self.d.len()
    }
    fn less(&mut self, i: usize, j: usize) -> bool {
        self.ncalls += 1;
        mix(&mut self.trace, 1);
        mix(&mut self.trace, i as u64);
        mix(&mut self.trace, j as u64);
        match self.kind {
            0 => self.d[i].key < self.d[j].key,
            1 => self.r.next() & 1 == 1,
            3 => self.d[i].key <= self.d[j].key,
            4 => pair_hash(self.d[i].id, self.d[j].id) & 1 == 1,
            2 => {
                // Go sort_test.go adversaryTestingData.Less (McIlroy's antiqsort).
                if self.d[i].key == self.gas && self.d[j].key == self.gas {
                    if i == self.candidate {
                        self.d[i].key = self.nsolid;
                        self.nsolid += 1;
                    } else {
                        self.d[j].key = self.nsolid;
                        self.nsolid += 1;
                    }
                }
                if self.d[i].key == self.gas {
                    self.candidate = i;
                } else if self.d[j].key == self.gas {
                    self.candidate = j;
                }
                self.d[i].key < self.d[j].key
            }
            _ => unreachable!(),
        }
    }
    fn swap(&mut self, i: usize, j: usize) {
        mix(&mut self.trace, 2);
        mix(&mut self.trace, i as u64);
        mix(&mut self.trace, j as u64);
        self.d.swap(i, j);
    }
}

fn float_val(k: i64, id: usize) -> f64 {
    match k % 8 {
        0 => f64::from_bits(0x7ff8000000000000 | id as u64),
        1 => -0.0,
        2 => 0.0,
        3 => f64::INFINITY,
        4 => f64::NEG_INFINITY,
        5 => 1.0,
        6 => -1.0,
        _ => k as f64 / 4.0,
    }
}

fn mm_val(k: i64, id: usize) -> f64 {
    match k % 10 {
        0 => f64::from_bits(0x7ff8000000000000 | id as u64), // quiet NaN
        1 => f64::from_bits(0x7ff0000000000000 | (id as u64 + 1)), // signaling NaN
        2 => -0.0,
        3 => 0.0,
        4 => f64::INFINITY,
        5 => f64::NEG_INFINITY,
        6 => 1.0,
        7 => -1.0,
        _ => k as f64 / 3.0,
    }
}

#[derive(Default, Debug, PartialEq)]
struct Res {
    ncalls: u64,
    trace: u64,
    res: u64,
    ids: Vec<u64>,
}

fn run(variant: u32, kind: u32, keys: &[i64], seed: u64) -> Res {
    let n = keys.len();
    let mut d: Vec<Elem> = (0..n)
        .map(|i| Elem {
            key: keys[i],
            id: i as i64,
        })
        .collect();
    let mut r = SplitMix(seed ^ 0x5eed);
    let mut res = Res::default();
    match variant {
        0 | 1 | 8 | 9 => {
            let mut t = Traced {
                d,
                kind,
                r: &mut r,
                trace: FNV_BASIS,
                ncalls: 0,
                nsolid: 0,
                candidate: 0,
                gas: 0,
            };
            if kind == 2 {
                t.gas = n as i64 - 1;
                for e in t.d.iter_mut() {
                    e.key = t.gas;
                }
            }
            match variant {
                0 => sort::sort(&mut t),
                1 => sort::stable(&mut t),
                8 => sort::sort(&mut sort::Reverse(&mut t)),
                9 => sort::stable(&mut sort::Reverse(&mut t)),
                _ => unreachable!(),
            }
            res.ncalls = t.ncalls;
            res.trace = t.trace;
            d = t.d;
        }
        2 | 3 => {
            let mut trace = FNV_BASIS;
            let mut ncalls = 0;
            let less = |x: &[Elem], i: usize, j: usize| {
                ncalls += 1;
                mix(&mut trace, 1);
                mix(&mut trace, i as u64);
                mix(&mut trace, j as u64);
                match kind {
                    1 => r.next() & 1 == 1,
                    3 => x[i].key <= x[j].key,
                    4 => pair_hash(x[i].id, x[j].id) & 1 == 1,
                    _ => x[i].key < x[j].key,
                }
            };
            if variant == 2 {
                sort::slice(&mut d, less);
            } else {
                sort::slice_stable(&mut d, less);
            }
            res.ncalls = ncalls;
            res.trace = trace;
        }
        4 | 5 => {
            let mut trace = FNV_BASIS;
            let mut ncalls = 0;
            let cmpf = |a: &Elem, b: &Elem| {
                ncalls += 1;
                mix(&mut trace, 3);
                mix(&mut trace, a.id as u64);
                mix(&mut trace, b.id as u64);
                let to_ord = |v: i64| match v {
                    -1 => Ordering::Less,
                    0 => Ordering::Equal,
                    _ => Ordering::Greater,
                };
                match kind {
                    1 => to_ord((r.next() % 3) as i64 - 1),
                    3 => {
                        if a.key <= b.key {
                            Ordering::Less
                        } else {
                            Ordering::Greater
                        }
                    }
                    4 => to_ord((pair_hash(a.id, b.id) % 3) as i64 - 1),
                    _ => a.key.cmp(&b.key),
                }
            };
            if variant == 4 {
                slices::sort_func(&mut d, cmpf);
            } else {
                slices::sort_stable_func(&mut d, cmpf);
            }
            res.ncalls = ncalls;
            res.trace = trace;
        }
        6 | 7 => {
            let mut f: Vec<f64> = (0..n).map(|i| float_val(keys[i], i)).collect();
            if variant == 6 {
                slices::sort(&mut f);
            } else {
                sort::Float64Slice(&mut f).sort();
            }
            res.res = FNV_BASIS;
            for v in &f {
                mix(&mut res.res, v.to_bits());
                res.ids.push(v.to_bits());
            }
            return res;
        }
        12 | 13 => {
            let f: Vec<f64> = (0..n).map(|i| mm_val(keys[i], i)).collect();
            let v = if variant == 12 {
                slices::min(&f)
            } else {
                slices::max(&f)
            };
            res.res = v.to_bits();
            return res;
        }
        10 => {
            let mut x: Vec<i64> = (0..n).map(|i| keys[i] * 7919 - 3 * i as i64).collect();
            slices::sort(&mut x);
            res.res = FNV_BASIS;
            for v in &x {
                mix(&mut res.res, *v as u64);
                res.ids.push(*v as u64);
            }
            return res;
        }
        11 => {
            let mut x: Vec<String> = (0..n)
                .map(|i| (keys[i] * 31 + (i % 3) as i64).to_string())
                .collect();
            slices::sort(&mut x);
            res.res = FNV_BASIS;
            for v in &x {
                for &c in v.as_bytes() {
                    mix(&mut res.res, c as u64);
                }
                mix(&mut res.res, 0x100);
            }
            return res;
        }
        _ => unreachable!(),
    }
    res.res = FNV_BASIS;
    for e in &d {
        mix(&mut res.res, e.id as u64);
        res.ids.push(e.id as u64);
    }
    res
}

fn run_file(path: &str) -> usize {
    let data = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("read {path}: {e}"));
    let mut failures = 0;
    let mut n_cases = 0;
    for (lineno, line) in data.lines().enumerate() {
        let f: Vec<u64> = line.split(' ').map(|x| x.parse().unwrap()).collect();
        let (variant, kind, p, n, distinct, seed) = (
            f[0] as u32,
            f[1] as u32,
            f[2] as usize,
            f[3] as usize,
            f[4] as usize,
            f[5],
        );
        let want = Res {
            ncalls: f[6],
            trace: f[7],
            res: f[8],
            ids: f[9..].to_vec(),
        };
        let keys = gen_keys(p, n, distinct, &mut SplitMix(seed.wrapping_mul(31)));
        let mut got = run(variant, kind, &keys, seed);
        if want.ids.is_empty() {
            got.ids.clear();
        }
        if got != want {
            failures += 1;
            if failures <= 10 {
                eprintln!(
                    "line {}: variant {variant} kind {kind} pattern {p} n {n} distinct {distinct}\n  want {:?}\n  got  {:?}",
                    lineno + 1,
                    want,
                    got
                );
            }
        }
        n_cases += 1;
    }
    assert_eq!(failures, 0, "{failures} mismatches out of {n_cases}");
    n_cases
}

#[test]
fn oracle_fixture() {
    let n = run_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/sort.txt"
    ));
    assert!(n > 10000, "fixture too small: {n}");
}

/// Dense sample (`go-sort -dense -maxn 3000 -idsmax 0 -every 10`): sizes
/// 0..=3000, every variant with key (ties), PRNG, non-strict `<=` and
/// identity-hash comparators plus the McIlroy adversary.
#[test]
fn oracle_dense() {
    let n = run_file(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/dense.txt"
    ));
    assert!(n > 12000, "fixture too small: {n}");
}

/// Large scratch corpus: GO_SORT_CORPUS=<file> cargo test --release -- --ignored
#[test]
#[ignore]
fn oracle_corpus() {
    let path = std::env::var("GO_SORT_CORPUS").expect("set GO_SORT_CORPUS");
    let n = run_file(&path);
    eprintln!("{n} cases OK");
}

// ---- API smoke tests (Go sort_test.go / search_test.go / slices sort_test.go) ----

#[test]
fn go_sort_tests() {
    // Go: sort/sort_test.go:ints/float64s/strs
    let mut ints: Vec<i64> = vec![
        74, 59, 238, -784, 9845, 959, 905, 0, 0, 42, 7586, -5467984, 7586,
    ];
    sort::ints(&mut ints);
    assert!(sort::ints_are_sorted(&ints));
    let mut f = vec![
        74.3,
        59.0,
        f64::INFINITY,
        238.2,
        -784.0,
        2.3,
        f64::NAN,
        f64::NAN,
        f64::NEG_INFINITY,
        9845.768,
        -959.7485,
        905.0,
        7.8,
        7.8,
    ];
    sort::float64s(&mut f);
    assert!(sort::float64s_are_sorted(&f));
    assert!(f[0].is_nan() && f[1].is_nan());
    let mut strs = vec!["", "Hello", "foo", "bar", "foo", "f00", "%*&^*&^&", "***"];
    sort::strings(&mut strs);
    assert_eq!(
        strs,
        vec!["", "%*&^*&^&", "***", "Hello", "bar", "f00", "foo", "foo"]
    );
    let mut v = ints.clone();
    sort::sort(&mut sort::Reverse(sort::IntSlice(&mut v)));
    assert!(v.windows(2).all(|w| w[0] >= w[1]));
}

#[test]
fn go_search_tests() {
    // Go: sort/search_test.go:tests
    let data: Vec<i64> = vec![-10, -5, 0, 1, 2, 3, 5, 7, 11, 100, 100, 100, 1000, 10000];
    let f = |n: usize, g: &dyn Fn(usize) -> bool| sort::search(n, g);
    assert_eq!(f(0, &|_| true), 0);
    assert_eq!(f(1, &|_| false), 1);
    assert_eq!(f(1, &|_| true), 0);
    assert_eq!(f(1e9 as usize, &|i| i >= 991), 991);
    assert_eq!(f(data.len(), &|i| data[i] >= -10), 0);
    assert_eq!(f(data.len(), &|i| data[i] >= 1), 3);
    assert_eq!(f(data.len(), &|i| data[i] >= 100), 9);
    assert_eq!(f(data.len(), &|i| data[i] >= 101), 12);
    assert_eq!(f(data.len(), &|i| data[i] >= 10001), 14);
    assert_eq!(sort::search_ints(&data, 11), 8);
    assert_eq!(sort::search_float64s(&[1.0, 2.0, 2.5], 2.2), 2);
    assert_eq!(sort::search_strings(&["a", "b", "d"], b"c"), 2);
    let (i, found) = sort::find(data.len(), |i| 100i64.cmp(&data[i]) as i32);
    assert_eq!((i, found), (9, true));
    let (i, found) = sort::find(data.len(), |i| 101i64.cmp(&data[i]) as i32);
    assert_eq!((i, found), (12, false));
}

#[test]
fn go_slices_tests() {
    let x = [3.0, f64::NAN, -0.0, 0.0];
    assert!(slices::min(&x).is_nan());
    assert!(slices::max(&x).is_nan());
    let z = [0.0f64, -0.0];
    assert!(slices::min(&z).is_sign_negative());
    assert!(slices::max(&[-0.0f64, 0.0]).is_sign_positive());
    assert!(slices::max(&[-0.0f64, -0.0]).is_sign_negative());
    let s = ["a", "c", "c", "e"];
    assert_eq!(slices::binary_search(&s, &"c"), (1, true));
    assert_eq!(slices::binary_search(&s, &"d"), (3, false));
    let nans = [f64::NAN, 1.0];
    assert_eq!(slices::binary_search(&nans, &f64::NAN), (0, true));
    assert_eq!(
        slices::binary_search_func(&s, "e", |a, b| a.cmp(&b)),
        (3, true)
    );
    assert_eq!(
        slices::min_func(&[(1, 'a'), (0, 'b'), (0, 'c')], |a, b| a.0.cmp(&b.0)),
        (0, 'b')
    );
    assert_eq!(
        slices::max_func(&[(1, 'a'), (0, 'b'), (1, 'c')], |a, b| a.0.cmp(&b.0)),
        (1, 'a')
    );
    assert!(slices::is_sorted(&[1, 2, 2, 3]));
    assert!(!slices::is_sorted_func(
        &[1, 2, 2, 1],
        |a: &i32, b: &i32| a.cmp(b)
    ));
}

#[test]
fn closure_interface_api() {
    // len/less/swap closures sharing data through RefCell.
    let data = RefCell::new(vec![
        5, 1, 4, 1, 5, 9, 2, 6, 5, 3, 5, 8, 9, 7, 9, 3, 2, 3, 8, 4,
    ]);
    let n = data.borrow().len();
    let calls = Cell::new(0);
    sort::sort_less_swap(
        n,
        |i, j| {
            calls.set(calls.get() + 1);
            let d = data.borrow();
            d[i] < d[j]
        },
        |i, j| data.borrow_mut().swap(i, j),
    );
    assert!(data.borrow().windows(2).all(|w| w[0] <= w[1]));
    assert!(calls.get() > 0);

    let mut v: Vec<(i32, i32)> = (0..50).map(|i| (i % 3, i)).collect();
    go_sort::stable_by(&mut v, |a, b| a.0 < b.0);
    assert!(
        v.windows(2)
            .all(|w| w[0].0 < w[1].0 || (w[0].0 == w[1].0 && w[0].1 < w[1].1))
    );
    go_sort::sort_by(&mut v, |a, b| a.1 > b.1);
    assert_eq!(v[0].1, 49);
}
