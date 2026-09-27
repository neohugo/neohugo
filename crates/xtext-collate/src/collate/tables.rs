//! The generated tables of golang.org/x/text@v0.26.0/collate/tables.go
//! (CLDR 23, UCA/Unicode 6.2.0), extracted by
//! `tools/go-oracle/xtext-collate gen` into `data/collate.bin`.

use std::sync::OnceLock;

use super::index::TableIndex;
use crate::blob::Blob;
use crate::colltab::contract::CtEntry;

pub(crate) struct CollateTables {
    pub unicode_version: &'static str,
    pub cldr_version: &'static str,
    pub available_locales: &'static str,
    pub var_top: u32,
    pub locales: Vec<TableIndex>,
    pub main_expand_elem: Vec<u32>,
    pub main_contract_elem: Vec<u32>,
    pub main_values: Vec<u32>,
    pub main_lookup: Vec<u16>,
    pub main_ct_entries: Vec<CtEntry>,
}

pub(crate) fn tables() -> &'static CollateTables {
    static T: OnceLock<CollateTables> = OnceLock::new();
    T.get_or_init(|| {
        let b = Blob::parse(include_bytes!("../../data/collate.bin"));
        let locales = b
            .u32s("locales")
            .as_chunks::<2>()
            .0
            .iter()
            .map(|c| TableIndex {
                lookup_offset: c[0],
                values_offset: c[1],
            })
            .collect();
        let main_ct_entries = b
            .bytes("mainCTEntries")
            .as_chunks::<4>()
            .0
            .iter()
            .map(|c| CtEntry {
                l: c[0],
                h: c[1],
                n: c[2],
                i: c[3],
            })
            .collect();
        CollateTables {
            unicode_version: b.str("UnicodeVersion"),
            cldr_version: b.str("CLDRVersion"),
            available_locales: b.str("availableLocales"),
            var_top: b.u32("varTop"),
            locales,
            main_expand_elem: b.u32s("mainExpandElem"),
            main_contract_elem: b.u32s("mainContractElem"),
            main_values: b.u32s("mainValues"),
            main_lookup: b.u16s("mainLookup"),
            main_ct_entries,
        }
    })
}

/// `collate.UnicodeVersion`: the Unicode version from which the tables are derived.
pub fn unicode_version() -> &'static str {
    tables().unicode_version
}

/// `collate.CLDRVersion`: the CLDR version from which the tables are derived.
pub fn cldr_version() -> &'static str {
    tables().cldr_version
}
