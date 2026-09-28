//! Collation signs of the `en` and `th` languages against
//! `tools/go-oracle/nh-langs/collate` (fixture `collate/collate.json.gz`): every language gets
//! collators built from its own tag (Go `langs.NewLanguage`), never `en` in place of `th`.

mod support;

use nh_langs::config::LanguageConfig;
use nh_langs::language::Language;
use support::j_bytes;

#[test]
fn collation_signs_match_go_for_en_and_th() {
    let fx = support::fixture("collate/collate.json.gz");
    let strings: Vec<Vec<u8>> = fx["strings"]
        .as_array()
        .unwrap()
        .iter()
        .map(j_bytes)
        .collect();
    let n = strings.len();
    let stride = fx["coreStride"].as_u64().unwrap() as usize;
    let random = fx["randomPairs"].as_u64().unwrap() as usize;
    let seed = fx["seed"].as_u64().unwrap();

    // The oracle's pair list: all pairs of every `stride`-th string, then LCG pairs.
    let core: Vec<usize> = (0..n).step_by(stride).collect();
    let mut pairs: Vec<(usize, usize)> = Vec::new();
    for a in 0..core.len() {
        for b in a + 1..core.len() {
            pairs.push((core[a], core[b]));
        }
    }
    let mut x = seed;
    let mut next = || {
        x = x
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((x >> 33) % n as u64) as usize
    };
    for _ in 0..random {
        let i = next();
        let j = next();
        pairs.push((i, j));
    }
    assert_eq!(pairs.len() as u64, fx["pairs"].as_u64().unwrap());

    for c in fx["cases"].as_array().unwrap() {
        let tag = c["tag"].as_str().unwrap();
        let l = Language::new(tag, "en", "", LanguageConfig::default()).unwrap();
        assert_eq!(l.tag, tag);
        assert_eq!(
            l.collator1().tag,
            tag,
            "collators are built from the language's own tag"
        );
        let want = c["signs"].as_str().unwrap().as_bytes();
        let mut coll = l.collator1().lock();
        let mut bad = Vec::new();
        for (k, &(i, j)) in pairs.iter().enumerate() {
            let got =
                b'1'.wrapping_add_signed(coll.compare_strings(&strings[i], &strings[j]) as i8);
            if got != want[k] {
                bad.push(format!(
                    "{tag}: {:?} vs {:?}: got {} want {}",
                    String::from_utf8_lossy(&strings[i]),
                    String::from_utf8_lossy(&strings[j]),
                    got as char,
                    want[k] as char
                ));
            }
        }
        drop(coll);

        // sort.SliceStable with the collator's Less.
        let mut order: Vec<usize> = (0..n).collect();
        let c1 = l.collator1();
        order.sort_by(|&a, &b| c1.compare_strings(&strings[a], &strings[b]).cmp(&0));
        let want_order: Vec<usize> = c["sorted"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_u64().unwrap() as usize)
            .collect();
        if order != want_order {
            bad.push(format!("{tag}: sorted order differs"));
        }
        eprintln!("{tag}: {} pairs, {} mismatches", pairs.len(), bad.len());
        assert!(bad.is_empty(), "{}", bad[..bad.len().min(20)].join("\n"));
    }
}
