//! Port of golang.org/x/text@v0.26.0/internal/language (language.go,
//! lookup.go, parse.go, compose.go, match.go, compact.go, tags.go) and
//! internal/tag, restricted to what `language.Parse`, `Canonicalize`,
//! `Compose`, `Tag.Base`, `Tag.Parent` and `colltab.MatchLang` use.
//!
//! Go's scanner keeps `token` as a slice aliasing the scanner's backing
//! array; the port models the array (`Scanner::b` + `Scanner::tail`, with
//! Go's capacities) so stale-token reads after the buffer was shifted under
//! it match Go byte for byte, see `Scanner` and PORTING.md (deviation 7).

use std::fmt;

use super::tables::{LangTables, Triple, tables};

// ---------------------------------------------------------------------------
// internal/tag

// Go: internal/tag/tag.go:Index.Elem
fn idx_elem(s: &[u8], x: usize) -> &[u8] {
    &s[x * 4..x * 4 + 4]
}

// Go: internal/tag/tag.go:cmp
fn tag_cmp(a: &[u8], b: &[u8]) -> i32 {
    let n = a.len().min(b.len());
    for (i, &c) in b[..n].iter().enumerate() {
        if a[i] > c {
            return 1;
        } else if a[i] < c {
            return -1;
        }
    }
    match a.len().cmp(&b.len()) {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Greater => 1,
        std::cmp::Ordering::Equal => 0,
    }
}

// Go: internal/tag/tag.go:Index.Index
fn idx_index(s: &[u8], key: &[u8]) -> isize {
    let n = key.len();
    // sort.Search(len(s)/4, cmp(s[i*4:i*4+n], key) != -1)
    let (mut lo, mut hi) = (0usize, s.len() / 4);
    while lo < hi {
        let h = lo + (hi - lo) / 2;
        if tag_cmp(&s[h * 4..h * 4 + n], key) == -1 {
            lo = h + 1;
        } else {
            hi = h;
        }
    }
    let i = lo * 4;
    if i + key.len() > s.len() || tag_cmp(&s[i..i + key.len()], key) != 0 {
        return -1;
    }
    lo as isize
}

// Go: internal/tag/tag.go:Index.Next
fn idx_next(s: &[u8], key: &[u8], x: isize) -> isize {
    let x = x + 1;
    let xu = x as usize;
    if xu * 4 < s.len() && tag_cmp(&s[xu * 4..xu * 4 + key.len()], key) == 0 {
        return x;
    }
    -1
}

// Go: internal/tag/tag.go:FixCase
fn fix_case(form: &[u8], b: &mut [u8]) -> bool {
    if form.len() != b.len() {
        return false;
    }
    for (i, c) in b.iter_mut().enumerate() {
        let mut ch = *c;
        if form[i] <= b'Z' {
            if ch >= b'a' {
                ch = ch.wrapping_sub(b'z' - b'Z');
            }
            if !(b'A'..=b'Z').contains(&ch) {
                return false;
            }
        } else {
            if ch <= b'Z' {
                ch = ch.wrapping_add(b'z' - b'Z');
            }
            if !(b'a'..=b'z').contains(&ch) {
                return false;
            }
        }
        *c = ch;
    }
    true
}

// Go: internal/tag/tag.go:Compare
fn tag_compare(a: &[u8], b: &[u8]) -> i32 {
    tag_cmp(a, b)
}

// ---------------------------------------------------------------------------
// errors

/// Errors of the language package.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LangError {
    /// `ErrSyntax`: "language: tag is not well-formed".
    Syntax,
    /// `ErrDuplicateKey`.
    DuplicateKey,
    /// `ValueError`: a well-formed but unknown subtag (first 8 bytes).
    Value([u8; 8]),
    /// `ErrMissingLikelyTagsData`.
    MissingLikelyTagsData,
    /// `errPrivateUse`.
    PrivateUse,
    /// `errInvalidArguments`.
    InvalidArguments,
    /// `errInvalidArgument` of the public Compose.
    InvalidArgument,
    /// A Go runtime panic inside a parsing function (recovered by Go's
    /// `Parse`/`Compose` as `ErrSyntax`).
    #[doc(hidden)]
    GoPanic,
}

impl LangError {
    // Go: internal/language/parse.go:NewValueError
    pub(crate) fn new_value(tag: &[u8]) -> LangError {
        let mut v = [0u8; 8];
        let n = tag.len().min(8);
        v[..n].copy_from_slice(&tag[..n]);
        LangError::Value(v)
    }

    /// For a ValueError, the subtag for which the error occurred.
    pub fn subtag(&self) -> Option<String> {
        if let LangError::Value(v) = self {
            let n = v.iter().position(|&c| c == 0).unwrap_or(8);
            return Some(String::from_utf8_lossy(&v[..n]).into_owned());
        }
        None
    }
}

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LangError::Syntax => f.write_str("language: tag is not well-formed"),
            LangError::DuplicateKey => {
                f.write_str("language: different values for same key in -u extension")
            }
            LangError::Value(_) => {
                // fmt %q of the subtag bytes (ASCII in practice).
                let s = self.subtag().unwrap_or_default();
                write!(f, "language: subtag {s:?} is well-formed but unknown")
            }
            LangError::MissingLikelyTagsData => f.write_str("missing likely tags data"),
            LangError::PrivateUse => f.write_str("cannot set a key on a private use tag"),
            LangError::InvalidArguments => f.write_str("invalid key or type"),
            LangError::InvalidArgument => f.write_str("invalid Extension or Variant"),
            LangError::GoPanic => f.write_str("language: tag is not well-formed"),
        }
    }
}

impl std::error::Error for LangError {}

// ---------------------------------------------------------------------------
// ids

/// `language.Language` (internal base language id).
pub type Language = u16;
/// `language.Script` (internal script id).
pub type Script = u16;
/// `language.Region` (internal region id).
pub type Region = u16;

fn t() -> &'static LangTables {
    tables()
}

pub(crate) fn c(name: &str) -> u16 {
    t().c(name) as u16
}

// Go: internal/language/lookup.go:getLangID
fn get_lang_id(s: &mut [u8]) -> (Language, Option<LangError>) {
    if s.len() == 2 {
        return get_lang_iso2(s);
    }
    get_lang_iso3(s)
}

// Go: internal/language/lookup.go:normLang (Language.Canonicalize)
/// Returns the mapped langID and alias type (0 Deprecated, 1 Macro,
/// 2 Legacy, -1 unknown).
pub(crate) fn norm_lang(id: Language) -> (Language, i8) {
    let m = &t().alias_map;
    let k = m.partition_point(|x| x.0 < id);
    if k < m.len() && m[k].0 == id {
        return (m[k].1, t().alias_types[k] as i8);
    }
    (id, -1)
}

// Go: internal/language/lookup.go:getLangISO2
fn get_lang_iso2(s: &mut [u8]) -> (Language, Option<LangError>) {
    if !fix_case(b"zz", s) {
        return (0, Some(LangError::Syntax));
    }
    let lang = t().lang;
    let i = idx_index(lang, s);
    if i != -1 && idx_elem(lang, i as usize)[3] != 0 {
        return (i as Language, None);
    }
    (0, Some(LangError::new_value(s)))
}

const BASE: u32 = (b'z' - b'a' + 1) as u32;

// Go: internal/language/lookup.go:strToInt
fn str_to_int(s: &[u8]) -> u32 {
    let mut v = 0u32;
    for &c in s {
        v = v.wrapping_mul(BASE);
        v = v.wrapping_add(c.wrapping_sub(b'a') as u32);
    }
    v
}

// Go: internal/language/lookup.go:intToStr
fn int_to_str(mut v: u32, s: &mut [u8]) {
    for i in (0..s.len()).rev() {
        s[i] = (v % BASE) as u8 + b'a';
        v /= BASE;
    }
}

// Go: internal/language/lookup.go:getLangISO3
fn get_lang_iso3(s: &mut [u8]) -> (Language, Option<LangError>) {
    let tb = t();
    if fix_case(b"und", s) {
        // first try to match canonical 3-letter entries
        let mut i = idx_index(tb.lang, &s[..2]);
        while i != -1 {
            let e = idx_elem(tb.lang, i as usize);
            if e[3] == 0 && e[2] == s[2] {
                // We treat "und" as special and always translate it to
                // "unspecified".
                let id = i as Language;
                if id as u32 == tb.c("nonCanonicalUnd") {
                    return (0, None);
                }
                return (id, None);
            }
            i = idx_next(tb.lang, &s[..2], i);
        }
        let i = idx_index(tb.alt_lang_iso3, s);
        if i != -1 {
            let k = idx_elem(tb.alt_lang_iso3, i as usize)[3];
            return (tb.alt_lang_index[k as usize], None);
        }
        let n = str_to_int(s);
        if tb.lang_no_index[(n / 8) as usize] & (1 << (n % 8)) != 0 {
            return ((n + tb.c("langNoIndexOffset")) as Language, None);
        }
        // Check for non-canonical uses of ISO3.
        let mut i = idx_index(tb.lang, &s[..1]);
        while i != -1 {
            let e = idx_elem(tb.lang, i as usize);
            if e[2] == s[1] && e[3] == s[2] {
                return (i as Language, None);
            }
            i = idx_next(tb.lang, &s[..1], i);
        }
        return (0, Some(LangError::new_value(s)));
    }
    (0, Some(LangError::Syntax))
}

// Go: internal/language/lookup.go:Language.StringToBuf / String
pub(crate) fn lang_string(b: Language) -> String {
    let tb = t();
    let off = tb.c("langNoIndexOffset") as Language;
    if b == 0 {
        return "und".to_string();
    } else if b >= off {
        let mut buf = [0u8; 3];
        int_to_str((b - off) as u32, &mut buf);
        return String::from_utf8_lossy(&buf).into_owned();
    }
    let l = idx_elem(tb.lang, b as usize);
    if l[3] == 0 {
        return String::from_utf8_lossy(&l[..3]).into_owned();
    }
    String::from_utf8_lossy(&l[..2]).into_owned()
}

// Go: internal/language/lookup.go:Language.SuppressScript
pub(crate) fn suppress_script(b: Language) -> Script {
    if (b as u32) < t().c("langNoIndexOffset") {
        return t().suppress_script[b as usize] as Script;
    }
    0
}

// Go: internal/language/lookup.go:Language.IsPrivateUse
#[allow(dead_code)]
pub(crate) fn lang_is_private_use(b: Language) -> bool {
    let tb = t();
    tb.c("langPrivateStart") <= b as u32 && b as u32 <= tb.c("langPrivateEnd")
}

// Go: internal/language/language.go:ParseRegion
pub(crate) fn parse_region(s: &str) -> Result<Region, LangError> {
    let n = s.len();
    if !(2..=3).contains(&n) {
        return Err(LangError::Syntax);
    }
    let mut buf = s.as_bytes().to_vec();
    match get_region_id(&mut buf) {
        // Go: defer func() { if recover() != nil { r = 0; err = ErrSyntax } }()
        Err(LangError::GoPanic) => Err(LangError::Syntax),
        r => r,
    }
}

// Go: internal/language/lookup.go:getRegionID
fn get_region_id(s: &mut [u8]) -> Result<Region, LangError> {
    if s.len() == 3 {
        if is_alpha(s[0]) {
            return get_region_iso3(s);
        }
        if let Some(i) = parse_uint10(s) {
            return get_region_m49(i as i32);
        }
    }
    get_region_iso2(s)
}

/// strconv.ParseUint(string(s), 10, 10) success value.
fn parse_uint10(s: &[u8]) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    let mut v: u32 = 0;
    for &c in s {
        if !c.is_ascii_digit() {
            return None;
        }
        v = v * 10 + (c - b'0') as u32;
        if v > 1023 {
            return None;
        }
    }
    Some(v)
}

// Go: internal/language/lookup.go:findIndex
fn find_index(idx: &[u8], key: &mut [u8], form: &[u8]) -> Result<usize, LangError> {
    if !fix_case(form, key) {
        return Err(LangError::Syntax);
    }
    let i = idx_index(idx, key);
    if i == -1 {
        return Err(LangError::new_value(key));
    }
    Ok(i as usize)
}

// Go: internal/language/lookup.go:getRegionISO2
fn get_region_iso2(s: &mut [u8]) -> Result<Region, LangError> {
    let i = find_index(t().region_iso, s, b"ZZ")?;
    Ok(i as Region + c("isoRegionOffset"))
}

// Go: internal/language/lookup.go:getRegionISO3
fn get_region_iso3(s: &mut [u8]) -> Result<Region, LangError> {
    let tb = t();
    if fix_case(b"ZZZ", s) {
        let mut i = idx_index(tb.region_iso, &s[..1]);
        while i != -1 {
            let e = idx_elem(tb.region_iso, i as usize);
            if e[2] == s[1] && e[3] == s[2] {
                return Ok(i as Region + c("isoRegionOffset"));
            }
            i = idx_next(tb.region_iso, &s[..1], i);
        }
        let mut i = 0;
        while i < tb.alt_region_iso3.len() {
            if tag_compare(&tb.alt_region_iso3[i..i + 3], s) == 0 {
                return Ok(tb.alt_region_ids[i / 3]);
            }
            i += 3;
        }
        return Err(LangError::new_value(s));
    }
    Err(LangError::Syntax)
}

/// Marker for a Go runtime panic (recovered by Parse as ErrSyntax).
pub(crate) const PANIC: LangError = LangError::GoPanic;

// Go: internal/language/lookup.go:getRegionM49
fn get_region_m49(n: i32) -> Result<Region, LangError> {
    let tb = t();
    if 0 < n && n <= 999 {
        const SEARCH_BITS: i32 = 7;
        const REGION_BITS: u32 = 9;
        const REGION_MASK: u16 = (1 << REGION_BITS) - 1;
        let idx = (n >> SEARCH_BITS) as usize;
        let lo = tb.m49_index[idx] as usize;
        let hi = tb.m49_index[idx + 1] as usize;
        let buf = &tb.from_m49[lo..hi];
        let val = (n as u16).wrapping_shl(REGION_BITS); // we rely on bits shifting out
        let i = buf.partition_point(|&x| x < val);
        // Go indexes fromM49[m49Index[idx]+i], which is out of range (a
        // panic recovered as ErrSyntax by Parse) past the last bucket.
        let Some(&r) = tb.from_m49.get(lo + i) else {
            return Err(PANIC);
        };
        if r & !REGION_MASK == val {
            return Ok(r & REGION_MASK);
        }
    }
    // Go: fmt.Fprint(bytes.NewBuffer([]byte(e.v[:])), n) writes into a copy,
    // so the ValueError is empty.
    Err(LangError::Value([0; 8]))
}

// Go: internal/language/lookup.go:normRegion
fn norm_region(r: Region) -> Region {
    let m = &t().region_old_map;
    let k = m.partition_point(|x| x.0 < r);
    if k < m.len() && m[k].0 == r {
        return m[k].1;
    }
    0
}

// Go: internal/language/language.go:Region.Canonicalize
pub(crate) fn region_canonicalize(r: Region) -> Region {
    let cr = norm_region(r);
    if cr != 0 {
        return cr;
    }
    r
}

// Go: internal/language/lookup.go:Region.typ
fn region_typ(r: Region) -> u8 {
    t().region_types[r as usize]
}

// Go: internal/language/lookup.go:Region.String
pub(crate) fn region_string(r: Region) -> String {
    let off = c("isoRegionOffset");
    if r < off {
        if r == 0 {
            return "ZZ".to_string();
        }
        return format!("{:03}", region_m49(r));
    }
    let e = idx_elem(t().region_iso, (r - off) as usize);
    String::from_utf8_lossy(&e[..2]).into_owned()
}

// Go: internal/language/lookup.go:Region.M49
pub(crate) fn region_m49(r: Region) -> i32 {
    t().m49[r as usize] as i16 as i32
}

// Go: internal/language/lookup.go:Region.IsPrivateUse
pub(crate) fn region_is_private_use(r: Region) -> bool {
    region_typ(r) & 1 != 0
}

// Go: internal/language/language.go:Region.IsCountry
pub(crate) fn region_is_country(r: Region) -> bool {
    if r == 0 || region_is_group(r) || region_is_private_use(r) && r != c("_XK") {
        return false;
    }
    true
}

// Go: internal/language/language.go:Region.IsGroup
pub(crate) fn region_is_group(r: Region) -> bool {
    if r == 0 {
        return false;
    }
    (t().region_inclusion[r as usize] as usize) < t().region_containment.len()
}

// Go: internal/language/language.go:Region.Contains
pub(crate) fn region_contains(r: Region, cr: Region) -> bool {
    if r == cr {
        return true;
    }
    let tb = t();
    let g = tb.region_inclusion[r as usize];
    if g as u32 >= tb.c("nRegionGroups") {
        return false;
    }
    let m = tb.region_containment[g as usize];

    let d = tb.region_inclusion[cr as usize];
    let b = tb.region_inclusion_bits[d as usize];

    // A contained country may belong to multiple disjoint groups. Matching any
    // of these indicates containment. If the contained region is a group, it
    // must strictly be a subset.
    if d as u32 >= tb.c("nRegionGroups") {
        return b & m != 0;
    }
    b & !m == 0
}

// Go: internal/language/lookup.go:getScriptID
fn get_script_id(s: &mut [u8]) -> Result<Script, LangError> {
    let i = find_index(t().script, s, b"Zzzz")?;
    Ok(i as Script)
}

// Go: internal/language/lookup.go:Script.String
pub(crate) fn script_string(s: Script) -> String {
    if s == 0 {
        return "Zzzz".to_string();
    }
    String::from_utf8_lossy(idx_elem(t().script, s as usize)).into_owned()
}

// ---------------------------------------------------------------------------
// Tag

const MAX_CORE_SIZE: usize = 12;
const MAX_SIMPLE_U_EXTENSION_SIZE: usize = 14;

/// The internal `language.Tag`.
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct ITag {
    pub lang_id: Language,
    pub region_id: Region,
    pub script_id: Script,
    /// offset in str, includes preceding '-'
    pub(crate) p_variant: u8,
    /// offset of first extension, includes preceding '-'
    pub(crate) p_ext: u16,
    /// String representation, only used if the tag has variants or
    /// extensions.
    pub(crate) str: String,
}

pub(crate) const UND: ITag = ITag {
    lang_id: 0,
    region_id: 0,
    script_id: 0,
    p_variant: 0,
    p_ext: 0,
    str: String::new(),
};

impl ITag {
    pub(crate) fn core(lang_id: Language, script_id: Script, region_id: Region) -> ITag {
        ITag {
            lang_id,
            region_id,
            script_id,
            ..Default::default()
        }
    }

    // Go: internal/language/language.go:Tag.Raw
    pub(crate) fn raw(&self) -> (Language, Script, Region) {
        (self.lang_id, self.script_id, self.region_id)
    }

    // Go: internal/language/language.go:Tag.equalTags
    pub(crate) fn equal_tags(&self, a: &ITag) -> bool {
        self.lang_id == a.lang_id && self.script_id == a.script_id && self.region_id == a.region_id
    }

    // Go: internal/language/language.go:Tag.IsRoot
    pub(crate) fn is_root(&self) -> bool {
        if (self.p_variant as usize) < self.str.len() {
            return false;
        }
        self.equal_tags(&UND)
    }

    // Go: internal/language/language.go:Tag.IsPrivateUse
    pub(crate) fn is_private_use(&self) -> bool {
        !self.str.is_empty() && self.p_variant == 0
    }

    // Go: internal/language/language.go:Tag.RemakeString
    pub(crate) fn remake_string(&mut self) {
        if self.str.is_empty() {
            return;
        }
        let mut extra = &self.str[self.p_variant as usize..];
        if self.p_variant > 0 {
            extra = &extra[1..];
        }
        if self.equal_tags(&UND) && extra.starts_with("x-") {
            self.str = extra.to_string();
            self.p_variant = 0;
            self.p_ext = 0;
            return;
        }
        let mut b = self.gen_core_bytes();
        if !extra.is_empty() {
            let diff = b.len() as isize - self.p_variant as isize;
            b.push('-');
            b.push_str(extra);
            self.p_variant = (self.p_variant as isize + diff) as u8;
            self.p_ext = (self.p_ext as isize + diff) as u16;
        } else {
            self.p_variant = b.len() as u8;
            self.p_ext = b.len() as u16;
        }
        self.str = b;
    }

    // Go: internal/language/language.go:Tag.genCoreBytes
    pub(crate) fn gen_core_bytes(&self) -> String {
        let mut s = lang_string(self.lang_id);
        if self.script_id != 0 {
            s.push('-');
            s.push_str(&script_string(self.script_id));
        }
        if self.region_id != 0 {
            s.push('-');
            s.push_str(&region_string(self.region_id));
        }
        s
    }

    // Go: internal/language/language.go:Tag.String
    pub(crate) fn string(&self) -> String {
        if !self.str.is_empty() {
            return self.str.clone();
        }
        if self.script_id == 0 && self.region_id == 0 {
            return lang_string(self.lang_id);
        }
        self.gen_core_bytes()
    }

    // Go: internal/language/language.go:Tag.Variants
    pub(crate) fn variants(&self) -> &str {
        if self.p_variant == 0 {
            return "";
        }
        &self.str[self.p_variant as usize..self.p_ext as usize]
    }

    // Go: internal/language/language.go:Tag.HasVariants
    pub(crate) fn has_variants(&self) -> bool {
        (self.p_variant as u16) < self.p_ext
    }

    // Go: internal/language/language.go:Tag.HasExtensions
    pub(crate) fn has_extensions(&self) -> bool {
        (self.p_ext as usize) < self.str.len()
    }

    // Go: internal/language/language.go:Tag.HasString
    pub(crate) fn has_string(&self) -> bool {
        !self.str.is_empty()
    }

    // Go: internal/language/language.go:Tag.Extension
    pub(crate) fn extension(&self, x: u8) -> Option<String> {
        let s = self.str.as_bytes();
        let mut i = self.p_ext as usize;
        while (i as isize) < s.len() as isize - 1 {
            let (ni, ext) = get_extension(s, i);
            i = ni;
            if ext[0] == x {
                return Some(String::from_utf8_lossy(ext).into_owned());
            }
        }
        None
    }

    // Go: internal/language/language.go:Tag.Extensions
    pub(crate) fn extensions(&self) -> Vec<String> {
        let s = self.str.as_bytes();
        let mut e = Vec::new();
        let mut i = self.p_ext as usize;
        while (i as isize) < s.len() as isize - 1 {
            let (ni, ext) = get_extension(s, i);
            i = ni;
            e.push(String::from_utf8_lossy(ext).into_owned());
        }
        e
    }

    // Go: internal/language/language.go:Tag.TypeForKey
    pub(crate) fn type_for_key(&self, key: &str) -> String {
        let (_, start, end, _) = self.find_type_for_key(key);
        if end != start {
            let mut s = &self.str[start..end];
            if let Some(p) = s.find('-') {
                s = &s[..p];
            }
            return s.to_string();
        }
        String::new()
    }

    // Go: internal/language/language.go:Tag.SetTypeForKey
    pub(crate) fn set_type_for_key(&self, key: &str, value: &str) -> (ITag, Option<LangError>) {
        let mut t = self.clone();
        if t.is_private_use() {
            return (t, Some(LangError::PrivateUse));
        }
        if key.len() != 2 {
            return (t, Some(LangError::InvalidArguments));
        }

        // Remove the setting if value is "".
        if value.is_empty() {
            let (mut start, sep, end, _) = t.find_type_for_key(key);
            if start != sep {
                let s = t.str.as_bytes();
                // Remove a possible empty extension.
                if s[start - 2] != b'-' {
                    // has previous elements.
                } else if end == s.len() || end + 2 < s.len() && s[end + 2] == b'-' {
                    start -= 2;
                }
                if start == t.p_variant as usize && end == s.len() {
                    t.str = String::new();
                    t.p_variant = 0;
                    t.p_ext = 0;
                } else {
                    t.str = format!("{}{}", &t.str[..start], &t.str[end..]);
                }
            }
            return (t, None);
        }

        if value.len() < 3 || value.len() > 8 {
            return (t, Some(LangError::InvalidArguments));
        }

        let mut buf = [0u8; MAX_CORE_SIZE + MAX_SIMPLE_U_EXTENSION_SIZE];
        let mut u_start = 0usize; // start of the -u extension.

        // Generate the tag string if needed.
        if t.str.is_empty() {
            let core = t.gen_core_bytes();
            let n = core.len().min(buf.len());
            buf[..n].copy_from_slice(&core.as_bytes()[..n]);
            u_start = n;
            buf[u_start] = b'-';
            u_start += 1;
        }

        // Create new key-type pair and parse it to verify.
        let bl;
        {
            let b = &mut buf[u_start..];
            b[..2].copy_from_slice(b"u-");
            let kl = key.len().min(b.len() - 2);
            b[2..2 + kl].copy_from_slice(&key.as_bytes()[..kl]);
            b[4] = b'-';
            let vl = value.len().min(b.len() - 5);
            b[5..5 + vl].copy_from_slice(&value.as_bytes()[..vl]);
            bl = 5 + vl;
        }
        // makeScanner(b): b aliases buf[uStart:], its capacity runs to the end
        // of buf.
        let mut scan = Scanner::with_tail(
            buf[u_start..u_start + bl].to_vec(),
            buf[u_start + bl..].to_vec(),
        );
        parse_extensions(&mut scan);
        if let Some(e) = scan.err.clone() {
            return (t, Some(e));
        }
        // Go keeps using `b`, which aliases the scanner buffer.
        let b: Vec<u8> = scan.b_alias(bl);

        // Assemble the replacement string.
        if t.str.is_empty() {
            t.p_variant = (u_start - 1) as u8;
            t.p_ext = (u_start - 1) as u16;
            let mut s = buf[..u_start].to_vec();
            s.extend_from_slice(&b);
            t.str = String::from_utf8_lossy(&s).into_owned();
        } else {
            let s = t.str.clone();
            let (start, sep, end, has_ext) = t.find_type_for_key(key);
            if start == sep {
                let mut bb: &[u8] = &b;
                if has_ext {
                    bb = &bb[2..];
                }
                t.str = format!("{}-{}{}", &s[..sep], String::from_utf8_lossy(bb), &s[end..]);
            } else {
                t.str = format!("{}-{}{}", &s[..start + 3], value, &s[end..]);
            }
        }
        (t, None)
    }

    // Go: internal/language/language.go:Tag.findTypeForKey
    /// Returns the start and end position for the type corresponding to key
    /// or the point at which to insert the key-value pair if the type wasn't
    /// found. The hasExt return value reports whether an -u extension was
    /// present.
    pub(crate) fn find_type_for_key(&self, key: &str) -> (usize, usize, usize, bool) {
        let s = self.str.as_bytes();
        let mut p = self.p_ext as usize;
        if key.len() != 2 || p == s.len() || p == 0 {
            return (p, p, p, false);
        }

        // Find the correct extension.
        p += 1;
        while s[p] != b'u' {
            if s[p] > b'u' {
                p -= 1;
                return (p, p, p, false);
            }
            p = next_extension(s, p);
            if p == s.len() {
                return (s.len(), s.len(), s.len(), false);
            }
            p += 1;
        }
        // Proceed to the hyphen following the extension name.
        p += 1;

        // curKey is the key currently being processed.
        let mut cur_key: &[u8] = b"";
        let (mut start, mut sep) = (0usize, 0usize);
        let mut end;
        let key = key.as_bytes();

        // Iterate over keys until we get the end of a section.
        loop {
            end = p;
            p += 1;
            while p < s.len() && s[p] != b'-' {
                p += 1;
            }
            let n = p as isize - end as isize - 1;
            if n <= 2 && cur_key == key {
                if sep < end {
                    sep += 1;
                }
                return (start, sep, end, true);
            }
            match n {
                0 | 1 => {
                    // invalid string / next extension
                    return (end, end, end, true);
                }
                2 => {
                    // next key
                    cur_key = &s[end + 1..p];
                    if cur_key > key {
                        return (end, end, end, true);
                    }
                    start = end;
                    sep = p;
                }
                _ => {}
            }
        }
    }

    // Go: internal/language/language.go:Tag.Parent
    /// Returns the CLDR parent of t.
    pub(crate) fn parent(&self) -> ITag {
        let mut t = self.clone();
        if !t.str.is_empty() {
            // Strip the variants and extensions.
            let (b, s, r) = t.raw();
            t = ITag::core(b, s, r);
            if t.region_id == 0 && t.script_id != 0 && t.lang_id != 0 {
                let (base, _) = add_tags(ITag::core(t.lang_id, 0, 0));
                if base.script_id == t.script_id {
                    return ITag::core(t.lang_id, 0, 0);
                }
            }
            return t;
        }
        if t.lang_id != 0 {
            if t.region_id != 0 {
                let mut max_script = t.script_id;
                if max_script == 0 {
                    let (max, _) = add_tags(t.clone());
                    max_script = max.script_id;
                }

                for p in &tables().parents {
                    if p.lang == t.lang_id && p.max_script == max_script {
                        for &r in &p.from_region {
                            if r == t.region_id {
                                return ITag::core(t.lang_id, p.script, p.to_region);
                            }
                        }
                    }
                }

                // Strip the script if it is the default one.
                let (base, _) = add_tags(ITag::core(t.lang_id, 0, 0));
                if base.script_id != max_script {
                    return ITag::core(t.lang_id, max_script, 0);
                }
                return ITag::core(t.lang_id, 0, 0);
            } else if t.script_id != 0 {
                // The parent for an base-script pair with a non-default script
                // is "und" instead of the base language.
                let (base, _) = add_tags(ITag::core(t.lang_id, 0, 0));
                if base.script_id != t.script_id {
                    return UND;
                }
                return ITag::core(t.lang_id, 0, 0);
            }
        }
        UND
    }

    // Go: internal/language/match.go:Tag.Maximize
    pub(crate) fn maximize(&self) -> (ITag, Option<LangError>) {
        add_tags(self.clone())
    }
}

// Go: internal/language/parse.go:getExtension
fn get_extension(s: &[u8], mut p: usize) -> (usize, &[u8]) {
    if s[p] == b'-' {
        p += 1;
    }
    if s[p] == b'x' {
        return (s.len(), &s[p..]);
    }
    let end = next_extension(s, p);
    (end, &s[p..end])
}

// Go: internal/language/parse.go:nextExtension
fn next_extension(s: &[u8], mut p: usize) -> usize {
    let n = s.len() as isize - 3;
    while (p as isize) < n {
        if s[p] == b'-' {
            if s[p + 2] == b'-' {
                return p;
            }
            p += 3;
        } else {
            p += 1;
        }
    }
    s.len()
}

// ---------------------------------------------------------------------------
// match.go

const IS_LIST: u16 = 1;
const SCRIPT_IN_FROM: u16 = 2;
#[allow(dead_code)]
const REGION_IN_FROM: u16 = 4;

impl ITag {
    fn set_undefined_lang(&mut self, id: Language) {
        if self.lang_id == 0 {
            self.lang_id = id;
        }
    }

    fn set_undefined_script(&mut self, id: Script) {
        if self.script_id == 0 {
            self.script_id = id;
        }
    }

    fn set_undefined_region(&mut self, id: Region) {
        if self.region_id == 0 || region_contains(self.region_id, id) {
            self.region_id = id;
        }
    }
}

// Go: internal/language/match.go:specializeRegion
fn specialize_region(t: &mut ITag) -> bool {
    let tb = tables();
    let i = tb.region_inclusion[t.region_id as usize];
    if (i as u32) < tb.c("nRegionGroups") {
        let x = tb.likely_region_group[i as usize];
        // (lang, region, script)
        if x.a == t.lang_id && x.c == t.script_id {
            t.region_id = x.b;
        }
        return true;
    }
    false
}

// Go: internal/language/match.go:addTags
pub(crate) fn add_tags(mut t: ITag) -> (ITag, Option<LangError>) {
    let tb = tables();
    // We leave private use identifiers alone.
    if t.is_private_use() {
        return (t, None);
    }
    if t.script_id != 0 && t.region_id != 0 {
        if t.lang_id != 0 {
            // already fully specified
            specialize_region(&mut t);
            return (t, None);
        }
        // Search matches for und-script-region.
        let x = tb.likely_region[t.region_id as usize];
        let list: &[Triple] = if x.c & IS_LIST != 0 {
            &tb.likely_region_list[x.a as usize..x.a as usize + x.b as usize]
        } else {
            &tb.likely_region[t.region_id as usize..t.region_id as usize + 1]
        };
        for x in list {
            // Deviating from the spec. See match_test.go for details.
            if x.b == t.script_id {
                t.set_undefined_lang(x.a);
                return (t, None);
            }
        }
    }
    if t.lang_id != 0 {
        // Search matches for lang-script and lang-region, where lang != und.
        if (t.lang_id as u32) < tb.c("langNoIndexOffset") {
            let x = tb.likely_lang[t.lang_id as usize];
            // (region, script, flags)
            if x.c & IS_LIST != 0 {
                let list = &tb.likely_lang_list[x.a as usize..x.a as usize + x.b as usize];
                if t.script_id != 0 {
                    for x in list {
                        if x.b == t.script_id && x.c & SCRIPT_IN_FROM != 0 {
                            t.set_undefined_region(x.a);
                            return (t, None);
                        }
                    }
                } else if t.region_id != 0 {
                    let mut count = 0;
                    let mut good_script = true;
                    let mut tt = t.clone();
                    for x in list {
                        // We visit all entries for which the script was not
                        // defined, including the ones where the region was not
                        // defined. This allows for proper disambiguation within
                        // regions.
                        if x.c & SCRIPT_IN_FROM == 0 && region_contains(t.region_id, x.a) {
                            tt.region_id = x.a;
                            tt.set_undefined_script(x.b);
                            good_script = good_script && tt.script_id == x.b;
                            count += 1;
                        }
                    }
                    if count == 1 {
                        return (tt, None);
                    }
                    // Even if we fail to find a unique Region, we might have
                    // an unambiguous script.
                    if good_script {
                        t.script_id = tt.script_id;
                    }
                }
            }
        }
    } else {
        // Search matches for und-script.
        if t.script_id != 0 {
            let x = tb.likely_script[t.script_id as usize];
            // (lang, region)
            if x.b != 0 {
                t.set_undefined_region(x.b);
                t.set_undefined_lang(x.a);
                return (t, None);
            }
        }
        // Search matches for und-region. If und-script-region exists, it
        // would have been found earlier.
        if t.region_id != 0 {
            let i = tb.region_inclusion[t.region_id as usize];
            if (i as u32) < tb.c("nRegionGroups") {
                let x = tb.likely_region_group[i as usize];
                // (lang, region, script)
                if x.b != 0 {
                    t.set_undefined_lang(x.a);
                    t.set_undefined_script(x.c);
                    t.region_id = x.b;
                }
            } else {
                let mut x = tb.likely_region[t.region_id as usize];
                // (lang, script, flags)
                if x.c & IS_LIST != 0 {
                    x = tb.likely_region_list[x.a as usize];
                }
                if x.b != 0 && x.c != SCRIPT_IN_FROM {
                    t.set_undefined_lang(x.a);
                    t.set_undefined_script(x.b);
                    return (t, None);
                }
            }
        }
    }

    // Search matches for lang.
    if (t.lang_id as u32) < tb.c("langNoIndexOffset") {
        let mut x = tb.likely_lang[t.lang_id as usize];
        if x.c & IS_LIST != 0 {
            x = tb.likely_lang_list[x.a as usize];
        }
        if x.a != 0 {
            t.set_undefined_script(x.b);
            t.set_undefined_region(x.a);
        }
        specialize_region(&mut t);
        if t.lang_id == 0 {
            t.lang_id = c("_en"); // default language
        }
        return (t, None);
    }
    (t, Some(LangError::MissingLikelyTagsData))
}

// ---------------------------------------------------------------------------
// parse.go

// Go: internal/language/parse.go:isAlpha
fn is_alpha(b: u8) -> bool {
    b > b'9'
}

// Go: internal/language/parse.go:isAlphaNum
fn is_alpha_num(s: &[u8]) -> bool {
    s.iter().all(|c| c.is_ascii_alphanumeric())
}

/// `max99thPercentileSize`: the scanner's inline buffer (`scanner.bytes`).
const MAX_99TH_PERCENTILE_SIZE: usize = 32;

/// Go runtime `roundupsize(size, noscan=true)` for go1.27.1 (darwin/arm64):
/// the capacity `[]byte(s)` and `append` growth give a byte slice.
fn go_roundupsize(size: usize) -> usize {
    const CLASSES: [usize; 68] = [
        0, 8, 16, 24, 32, 48, 64, 80, 96, 112, 128, 144, 160, 176, 192, 208, 224, 240, 256, 288,
        320, 352, 384, 416, 448, 480, 512, 576, 640, 704, 768, 896, 1024, 1152, 1280, 1408, 1536,
        1792, 2048, 2304, 2688, 3072, 3200, 3456, 4096, 4864, 5376, 6144, 6528, 6784, 6912, 8192,
        9472, 9728, 10240, 10880, 12288, 13568, 14336, 16384, 18432, 19072, 20480, 21760, 24576,
        27264, 28672, 32768,
    ];
    const MAX_SMALL_SIZE: usize = 32768;
    const MALLOC_HEADER_SIZE: usize = 8;
    const PAGE_SIZE: usize = 8192;
    if size <= MAX_SMALL_SIZE - MALLOC_HEADER_SIZE {
        return *CLASSES.iter().find(|&&c| c >= size).unwrap();
    }
    (size + PAGE_SIZE - 1) & !(PAGE_SIZE - 1)
}

/// Go runtime `nextslicecap` (go1.27.1).
fn go_nextslicecap(new_len: usize, old_cap: usize) -> usize {
    let mut newcap = old_cap;
    let doublecap = newcap + newcap;
    if new_len > doublecap {
        return new_len;
    }
    const THRESHOLD: usize = 256;
    if old_cap < THRESHOLD {
        return doublecap;
    }
    loop {
        newcap += (newcap + 3 * THRESHOLD) >> 2;
        if newcap >= new_len {
            break;
        }
    }
    newcap
}

/// scanner is used to scan BCP 47 tokens, which are separated by _ or -.
///
/// Go semantics of the buffer are modelled exactly: `b` is the Go slice
/// `scan.b` and `tail` the rest of its backing array up to `cap(scan.b)`
/// (Go never clears it when `b` shrinks). Go's `token` is a slice into the
/// backing array; the port keeps its (start, end) and reads it from the
/// array, so a token read after the buffer was shifted under it (variant
/// de-duplication or -u key de-duplication followed by an extension) sees
/// exactly the bytes Go sees. When Go moves `b` to a new array (growth past
/// its capacity) the old array stays visible to the stale token and to the
/// caller's aliasing slice (`SetTypeForKey`).
pub(crate) struct Scanner {
    pub(crate) b: Vec<u8>,
    /// Backing array bytes `b[len(b):cap(b)]`.
    tail: Vec<u8>,
    /// The array a stale token points into after `b` moved to a new array.
    tok_arr: Option<Vec<u8>>,
    /// The original backing array once `b` moved (what the caller's slice
    /// passed to makeScanner keeps seeing).
    orig_arr: Option<Vec<u8>>,
    // token = array[tok_start..tok_end] when has_token
    has_token: bool,
    tok_start: usize,
    tok_end: usize,
    start: usize, // start position of the current token
    end: usize,   // end position of the current token
    next: usize,  // next point for scan
    pub(crate) err: Option<LangError>,
    done: bool,
    /// Set when the Go code would have panicked (index out of range).
    pub(crate) panicked: bool,
}

impl Scanner {
    // Go: internal/language/parse.go:makeScannerString
    /// A scanner over a copy of `s` with Go's capacity: the 32-byte inline
    /// array for short input, else `[]byte(s)` (size-class rounded).
    pub(crate) fn new(s: Vec<u8>) -> Scanner {
        let cap = if s.len() <= MAX_99TH_PERCENTILE_SIZE {
            MAX_99TH_PERCENTILE_SIZE
        } else {
            go_roundupsize(s.len())
        };
        let tail = vec![0u8; cap - s.len()];
        Scanner::with_tail(s, tail)
    }

    // Go: internal/language/parse.go:makeScanner
    /// A scanner using `b` as the input buffer; `tail` is the rest of the
    /// caller's array (`b[len(b):cap(b)]`).
    pub(crate) fn with_tail(b: Vec<u8>, tail: Vec<u8>) -> Scanner {
        let mut s = Scanner {
            b,
            tail,
            tok_arr: None,
            orig_arr: None,
            has_token: false,
            tok_start: 0,
            tok_end: 0,
            start: 0,
            end: 0,
            next: 0,
            err: None,
            done: false,
            panicked: false,
        };
        s.init();
        s
    }

    fn cap(&self) -> usize {
        self.b.len() + self.tail.len()
    }

    /// The whole backing array (`b[:cap(b)]`).
    fn arr(&self) -> Vec<u8> {
        let mut a = Vec::with_capacity(self.cap());
        a.extend_from_slice(&self.b);
        a.extend_from_slice(&self.tail);
        a
    }

    /// Makes `arr` the backing array, `b = arr[:len]`.
    fn set_arr(&mut self, mut arr: Vec<u8>, len: usize) {
        self.tail = arr.split_off(len);
        self.b = arr;
    }

    /// Go moved `b` to the new array `arr` (len `len`); the old array stays
    /// visible through the token and the caller's slice.
    fn realloc(&mut self, arr: Vec<u8>, len: usize) {
        let old = self.arr();
        if self.orig_arr.is_none() {
            self.orig_arr = Some(old.clone());
        }
        if self.has_token && self.tok_arr.is_none() {
            self.tok_arr = Some(old);
        }
        self.set_arr(arr, len);
    }

    /// Go `s.b = s.b[:n]` (may extend into the backing array up to cap).
    fn reslice(&mut self, n: usize) {
        let mut n = n;
        if n > self.cap() {
            // Go: slice bounds out of range (recovered by Parse/Compose).
            self.panicked = true;
            n = self.cap();
        }
        if n <= self.b.len() {
            let mut t = self.b.split_off(n);
            t.extend_from_slice(&self.tail);
            self.tail = t;
        } else {
            let k = n - self.b.len();
            self.b.extend_from_slice(&self.tail[..k]);
            self.tail.drain(..k);
        }
    }

    /// Go `s.b[lo:hi]` (read; `hi` may reach into the array up to cap).
    fn sub(&mut self, lo: usize, hi: usize) -> Vec<u8> {
        if lo > hi || hi > self.cap() {
            self.panicked = true;
            return Vec::new();
        }
        if hi <= self.b.len() {
            return self.b[lo..hi].to_vec();
        }
        self.arr()[lo..hi].to_vec()
    }

    /// Go `copy(s.b[p:], src)` (p > len(s.b) panics).
    fn copy_at(&mut self, p: usize, src: &[u8]) {
        if p > self.b.len() {
            self.panicked = true;
            return;
        }
        let k = src.len().min(self.b.len() - p);
        self.b[p..p + k].copy_from_slice(&src[..k]);
    }

    /// Go `append(s.b, x...)`: in place within cap, else growslice.
    fn append(&mut self, x: &[u8]) {
        let len = self.b.len();
        let new_len = len + x.len();
        if new_len <= self.cap() {
            self.b.extend_from_slice(x);
            self.tail.drain(..x.len());
            return;
        }
        let newcap = go_roundupsize(go_nextslicecap(new_len, self.cap()));
        let mut arr = vec![0u8; newcap];
        arr[..len].copy_from_slice(&self.b);
        arr[len..new_len].copy_from_slice(x);
        self.realloc(arr, new_len);
    }

    /// The bytes Go's caller-held slice `b[:n]` sees after the scanner worked
    /// in place on it (makeScanner does not copy).
    fn b_alias(&self, n: usize) -> Vec<u8> {
        let a = match &self.orig_arr {
            Some(a) => a.clone(),
            None => self.arr(),
        };
        a[..n.min(a.len())].to_vec()
    }

    /// Go `scan.token` (a slice of the backing array it was taken from).
    fn token(&self) -> std::borrow::Cow<'_, [u8]> {
        use std::borrow::Cow;
        if !self.has_token {
            return Cow::Borrowed(&[]);
        }
        let (ts, te) = (self.tok_start, self.tok_end);
        if let Some(a) = &self.tok_arr {
            return Cow::Owned(a[ts..te].to_vec());
        }
        if te <= self.b.len() {
            return Cow::Borrowed(&self.b[ts..te]);
        }
        Cow::Owned(self.arr()[ts..te].to_vec())
    }

    fn token_len(&self) -> usize {
        if !self.has_token {
            return 0;
        }
        self.tok_end - self.tok_start
    }

    // Go: internal/language/parse.go:scanner.init
    fn init(&mut self) {
        for c in self.b.iter_mut() {
            if *c == b'_' {
                *c = b'-';
            }
        }
        self.scan();
    }

    // Go: internal/language/parse.go:scanner.toLower
    fn to_lower(&mut self, start: usize, end: usize) {
        for i in start..end {
            let Some(&c) = self.b.get(i) else {
                // Go: index out of range.
                self.panicked = true;
                return;
            };
            if c.is_ascii_uppercase() {
                self.b[i] += b'a' - b'A';
            }
        }
    }

    // Go: internal/language/parse.go:scanner.setError
    fn set_error(&mut self, e: Option<LangError>) {
        if let Some(e) = e
            && (self.err.is_none()
                || (e == LangError::Syntax && self.err != Some(LangError::Syntax)))
        {
            self.err = Some(e);
        }
    }

    // Go: internal/language/parse.go:scanner.resizeRange
    /// Shrinks or grows the array at position oldStart such that a new
    /// string of size newSize can fit between oldStart and oldEnd. Sets the
    /// scan point to after the resized range.
    fn resize_range(&mut self, old_start: usize, old_end: usize, new_size: usize) {
        self.start = old_start;
        let end = old_start + new_size;
        if end != old_end {
            let len = self.b.len();
            let n = len as isize + (end as isize - old_end as isize);
            if old_end > len || n < 0 || old_start > self.cap() {
                // Go: slice bounds out of range.
                self.panicked = true;
                return;
            }
            let n = n as usize;
            let cnt = len - old_end; // copy(b[end:], s.b[oldEnd:])
            let arr = self.arr();
            if n > self.cap() {
                // b = make([]byte, n); copy(b, s.b[:oldStart])
                let mut nb = vec![0u8; n];
                let k = old_start.min(n);
                nb[..k].copy_from_slice(&arr[..k]);
                nb[end..end + cnt].copy_from_slice(&arr[old_end..old_end + cnt]);
                self.realloc(nb, n);
            } else {
                // b = s.b[:n] (same array)
                let mut arr = arr;
                arr.copy_within(old_end..old_end + cnt, end);
                self.set_arr(arr, n);
            }
            self.next = (end as isize + (self.next as isize - self.end as isize)) as usize;
            self.end = end;
        }
    }

    // Go: internal/language/parse.go:scanner.replace
    fn replace(&mut self, repl: &str) {
        self.resize_range(self.start, self.end, repl.len());
        let s = self.start;
        self.copy_at(s, repl.as_bytes());
    }

    // Go: internal/language/parse.go:scanner.gobble
    /// Removes the current token from the input. Caller must call scan after
    /// calling gobble.
    fn gobble(&mut self, e: Option<LangError>) {
        self.set_error(e);
        if self.start == 0 {
            // s.b = s.b[:+copy(s.b, s.b[s.next:])]
            self.copy_down(0, self.next);
            self.end = 0;
        } else {
            // s.b = s.b[:s.start-1+copy(s.b[s.start-1:], s.b[s.end:])]
            self.copy_down(self.start - 1, self.end);
            self.end = self.start - 1;
        }
        self.next = self.start;
    }

    /// Go `s.b = s.b[:dst+copy(s.b[dst:], s.b[src:])]` (in place).
    fn copy_down(&mut self, dst: usize, src: usize) {
        let len = self.b.len();
        if dst > len || src > len {
            // Go: slice bounds out of range.
            self.panicked = true;
            return;
        }
        let cnt = (len - dst).min(len - src);
        let mut arr = self.arr();
        arr.copy_within(src..src + cnt, dst);
        self.set_arr(arr, dst + cnt);
    }

    // Go: internal/language/parse.go:scanner.deleteRange
    /// Removes the given range from s.b before the current token.
    fn delete_range(&mut self, start: usize, end: usize) {
        // s.b = s.b[:start+copy(s.b[start:], s.b[end:])]
        self.copy_down(start, end);
        let diff = end - start;
        self.next -= diff;
        self.start -= diff;
        self.end -= diff;
    }

    // Go: internal/language/parse.go:scanner.scan
    /// Parses the next token of a BCP 47 string. Tokens that are larger than
    /// 8 characters or include non-alphanumeric characters result in an
    /// error and are gobbled and removed from the output. It returns the end
    /// position of the last token consumed.
    fn scan(&mut self) -> usize {
        let end = self.end;
        self.has_token = false;
        self.tok_arr = None;
        self.start = self.next;
        while self.next < self.b.len() {
            let i;
            match self.b[self.next..].iter().position(|&c| c == b'-') {
                None => {
                    self.end = self.b.len();
                    self.next = self.b.len();
                    i = self.end as isize - self.start as isize;
                }
                Some(k) => {
                    self.end = self.next + k;
                    self.next = self.end + 1;
                    i = k as isize;
                }
            }
            let tok_ok = {
                let token = &self.b[self.start..self.end];
                !(!(1..=8).contains(&i) || !is_alpha_num(token))
            };
            if !tok_ok {
                self.gobble(Some(LangError::Syntax));
                continue;
            }
            self.has_token = true;
            self.tok_start = self.start;
            self.tok_end = self.end;
            return end;
        }
        let n = self.b.len();
        if n > 0 && self.b[n - 1] == b'-' {
            self.set_error(Some(LangError::Syntax));
            self.reslice(n - 1);
        }
        self.done = true;
        end
    }

    // Go: internal/language/parse.go:scanner.acceptMinSize
    /// Parses multiple tokens of the given size or greater. It returns the
    /// end position of the last token consumed.
    fn accept_min_size(&mut self, min: usize) -> usize {
        let mut end = self.end;
        self.scan();
        while self.token_len() >= min {
            end = self.end;
            self.scan();
        }
        end
    }
}

const MAX_ALT_TAGLEN: usize = 11; // len("en-US-POSIX")

// Go: internal/language/parse.go:Parse
/// Parses the given BCP 47 string and returns a valid Tag. If parsing failed
/// it returns an error and any part of the tag that could be parsed.
pub(crate) fn parse(s: &str) -> (ITag, Option<LangError>) {
    // TODO: consider supporting old-style locale key-value pairs.
    if s.is_empty() {
        return (UND, Some(LangError::Syntax));
    }
    if s.len() <= MAX_ALT_TAGLEN {
        let mut b = [0u8; MAX_ALT_TAGLEN];
        for (i, ch) in s.char_indices() {
            // Generating invalid UTF-8 is okay as it won't match.
            let mut c = ch as u32;
            if (b'A' as u32..=b'Z' as u32).contains(&c) {
                c += (b'a' - b'A') as u32;
            } else if c == b'_' as u32 {
                c = b'-' as u32;
            }
            b[i] = c as u8;
        }
        if let Some(t) = grandfathered(&b) {
            return (t, None);
        }
    }
    let mut scan = Scanner::new(s.as_bytes().to_vec());
    let r = parse_scan(&mut scan, s.as_bytes());
    if scan.panicked {
        // Go: defer func() { if recover() != nil { t = Und; err = ErrSyntax } }()
        return (UND, Some(LangError::Syntax));
    }
    r
}

// Go: internal/language/parse.go:parse
pub(crate) fn parse_scan(scan: &mut Scanner, s: &[u8]) -> (ITag, Option<LangError>) {
    let mut t = UND;
    let mut end;
    let n = scan.token_len();
    if n <= 1 {
        let l = scan.b.len();
        scan.to_lower(0, l);
        if n == 0 || scan.token()[0] != b'x' {
            return (t, Some(LangError::Syntax));
        }
        end = parse_extensions(scan);
    } else if n >= 4 {
        return (UND, Some(LangError::Syntax));
    } else {
        // the usual case
        let r = parse_tag(scan, true);
        t = r.0;
        end = r.1;
        if scan.token_len() == 1 {
            t.p_ext = end as u16;
            end = parse_extensions(scan);
        } else if end < scan.b.len() {
            scan.set_error(Some(LangError::Syntax));
            scan.reslice(end);
        }
    }
    if (t.p_variant as usize) < scan.b.len() {
        let mut s = s;
        if end < s.len() {
            s = &s[..end];
        }
        if !s.is_empty() && tag_compare(s, &scan.b) == 0 {
            t.str = String::from_utf8_lossy(s).into_owned();
        } else {
            t.str = String::from_utf8_lossy(&scan.b).into_owned();
        }
    } else {
        t.p_variant = 0;
        t.p_ext = 0;
    }
    (t, scan.err.clone())
}

// Go: internal/language/parse.go:parseTag
/// Parses language, script, region and variants. It returns a Tag and the
/// end position in the input that was parsed. If doNorm is true, then
/// <lang>-<extlang> will be normalized to <extlang>.
fn parse_tag(scan: &mut Scanner, do_norm: bool) -> (ITag, usize) {
    let mut t = ITag::default();
    // TODO: set an error if an unknown lang, script or region is encountered.
    let (ts, te) = (scan.tok_start, scan.tok_end);
    let (lang, e) = get_lang_id(&mut scan.b[ts..te]);
    t.lang_id = lang;
    scan.set_error(e);
    scan.replace(&lang_string(t.lang_id));
    let lang_start = scan.start;
    let mut end = scan.scan();
    while scan.token_len() == 3 && is_alpha(scan.token()[0]) {
        // From http://tools.ietf.org/html/bcp47, <lang>-<extlang> tags are
        // equivalent to a tag of the form <extlang>.
        if do_norm {
            let (ts, te) = (scan.tok_start, scan.tok_end);
            let (lang, e) = get_lang_id(&mut scan.b[ts..te]);
            if lang != 0 {
                t.lang_id = lang;
                let lang_str = lang_string(lang);
                scan.copy_at(lang_start, lang_str.as_bytes());
                match scan.b.get_mut(lang_start + lang_str.len()) {
                    Some(c) => *c = b'-',
                    None => scan.panicked = true, // Go: index out of range
                }
                scan.start = lang_start + lang_str.len() + 1;
            }
            scan.gobble(e);
        }
        end = scan.scan();
    }
    if scan.token_len() == 4 && is_alpha(scan.token()[0]) {
        let (ts, te) = (scan.tok_start, scan.tok_end);
        let r = get_script_id(&mut scan.b[ts..te]);
        match r {
            Ok(s) => t.script_id = s,
            Err(_) => t.script_id = 0,
        }
        if t.script_id == 0 {
            scan.gobble(r.err());
        }
        end = scan.scan();
    }
    let n = scan.token_len();
    if (2..=3).contains(&n) {
        let (ts, te) = (scan.tok_start, scan.tok_end);
        let r = get_region_id(&mut scan.b[ts..te]);
        match &r {
            Ok(v) => t.region_id = *v,
            Err(_) => t.region_id = 0,
        }
        if let Err(LangError::GoPanic) = r {
            scan.panicked = true;
        }
        if t.region_id == 0 {
            scan.gobble(r.err());
        } else {
            scan.replace(&region_string(t.region_id));
        }
        end = scan.scan();
    }
    let l = scan.b.len();
    let st = scan.start;
    scan.to_lower(st.min(l), l);
    t.p_variant = end as u8;
    end = parse_variants(scan, end);
    t.p_ext = end as u16;
    (t, end)
}

struct VariantsSort<'a> {
    i: &'a mut Vec<u8>,
    v: &'a mut Vec<Vec<u8>>,
}

impl go_sort::sort::Interface for VariantsSort<'_> {
    fn len(&self) -> usize {
        self.i.len()
    }
    fn swap(&mut self, i: usize, j: usize) {
        self.i.swap(i, j);
        self.v.swap(i, j);
    }
    fn less(&mut self, i: usize, j: usize) -> bool {
        self.i[i] < self.i[j]
    }
}

struct BytesSort<'a> {
    b: &'a mut Vec<Vec<u8>>,
    n: usize, // first n bytes to compare
}

impl go_sort::sort::Interface for BytesSort<'_> {
    fn len(&self) -> usize {
        self.b.len()
    }
    fn swap(&mut self, i: usize, j: usize) {
        self.b.swap(i, j);
    }
    fn less(&mut self, i: usize, j: usize) -> bool {
        for k in 0..self.n {
            // Go indexes b[i][k] (panics if shorter; callers guarantee length).
            let (a, b) = (self.b[i].get(k), self.b[j].get(k));
            if a == b {
                continue;
            }
            return a < b;
        }
        false
    }
}

// Go: internal/language/parse.go:parseVariants
/// Scans tokens as long as each token is a valid variant string. Duplicate
/// variants are removed.
fn parse_variants(scan: &mut Scanner, mut end: usize) -> usize {
    let start = scan.start;
    let mut var_id: Vec<u8> = Vec::new();
    let mut variant: Vec<Vec<u8>> = Vec::new();
    let mut last: i32 = -1;
    let mut need_sort = false;
    while scan.token_len() >= 4 {
        // TODO: measure the impact of needing this conversion and redesign
        // the data structure if there is an issue.
        let v = tables().variant_index.get(&*scan.token()).copied();
        let Some(v) = v else {
            // unknown variant
            let e = LangError::new_value(&scan.token());
            scan.gobble(Some(e));
            scan.scan();
            continue;
        };
        var_id.push(v);
        variant.push(scan.token().to_vec());
        if !need_sort {
            if last < v as i32 {
                last = v as i32;
            } else {
                need_sort = true;
                // There is no legal combinations of more than 7 variants
                // (and this is by no means a useful sequence).
                const MAX_VARIANTS: usize = 8;
                if var_id.len() > MAX_VARIANTS {
                    break;
                }
            }
        }
        end = scan.end;
        scan.scan();
    }
    if need_sort {
        go_sort::sort::sort(&mut VariantsSort {
            i: &mut var_id,
            v: &mut variant,
        });
        let (mut k, mut l) = (0usize, -1i32);
        for i in 0..var_id.len() {
            let w = var_id[i] as i32;
            if l == w {
                // Remove duplicates.
                continue;
            }
            var_id[k] = var_id[i];
            variant[k] = variant[i].clone();
            k += 1;
            l = w;
        }
        let str = variant[..k].join(&b'-');
        if str.is_empty() {
            end = start - 1;
        } else {
            scan.resize_range(start, end, str.len());
            let s = scan.start;
            scan.copy_at(s, &str);
            end = scan.end;
        }
    }
    end
}

// Go: internal/language/parse.go:parseExtensions
/// Parses and normalizes the extensions in the buffer. It returns the last
/// position of scan.b that is part of any extension. It also trims scan.b to
/// remove excess parts accordingly.
pub(crate) fn parse_extensions(scan: &mut Scanner) -> usize {
    let start = scan.start;
    let mut exts: Vec<Vec<u8>> = Vec::new();
    let mut private: Vec<u8> = Vec::new();
    let mut end = scan.end;
    while scan.token_len() == 1 {
        let ext_start = scan.start;
        let ext = scan.token()[0];
        end = parse_extension(scan);
        let extension = scan.sub(ext_start, end);
        if extension.len() < 3 || (ext != b'x' && extension.len() < 4) {
            scan.set_error(Some(LangError::Syntax));
            end = ext_start;
            continue;
        } else if start == ext_start && (ext == b'x' || scan.start == scan.b.len()) {
            scan.reslice(end);
            return end;
        } else if ext == b'x' {
            private = extension;
            break;
        }
        exts.push(extension);
    }
    go_sort::sort::sort(&mut BytesSort { b: &mut exts, n: 1 });
    if !private.is_empty() {
        exts.push(private);
    }
    scan.reslice(start);
    if !exts.is_empty() {
        let j = exts.join(&b'-');
        scan.append(&j);
    } else if start > 0 {
        // Strip trailing '-'.
        scan.reslice(start - 1);
    }
    end
}

// Go: internal/language/parse.go:parseExtension
/// Parses a single extension and returns the position of the extension end.
fn parse_extension(scan: &mut Scanner) -> usize {
    let (start, mut end) = (scan.start, scan.end);
    match scan.token()[0] {
        b'u' => {
            // https://www.ietf.org/rfc/rfc6067.txt
            let attr_start = end;
            scan.scan();
            let mut last: Vec<u8> = Vec::new();
            while scan.token_len() > 2 {
                if *scan.token() >= *last.as_slice() {
                    // Attributes are unsorted. Start over from scratch.
                    let p = attr_start + 1;
                    scan.next = p;
                    let mut attrs: Vec<Vec<u8>> = Vec::new();
                    scan.scan();
                    while scan.token_len() > 2 {
                        attrs.push(scan.token().to_vec());
                        end = scan.end;
                        scan.scan();
                    }
                    go_sort::sort::sort(&mut BytesSort {
                        b: &mut attrs,
                        n: 3,
                    });
                    let j = attrs.join(&b'-');
                    scan.copy_at(p, &j);
                    break;
                }
                last = scan.token().to_vec();
                end = scan.end;
                scan.scan();
            }
            // Scan key-type sequences. A key is of length 2 and may be
            // followed by 0 or more "type" subtags from 3 to the maximum of 8
            // letters.
            let mut last: Vec<u8> = Vec::new();
            let attr_end = end;
            while scan.token_len() == 2 {
                let key = scan.token().to_vec();
                end = scan.end;
                scan.scan();
                while end < scan.end && scan.token_len() > 2 {
                    end = scan.end;
                    scan.scan();
                }
                // TODO: check key value validity
                if key.as_slice().cmp(last.as_slice()) != std::cmp::Ordering::Greater
                    || scan.err.is_some()
                {
                    // We have an invalid key or the keys are not sorted.
                    // Start scanning keys from scratch and reorder.
                    let p = attr_end + 1;
                    scan.next = p;
                    let mut keys: Vec<Vec<u8>> = Vec::new();
                    scan.scan();
                    while scan.token_len() == 2 {
                        let key_start = scan.start;
                        end = scan.end;
                        scan.scan();
                        while end < scan.end && scan.token_len() > 2 {
                            end = scan.end;
                            scan.scan();
                        }
                        let k = scan.sub(key_start, end);
                        keys.push(k);
                    }
                    go_sort::sort::stable(&mut BytesSort { b: &mut keys, n: 2 });
                    let n = keys.len();
                    if n > 0 {
                        let mut k = 0;
                        for i in 1..n {
                            if keys[k][..2] != keys[i][..2] {
                                k += 1;
                                keys[k] = keys[i].clone();
                            } else if keys[k] != keys[i] {
                                scan.set_error(Some(LangError::DuplicateKey));
                            }
                        }
                        keys.truncate(k + 1);
                    }
                    let reordered = keys.join(&b'-');
                    let e = p + reordered.len();
                    if e < end {
                        scan.delete_range(e, end);
                        end = e;
                    }
                    scan.copy_at(p, &reordered);
                    break;
                }
                last = key;
            }
        }
        b't' => {
            // https://www.ietf.org/rfc/rfc6497.txt
            scan.scan();
            let n = scan.token_len();
            if (2..=3).contains(&n) && is_alpha(scan.token()[1]) {
                let r = parse_tag(scan, false);
                end = r.1;
                scan.to_lower(start, end);
            }
            while scan.token_len() == 2 && !is_alpha(scan.token()[1]) {
                end = scan.accept_min_size(3);
            }
        }
        b'x' => {
            end = scan.accept_min_size(1);
        }
        _ => {
            end = scan.accept_min_size(2);
        }
    }
    end
}

// Go: internal/language/lookup.go:grandfathered
fn grandfathered(s: &[u8; MAX_ALT_TAGLEN]) -> Option<ITag> {
    // grandfatheredMap: legacy and grandfathered tags to their base language
    // or index to more elaborate tag.
    let key: &[u8] = {
        let n = s.iter().position(|&c| c == 0).unwrap_or(MAX_ALT_TAGLEN);
        // The Go map key is the full zero-padded array: any byte after the
        // first zero must also be zero.
        if s[n..].iter().any(|&c| c != 0) {
            return None;
        }
        &s[..n]
    };
    let v: i32 = match key {
        b"art-lojban" => c("_jbo") as i32,
        b"i-ami" => c("_ami") as i32,
        b"i-bnn" => c("_bnn") as i32,
        b"i-hak" => c("_hak") as i32,
        b"i-klingon" => c("_tlh") as i32,
        b"i-lux" => c("_lb") as i32,
        b"i-navajo" => c("_nv") as i32,
        b"i-pwn" => c("_pwn") as i32,
        b"i-tao" => c("_tao") as i32,
        b"i-tay" => c("_tay") as i32,
        b"i-tsu" => c("_tsu") as i32,
        b"no-bok" => c("_nb") as i32,
        b"no-nyn" => c("_nn") as i32,
        b"sgn-be-fr" => c("_sfb") as i32,
        b"sgn-be-nl" => c("_vgt") as i32,
        b"sgn-ch-de" => c("_sgg") as i32,
        b"zh-guoyu" => c("_cmn") as i32,
        b"zh-hakka" => c("_hak") as i32,
        b"zh-min-nan" => c("_nan") as i32,
        b"zh-xiang" => c("_hsn") as i32,
        // Grandfathered tags with no modern replacement will be converted
        // as follows:
        b"cel-gaulish" => -1,
        b"en-gb-oed" => -2,
        b"i-default" => -3,
        b"i-enochian" => -4,
        b"i-mingo" => -5,
        b"zh-min" => -6,
        // CLDR-specific tag.
        b"root" => 0,
        b"en-us-posix" => -7,
        _ => return None,
    };
    const ALT_TAG_INDEX: [usize; 8] = [0, 17, 31, 45, 61, 74, 86, 102];
    const ALT_TAGS: &str = "xtg-x-cel-gaulishen-GB-oxendicten-x-i-defaultund-x-i-enochiansee-x-i-mingonan-x-zh-minen-US-u-va-posix";
    if v < 0 {
        let k = (-v) as usize;
        return Some(make(&ALT_TAGS[ALT_TAG_INDEX[k - 1]..ALT_TAG_INDEX[k]]));
    }
    Some(ITag::core(v as Language, 0, 0))
}

// Go: internal/language/language.go:Make
pub(crate) fn make(s: &str) -> ITag {
    parse(s).0
}

// Go: internal/language/tags.go:MustParse
pub(crate) fn must_parse(s: &str) -> ITag {
    let (t, err) = parse(s);
    if let Some(e) = err {
        panic!("language.MustParse({s:?}): {e}");
    }
    t
}

// ---------------------------------------------------------------------------
// compose.go

/// A Builder allows constructing a Tag from individual components.
#[derive(Default)]
pub(crate) struct Builder {
    pub tag: ITag,
    private: String,
    variants: Vec<String>,
    extensions: Vec<String>,
    /// Set when Make's parse would have panicked in Go (Compose recovers
    /// it as ErrSyntax).
    pub(crate) panicked: bool,
}

struct SortVariants<'a>(&'a mut Vec<String>);

impl go_sort::sort::Interface for SortVariants<'_> {
    fn len(&self) -> usize {
        self.0.len()
    }
    fn swap(&mut self, i: usize, j: usize) {
        self.0.swap(i, j);
    }
    fn less(&mut self, i: usize, j: usize) -> bool {
        let vi = tables()
            .variant_index
            .get(self.0[i].as_bytes())
            .copied()
            .unwrap_or(0);
        let vj = tables()
            .variant_index
            .get(self.0[j].as_bytes())
            .copied()
            .unwrap_or(0);
        vi < vj
    }
}

impl Builder {
    // Go: internal/language/compose.go:Builder.Make
    /// Returns a new Tag from the current settings.
    pub(crate) fn make(&mut self) -> ITag {
        let mut t = self.tag.clone();

        if !self.extensions.is_empty() || !self.variants.is_empty() {
            go_sort::sort::sort(&mut SortVariants(&mut self.variants));
            go_sort::sort_by(&mut self.extensions, |a, b| a < b);

            if !self.private.is_empty() {
                self.extensions.push(self.private.clone());
            }
            // Go: n := maxCoreSize + tokenLen(b.variants...) + tokenLen(b.extensions...)
            let n = MAX_CORE_SIZE
                + self.variants.iter().map(|v| v.len() + 1).sum::<usize>()
                + self.extensions.iter().map(|e| e.len() + 1).sum::<usize>();
            let mut buf = t.gen_core_bytes();
            t.p_variant = buf.len() as u8;
            for v in &self.variants {
                buf.push('-');
                buf.push_str(v);
            }
            t.p_ext = buf.len() as u16;
            for e in &self.extensions {
                buf.push('-');
                buf.push_str(e);
            }
            t.str = buf.clone();
            // We may not always need to remake the string, but when or when
            // not to do so is rather tricky.
            // makeScanner(buf[:p]) over buf := make([]byte, n).
            let p = buf.len();
            let mut scan = Scanner::with_tail(buf.into_bytes(), vec![0u8; n.saturating_sub(p)]);
            let (t, _) = parse_scan(&mut scan, b"");
            if scan.panicked {
                self.panicked = true;
            }
            return t;
        } else if !self.private.is_empty() {
            t.str = self.private.clone();
            t.remake_string();
        }
        t
    }

    // Go: internal/language/compose.go:Builder.SetTag
    /// Copies all the settings from a given Tag. Any previously set values
    /// are discarded.
    pub(crate) fn set_tag(&mut self, t: &ITag) {
        self.tag.lang_id = t.lang_id;
        self.tag.region_id = t.region_id;
        self.tag.script_id = t.script_id;
        // TODO: optimize
        self.variants.clear();
        let variants = t.variants();
        if !variants.is_empty() {
            for vr in variants[1..].split('-') {
                self.variants.push(vr.to_string());
            }
        }
        self.extensions.clear();
        self.private.clear();
        for e in t.extensions() {
            self.add_ext(&e);
        }
    }

    // Go: internal/language/compose.go:Builder.AddExt
    pub(crate) fn add_ext(&mut self, e: &str) {
        let e0 = e.as_bytes()[0];
        if e0 == b'x' {
            if self.private.is_empty() {
                self.private = e.to_string();
            }
            return;
        }
        for s in self.extensions.iter_mut() {
            if s.as_bytes()[0] == e0 {
                if e0 == b'u' {
                    s.push_str(&e[1..]);
                }
                return;
            }
        }
        self.extensions.push(e.to_string());
    }

    // Go: internal/language/compose.go:Builder.SetExt
    pub(crate) fn set_ext(&mut self, e: &str) {
        let e0 = e.as_bytes()[0];
        if e0 == b'x' {
            self.private = e.to_string();
            return;
        }
        for s in self.extensions.iter_mut() {
            if s.as_bytes()[0] == e0 {
                if e0 == b'u' {
                    *s = format!("{}{}", e, &s[1..]);
                } else {
                    *s = e.to_string();
                }
                return;
            }
        }
        self.extensions.push(e.to_string());
    }

    // Go: internal/language/compose.go:Builder.AddVariant
    pub(crate) fn add_variant(&mut self, v: &[&str]) {
        for v in v {
            if !v.is_empty() {
                self.variants.push(v.to_string());
            }
        }
    }

    // Go: internal/language/compose.go:Builder.ClearVariants
    #[allow(dead_code)]
    pub(crate) fn clear_variants(&mut self) {
        self.variants.clear();
    }

    // Go: internal/language/compose.go:Builder.ClearExtensions
    pub(crate) fn clear_extensions(&mut self) {
        self.private.clear();
        self.extensions.clear();
    }
}

// ---------------------------------------------------------------------------
// compact.go (internal/language)

// Go: internal/language/compact.go:GetCompactCore
pub(crate) fn get_compact_core(t: &ITag) -> Option<u32> {
    if t.lang_id as u32 > tables().c("langNoIndexOffset") {
        return None;
    }
    let mut cci = (t.lang_id as u32) << (8 + 12);
    cci |= (t.script_id as u32) << 12;
    cci |= t.region_id as u32;
    Some(cci)
}

// Go: internal/language/compact.go:CompactCoreInfo.Tag
pub(crate) fn compact_core_tag(c: u32) -> ITag {
    ITag::core(
        (c >> 20) as Language,
        ((c >> 12) & 0xff) as Script,
        (c & 0x3ff) as Region,
    )
}
