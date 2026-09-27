//! The generated tables of golang.org/x/text@v0.26.0/internal/language
//! (tables.go, CLDR 32) and internal/language/compact (tables.go), extracted
//! by `tools/go-oracle/xtext-collate gen` into `data/language.bin`.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::blob::Blob;

pub(crate) struct ParentRel {
    pub lang: u16,
    pub script: u16,
    pub max_script: u16,
    pub to_region: u16,
    pub from_region: Vec<u16>,
}

#[derive(Clone, Copy, Default)]
pub(crate) struct Triple {
    pub a: u16,
    pub b: u16,
    pub c: u16,
}

pub(crate) struct LangTables {
    pub lang: &'static [u8],
    pub alt_lang_iso3: &'static [u8],
    pub script: &'static [u8],
    pub region_iso: &'static [u8],
    pub alt_region_iso3: &'static [u8],

    pub lang_no_index: &'static [u8],
    pub alt_lang_index: Vec<u16>,
    pub alias_map: Vec<(u16, u16)>,
    pub alias_types: &'static [u8],
    pub suppress_script: &'static [u8],
    pub region_types: &'static [u8],
    pub alt_region_ids: Vec<u16>,
    pub region_old_map: Vec<(u16, u16)>,
    pub m49: Vec<u16>,
    pub m49_index: Vec<u16>,
    pub from_m49: Vec<u16>,
    pub region_containment: Vec<u64>,
    pub region_inclusion: &'static [u8],
    pub region_inclusion_bits: Vec<u64>,
    #[allow(dead_code)]
    pub region_inclusion_next: &'static [u8],
    pub variant_index: HashMap<&'static [u8], u8>,

    /// likelyScript: (lang, region)
    pub likely_script: Vec<Triple>,
    /// likelyLang / likelyLangList: (region, script, flags)
    pub likely_lang: Vec<Triple>,
    pub likely_lang_list: Vec<Triple>,
    /// likelyRegion / likelyRegionList: (lang, script, flags)
    pub likely_region: Vec<Triple>,
    pub likely_region_list: Vec<Triple>,
    /// likelyRegionGroup: (lang, region, script)
    pub likely_region_group: Vec<Triple>,
    pub parents: Vec<ParentRel>,

    pub core_tags: Vec<u32>,
    pub special_tags_str: &'static str,

    pub consts: HashMap<&'static str, u32>,
}

impl LangTables {
    pub(crate) fn c(&self, name: &str) -> u32 {
        *self
            .consts
            .get(name)
            .unwrap_or_else(|| panic!("language const {name} missing"))
    }
}

pub(crate) fn tables() -> &'static LangTables {
    static T: OnceLock<LangTables> = OnceLock::new();
    T.get_or_init(|| {
        let b = Blob::parse(include_bytes!("../../data/language.bin"));
        let pairs = |name: &str| {
            b.u16s(name)
                .as_chunks::<2>()
                .0
                .iter()
                .map(|c| (c[0], c[1]))
                .collect::<Vec<_>>()
        };
        let triples = |name: &str, n: usize| {
            b.u16s(name)
                .chunks_exact(n)
                .map(|c| Triple {
                    a: c[0],
                    b: c[1],
                    c: if n > 2 { c[2] } else { 0 },
                })
                .collect::<Vec<_>>()
        };
        let mut parents = Vec::new();
        let pv = b.u16s("parents");
        let mut i = 0;
        while i < pv.len() {
            let n = pv[i + 4] as usize;
            parents.push(ParentRel {
                lang: pv[i],
                script: pv[i + 1],
                max_script: pv[i + 2],
                to_region: pv[i + 3],
                from_region: pv[i + 5..i + 5 + n].to_vec(),
            });
            i += 5 + n;
        }
        let mut variant_index = HashMap::new();
        let vb = b.bytes("variantIndex");
        let mut p = 0;
        while p < vb.len() {
            let z = p + vb[p..].iter().position(|&c| c == 0).expect("variantIndex");
            variant_index.insert(&vb[p..z], vb[z + 1]);
            p = z + 2;
        }
        let mut consts = HashMap::new();
        for name in [
            "nonCanonicalUnd",
            "langPrivateStart",
            "langPrivateEnd",
            "langNoIndexOffset",
            "isoRegionOffset",
            "nRegionGroups",
            "NumLanguages",
            "NumScripts",
            "NumRegions",
            "_en",
            "_sh",
            "_nb",
            "_mo",
            "_no",
            "_jbo",
            "_ami",
            "_bnn",
            "_hak",
            "_tlh",
            "_lb",
            "_nv",
            "_pwn",
            "_tao",
            "_tay",
            "_tsu",
            "_nn",
            "_sfb",
            "_vgt",
            "_sgg",
            "_cmn",
            "_nan",
            "_hsn",
            "_Latn",
            "_Hani",
            "_Hans",
            "_Hant",
            "_Qaaa",
            "_Qaai",
            "_Qabx",
            "_Zinh",
            "_Zyyy",
            "_Zzzz",
            "_001",
            "_419",
            "_BR",
            "_CA",
            "_ES",
            "_GB",
            "_MD",
            "_PT",
            "_UK",
            "_US",
            "_ZZ",
            "_XA",
            "_XC",
            "_XK",
        ] {
            let key: &'static str = name;
            consts.insert(key, b.u32(&format!("const:{name}")));
        }
        LangTables {
            lang: b.bytes("lang"),
            alt_lang_iso3: b.bytes("altLangISO3"),
            script: b.bytes("script"),
            region_iso: b.bytes("regionISO"),
            alt_region_iso3: b.bytes("altRegionISO3"),
            lang_no_index: b.bytes("langNoIndex"),
            alt_lang_index: b.u16s("altLangIndex"),
            alias_map: pairs("AliasMap"),
            alias_types: b.bytes("AliasTypes"),
            suppress_script: b.bytes("suppressScript"),
            region_types: b.bytes("regionTypes"),
            alt_region_ids: b.u16s("altRegionIDs"),
            region_old_map: pairs("regionOldMap"),
            m49: b.u16s("m49"),
            m49_index: b.u16s("m49Index"),
            from_m49: b.u16s("fromM49"),
            region_containment: b.u64s("regionContainment"),
            region_inclusion: b.bytes("regionInclusion"),
            region_inclusion_bits: b.u64s("regionInclusionBits"),
            region_inclusion_next: b.bytes("regionInclusionNext"),
            variant_index,
            likely_script: triples("likelyScript", 2),
            likely_lang: triples("likelyLang", 3),
            likely_lang_list: triples("likelyLangList", 3),
            likely_region: triples("likelyRegion", 3),
            likely_region_list: triples("likelyRegionList", 3),
            likely_region_group: triples("likelyRegionGroup", 3),
            parents,
            core_tags: b.u32s("coreTags"),
            special_tags_str: b.str("specialTagsStr"),
            consts,
        }
    })
}
