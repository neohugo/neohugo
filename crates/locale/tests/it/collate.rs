//! Collation sanity on the 2,802 strings the reference sites sort (`corpus/collate/site-strings.hex`:
//! titles, taxonomy terms, ingredients, in English and Thai).

use std::cmp::Ordering;

use neohugo_base::Collate;
use neohugo_locale::{Collator, Locale};

fn site_strings() -> Vec<String> {
    let path = neohugo_testkit::fixture::testdata("corpus/collate/site-strings.hex");
    let text = std::fs::read_to_string(&path).unwrap();
    text.lines()
        .filter(|l| !l.is_empty())
        .filter_map(|hex| {
            let bytes: Vec<u8> = (0..hex.len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            String::from_utf8(bytes).ok()
        })
        .collect()
}

fn sorted(c: &dyn Collate, strings: &[String]) -> Vec<String> {
    let mut v = strings.to_vec();
    v.sort_by(|a, b| c.compare(a, b));
    v
}

#[test]
fn collation_is_a_consistent_total_order() {
    let strings = site_strings();
    assert!(strings.len() > 2700, "{} strings", strings.len());
    let c = Collator::for_language("en");
    let order = sorted(&c, &strings);
    let mut equal_pairs = 0;
    for w in order.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let ab = c.compare(a, b);
        assert_ne!(ab, Ordering::Greater, "{a:?} > {b:?} after sorting");
        assert_eq!(
            c.compare(b, a),
            ab.reverse(),
            "{a:?} vs {b:?} not antisymmetric"
        );
        if ab == Ordering::Equal && a != b {
            equal_pairs += 1;
        }
    }
    // spot-check transitivity over strides of the sorted order
    for stride in [7, 101, 997] {
        for i in (0..order.len().saturating_sub(stride)).step_by(stride) {
            assert_ne!(c.compare(&order[i], &order[i + stride]), Ordering::Greater);
        }
    }
    let bytewise = {
        let mut v = strings.clone();
        v.sort();
        v
    };
    let moved = order.iter().zip(&bytewise).filter(|(a, b)| a != b).count();
    println!(
        "collate: {} strings, {moved} positions differ from byte order, {equal_pairs} distinct \
         strings compare equal",
        strings.len()
    );
    assert!(moved > 0, "collation looks like byte order");
    assert_eq!(equal_pairs, 0, "distinct site strings compare equal");
}

/// Thai sorts in the root order (as Hugo did with x/text): the same order as English, Latin
/// before Thai, Thai leading vowels skipped.
#[test]
fn thai_uses_the_root_order() {
    let strings = site_strings();
    let en = sorted(&Collator::for_language("en"), &strings);
    let th = sorted(&Locale::new("th"), &strings);
    let und = sorted(&Collator::for_language("und"), &strings);
    assert_eq!(en, und);
    assert_eq!(th, und);

    let c = Collator::for_language("th");
    let tags = sorted(&c, &["ปาร์ตี้", "เนย", "คาราเมล", "คริสปี้พาย"].map(String::from));
    assert_eq!(tags, ["คริสปี้พาย", "คาราเมล", "เนย", "ปาร์ตี้"]);
    let ingredients = sorted(
        &c,
        &["น้ำตาล", "เกลือไอโอดีน", "INS 941", "คาราเมล", "INS 160a (i)"].map(String::from),
    );
    assert_eq!(
        ingredients,
        ["INS 160a (i)", "INS 941", "เกลือไอโอดีน", "คาราเมล", "น้ำตาล"]
    );
}

/// Thai sorts PAIYANNOI (ฯ) with the punctuation, before digits and Latin letters, as Hugo's Thai
/// collation does; other languages keep the root order, where it is a Thai letter.
#[test]
fn thai_sorts_paiyannoi_as_punctuation() {
    let th = Collator::for_language("th");
    let und = Collator::for_language("und");
    assert_eq!(th.compare("ฯ", "A"), Ordering::Less);
    assert_eq!(th.compare("ฯ", "1"), Ordering::Less);
    assert_eq!(und.compare("ฯ", "A"), Ordering::Greater);
    // Inside a word it is punctuation too: before any letter at that position.
    assert_eq!(th.compare("กรุงเทพฯ", "กรุงเทพก"), Ordering::Less);
    assert_eq!(th.compare("กรุงเทพฯ", "กรุงเทพ"), Ordering::Greater);
    // Distinct strings stay distinct and the order stays antisymmetric.
    assert_eq!(th.compare("aฯ", "a!"), Ordering::Greater);
    assert_eq!(th.compare("a!", "aฯ"), Ordering::Less);
    assert_eq!(th.compare("ฯ", "ฯ"), Ordering::Equal);
    let list = sorted(
        &th,
        &["กรุงเทพฯ ข", "Bangkok", "กรุงเทพ", "ฯลฯ", "123"].map(String::from),
    );
    assert_eq!(list, ["ฯลฯ", "123", "Bangkok", "กรุงเทพ", "กรุงเทพฯ ข"]);
    // `Locale` uses the same collator.
    assert_eq!(Locale::new("th").compare("ฯ", "A"), Ordering::Less);
}

#[test]
fn case_and_accents_are_secondary_to_letters() {
    let c = Collator::for_language("en");
    assert_eq!(c.compare("apple", "Banana"), Ordering::Less);
    assert_eq!(c.compare("a", "A"), Ordering::Less);
    assert_eq!(c.compare("résumé", "resume"), Ordering::Greater);
    assert_eq!(c.compare("résumé", "resumes"), Ordering::Less);
    assert_eq!(c.compare("item 10", "item 9"), Ordering::Less); // no numeric collation
    // a language's tailoring applies (Swedish sorts ä after z)
    assert_eq!(
        Collator::for_language("sv").compare("ä", "z"),
        Ordering::Greater
    );
    assert_eq!(c.compare("ä", "z"), Ordering::Less);
}
