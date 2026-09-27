//! Port of golang.org/x/text/collate/index.go.

use super::tables::tables;
use crate::colltab::table::Table;
use crate::colltab::trie::Trie;

const BLOCK_SIZE: usize = 64;

// Go: collate/index.go:getTable
pub(crate) fn get_table(t: TableIndex) -> Table {
    let m = tables();
    Table {
        index: Trie {
            index0: &m.main_lookup[BLOCK_SIZE * t.lookup_offset as usize..],
            values0: &m.main_values[BLOCK_SIZE * t.values_offset as usize..],
            index: &m.main_lookup[..],
            values: &m.main_values[..],
        },
        expand_elem: &m.main_expand_elem[..],
        contract_tries: &m.main_ct_entries[..],
        contract_elem: &m.main_contract_elem[..],
        max_contract_len: 18,
        variable_top: m.var_top,
    }
}

/// tableIndex holds information for constructing a table for a certain
/// locale based on the main table.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TableIndex {
    pub lookup_offset: u32,
    pub values_offset: u32,
}
