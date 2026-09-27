//! Port of golang.org/x/text/internal/colltab/collelem.go.

use crate::goutf8::is_ideographic;

/// Level identifies the collation comparison level.
pub type Level = usize;

pub const PRIMARY: Level = 0;
pub const SECONDARY: Level = 1;
pub const TERTIARY: Level = 2;
pub const QUATERNARY: Level = 3;
pub const IDENTITY: Level = 4;
pub const NUM_LEVELS: usize = 5;

pub(crate) const DEFAULT_SECONDARY: i32 = 0x20;
pub(crate) const DEFAULT_TERTIARY: i32 = 0x2;
pub(crate) const MAX_TERTIARY: u8 = 0x1F;
/// `colltab.MaxQuaternary` (21 bits).
pub const MAX_QUATERNARY: i32 = 0x1FFFFF;

/// A collation element (Go `colltab.Elem`, a `uint32`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Elem(pub u32);

const MAX_CE: u32 = 0xAFFFFFFF;
/// `colltab.PrivateUse`.
pub const PRIVATE_USE: u32 = MIN_CONTRACT;
const MIN_CONTRACT: u32 = 0xC0000000;
const MAX_CONTRACT: u32 = 0xDFFFFFFF;
#[allow(dead_code)]
const MIN_EXPAND: u32 = 0xE0000000;
const MAX_EXPAND: u32 = 0xEFFFFFFF;
#[allow(dead_code)]
const MIN_DECOMP: u32 = 0xF0000000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CeType {
    Normal,
    ContractionIndex,
    ExpansionIndex,
    Decompose,
}

const CE_TYPE_MASK: u32 = 0xC0000000;
const CE_TYPE_MASK_EXT: u32 = 0xE0000000;
const CE_IGNORE_MASK: u32 = 0xF00FFFFF;
const CE_TYPE1: u32 = 0x40000000;
const CE_TYPE2: u32 = 0x00000000;
const CE_TYPE3_OR_4: u32 = 0x80000000;
const CE_TYPE4: u32 = 0xA0000000;
const CE_TYPE_Q: u32 = 0xC0000000;
/// `colltab.Ignore`.
pub const IGNORE: Elem = Elem(CE_TYPE4);
const FIRST_NON_PRIMARY: u32 = 0x80000000;
const LAST_SPECIAL_PRIMARY: u32 = 0xA0000000;
#[allow(dead_code)]
const SECONDARY_MASK: u32 = 0x80000000;
const HAS_TERTIARY_MASK: u32 = 0x40000000;
const PRIMARY_VALUE_MASK: u32 = 0x3FFFFE00;
const MAX_PRIMARY_BITS: u32 = 21;
const COMPACT_PRIMARY_BITS: u32 = 16;
const MAX_SECONDARY_BITS: u32 = 12;
const MAX_TERTIARY_BITS: u32 = 8;
const MAX_CCC_BITS: u32 = 8;
const MAX_SECONDARY_COMPACT_BITS: u32 = 8;
const MAX_SECONDARY_DIFF_BITS: u32 = 4;
const MAX_TERTIARY_COMPACT_BITS: u32 = 5;
const PRIMARY_SHIFT: u32 = 9;
const COMPACT_SECONDARY_SHIFT: u32 = 5;
const MIN_COMPACT_SECONDARY: i32 = DEFAULT_SECONDARY - 4;

impl Elem {
    // Go: internal/colltab/collelem.go:Elem.ctype
    pub(crate) fn ctype(self) -> CeType {
        let ce = self.0;
        if ce <= MAX_CE {
            return CeType::Normal;
        }
        if ce <= MAX_CONTRACT {
            CeType::ContractionIndex
        } else if ce <= MAX_EXPAND {
            CeType::ExpansionIndex
        } else {
            CeType::Decompose
        }
    }

    // Go: internal/colltab/collelem.go:Elem.Mask
    pub fn mask(self, _l: Level) -> u32 {
        0
    }

    // Go: internal/colltab/collelem.go:Elem.CCC
    pub fn ccc(self) -> u8 {
        let ce = self.0;
        if ce & CE_TYPE3_OR_4 != 0 {
            if ce & CE_TYPE4 == CE_TYPE3_OR_4 {
                return (ce >> 16) as u8;
            }
            return (ce >> 20) as u8;
        }
        0
    }

    // Go: internal/colltab/collelem.go:Elem.Primary
    pub fn primary(self) -> i32 {
        let ce = self.0;
        if ce >= FIRST_NON_PRIMARY {
            if ce > LAST_SPECIAL_PRIMARY {
                return 0;
            }
            return (ce as u16) as i32;
        }
        ((ce & PRIMARY_VALUE_MASK) >> PRIMARY_SHIFT) as i32
    }

    // Go: internal/colltab/collelem.go:Elem.Secondary
    pub fn secondary(self) -> i32 {
        let ce = self.0;
        match ce & CE_TYPE_MASK {
            CE_TYPE1 => (ce as u8) as i32,
            CE_TYPE2 => MIN_COMPACT_SECONDARY + ((ce >> COMPACT_SECONDARY_SHIFT) & 0xF) as i32,
            CE_TYPE3_OR_4 => {
                if ce < CE_TYPE4 {
                    return DEFAULT_SECONDARY;
                }
                ((ce >> 8) & 0xFFF) as i32
            }
            _ => 0, // ceTypeQ
        }
    }

    // Go: internal/colltab/collelem.go:Elem.Tertiary
    pub fn tertiary(self) -> u8 {
        let ce = self.0;
        if ce & HAS_TERTIARY_MASK == 0 {
            if ce & CE_TYPE3_OR_4 == 0 {
                return (ce & 0x1F) as u8;
            }
            if ce & CE_TYPE4 == CE_TYPE4 {
                return ce as u8;
            }
            return ((ce >> 24) as u8) & 0x1F; // type 2
        } else if ce & CE_TYPE_MASK == CE_TYPE1 {
            return DEFAULT_TERTIARY as u8;
        }
        // ce is a quaternary value.
        0
    }

    // Go: internal/colltab/collelem.go:Elem.updateTertiary
    pub(crate) fn update_tertiary(self, t: u8) -> Elem {
        let mut ce = self.0;
        if ce & CE_TYPE_MASK == CE_TYPE1 {
            // convert to type 4
            let mut nce = ce & PRIMARY_VALUE_MASK;
            nce |= ((ce as u8).wrapping_sub(MIN_COMPACT_SECONDARY as u8) as u32)
                << COMPACT_SECONDARY_SHIFT;
            ce = nce;
        } else if ce & CE_TYPE_MASK_EXT == CE_TYPE3_OR_4 {
            ce &= !((MAX_TERTIARY as u32) << 24);
            return Elem(ce | ((t as u32) << 24));
        } else {
            // type 2 or 4
            ce &= !(MAX_TERTIARY as u32);
        }
        Elem(ce | t as u32)
    }

    // Go: internal/colltab/collelem.go:Elem.Quaternary
    pub fn quaternary(self) -> i32 {
        let ce = self.0;
        if ce & CE_TYPE_MASK == CE_TYPE_Q {
            return ((ce & PRIMARY_VALUE_MASK) >> PRIMARY_SHIFT) as i32;
        } else if ce & CE_IGNORE_MASK == IGNORE.0 {
            return 0;
        }
        MAX_QUATERNARY
    }

    // Go: internal/colltab/collelem.go:Elem.Weight
    pub fn weight(self, l: Level) -> i32 {
        match l {
            PRIMARY => self.primary(),
            SECONDARY => self.secondary(),
            TERTIARY => self.tertiary() as i32,
            QUATERNARY => self.quaternary(),
            _ => 0,
        }
    }
}

// Go: internal/colltab/collelem.go:makeImplicitCE
pub(crate) fn make_implicit_ce(primary: i32) -> Elem {
    Elem(CE_TYPE1 | ((primary as u32).wrapping_shl(PRIMARY_SHIFT)) | DEFAULT_SECONDARY as u32)
}

// Go: internal/colltab/collelem.go:MakeElem
/// Returns an Elem for the given values, or an error (Go's `fmt.Errorf`
/// message) if the combination is invalid.
pub fn make_elem(primary: i32, secondary: i32, tertiary: i32, ccc: u8) -> Result<Elem, String> {
    let w = primary;
    if w >= 1 << MAX_PRIMARY_BITS || w < 0 {
        return Err(format!(
            "makeCE: primary weight out of bounds: {:x} >= {:x}",
            w,
            1 << MAX_PRIMARY_BITS
        ));
    }
    let w = secondary;
    if w >= 1 << MAX_SECONDARY_BITS || w < 0 {
        return Err(format!(
            "makeCE: secondary weight out of bounds: {:x} >= {:x}",
            w,
            1 << MAX_SECONDARY_BITS
        ));
    }
    let w = tertiary;
    if w >= 1 << MAX_TERTIARY_BITS || w < 0 {
        return Err(format!(
            "makeCE: tertiary weight out of bounds: {:x} >= {:x}",
            w,
            1 << MAX_TERTIARY_BITS
        ));
    }
    let mut ce: u32;
    if primary != 0 {
        if ccc != 0 {
            if primary >= 1 << COMPACT_PRIMARY_BITS {
                return Err(format!(
                    "makeCE: primary weight with non-zero CCC out of bounds: {:x} >= {:x}",
                    primary,
                    1 << COMPACT_PRIMARY_BITS
                ));
            }
            if secondary != DEFAULT_SECONDARY {
                return Err(format!(
                    "makeCE: cannot combine non-default secondary value ({:x}) with non-zero CCC ({:x})",
                    secondary, ccc
                ));
            }
            ce = (tertiary as u32) << (COMPACT_PRIMARY_BITS + MAX_CCC_BITS);
            ce |= (ccc as u32) << COMPACT_PRIMARY_BITS;
            ce |= primary as u32;
            ce |= CE_TYPE3_OR_4;
        } else if tertiary == DEFAULT_TERTIARY {
            if secondary >= 1 << MAX_SECONDARY_COMPACT_BITS {
                return Err(format!(
                    "makeCE: secondary weight with non-zero primary out of bounds: {:x} >= {:x}",
                    secondary,
                    1 << MAX_SECONDARY_COMPACT_BITS
                ));
            }
            ce = ((primary << (MAX_SECONDARY_COMPACT_BITS + 1)) + secondary) as u32;
            ce |= CE_TYPE1;
        } else {
            let d = secondary - DEFAULT_SECONDARY + MAX_SECONDARY_DIFF_BITS as i32;
            if d >= 1 << MAX_SECONDARY_DIFF_BITS || d < 0 {
                return Err(format!(
                    "makeCE: secondary weight diff out of bounds: {:x} < 0 || {:x} > {:x}",
                    d,
                    d,
                    1 << MAX_SECONDARY_DIFF_BITS
                ));
            }
            if tertiary >= 1 << MAX_TERTIARY_COMPACT_BITS {
                return Err(format!(
                    "makeCE: tertiary weight with non-zero primary out of bounds: {:x} > {:x}",
                    tertiary,
                    1 << MAX_TERTIARY_COMPACT_BITS
                ));
            }
            ce = ((primary << MAX_SECONDARY_DIFF_BITS) + d) as u32;
            ce = (ce << MAX_TERTIARY_COMPACT_BITS) + tertiary as u32;
        }
    } else {
        ce = ((secondary << MAX_TERTIARY_BITS) + tertiary) as u32;
        ce = ce.wrapping_add((ccc as u32) << (MAX_SECONDARY_BITS + MAX_TERTIARY_BITS));
        ce |= CE_TYPE4;
    }
    Ok(Elem(ce))
}

// Go: internal/colltab/collelem.go:MakeQuaternary
pub fn make_quaternary(v: i32) -> Elem {
    Elem(CE_TYPE_Q | (v as u32).wrapping_shl(PRIMARY_SHIFT))
}

const MAX_N_BITS: u32 = 4;
const MAX_TRIE_INDEX_BITS: u32 = 12;
const MAX_CONTRACT_OFFSET_BITS: u32 = 13;

// Go: internal/colltab/collelem.go:splitContractIndex
pub(crate) fn split_contract_index(ce: Elem) -> (usize, usize, usize) {
    let mut ce = ce.0;
    let n = (ce & ((1 << MAX_N_BITS) - 1)) as usize;
    ce >>= MAX_N_BITS;
    let index = (ce & ((1 << MAX_TRIE_INDEX_BITS) - 1)) as usize;
    ce >>= MAX_TRIE_INDEX_BITS;
    let offset = (ce & ((1 << MAX_CONTRACT_OFFSET_BITS) - 1)) as usize;
    (index, n, offset)
}

// Go: internal/colltab/collelem.go:splitExpandIndex
pub(crate) fn split_expand_index(ce: Elem) -> usize {
    (ce.0 as u16) as usize
}

// Go: internal/colltab/collelem.go:splitDecompose
pub(crate) fn split_decompose(ce: Elem) -> (u8, u8) {
    (ce.0 as u8, (ce.0 >> 8) as u8)
}

const MIN_UNIFIED: i32 = 0x4E00;
const MAX_UNIFIED: i32 = 0x9FFF;
const MIN_COMPATIBILITY: i32 = 0xF900;
const MAX_COMPATIBILITY: i32 = 0xFAFF;
#[allow(dead_code)]
const MIN_RARE: i32 = 0x3400;
#[allow(dead_code)]
const MAX_RARE: i32 = 0x4DBF;

const COMMON_UNIFIED_OFFSET: i32 = 0x10000;
const RARE_UNIFIED_OFFSET: i32 = 0x20000;
const OTHER_OFFSET: i32 = 0x50000;

// Go: internal/colltab/collelem.go:implicitPrimary
pub(crate) fn implicit_primary(r: i32) -> i32 {
    if is_ideographic(r) {
        if (MIN_UNIFIED..=MAX_UNIFIED).contains(&r) {
            // The most common case for CJK.
            return r + COMMON_UNIFIED_OFFSET;
        }
        if (MIN_COMPATIBILITY..=MAX_COMPATIBILITY).contains(&r) {
            // This will typically not hit. The DUCET explicitly specifies mappings
            // for all characters that do not decompose.
            return r + COMMON_UNIFIED_OFFSET;
        }
        return r + RARE_UNIFIED_OFFSET;
    }
    r + OTHER_OFFSET
}
