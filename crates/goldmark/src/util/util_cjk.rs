// Go: github.com/yuin/goldmark@v1.7.12/util/util_cjk.go

use go_unicode::{Range16, Range32, RangeTable, Rune};

static CJK_RADICALS_SUPPLEMENT: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x2E80,
        hi: 0x2EFF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static KANGXI_RADICALS: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x2F00,
        hi: 0x2FDF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static IDEOGRAPHIC_DESCRIPTION_CHARACTERS: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x2FF0,
        hi: 0x2FFF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static CJK_SYMBOLS_AND_PUNCTUATION: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x3000,
        hi: 0x303F,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static HIRAGANA: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x3040,
        hi: 0x309F,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static KATAKANA: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x30A0,
        hi: 0x30FF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static KANBUN: RangeTable = RangeTable {
    r16: &[
        Range16 {
            lo: 0x3130,
            hi: 0x318F,
            stride: 1,
        },
        Range16 {
            lo: 0x3190,
            hi: 0x319F,
            stride: 1,
        },
    ],
    r32: &[],
    latin_offset: 0,
};

static CJK_STROKES: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x31C0,
        hi: 0x31EF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static KATAKANA_PHONETIC_EXTENSIONS: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x31F0,
        hi: 0x31FF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static CJK_COMPATIBILITY: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x3300,
        hi: 0x33FF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static CJK_UNIFIED_IDEOGRAPHS_EXTENSION_A: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x3400,
        hi: 0x4DBF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static CJK_UNIFIED_IDEOGRAPHS: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0x4E00,
        hi: 0x9FFF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static YI_SYLLABLES: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0xA000,
        hi: 0xA48F,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static YI_RADICALS: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0xA490,
        hi: 0xA4CF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static CJK_COMPATIBILITY_IDEOGRAPHS: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0xF900,
        hi: 0xFAFF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static VERTICAL_FORMS: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0xFE10,
        hi: 0xFE1F,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static CJK_COMPATIBILITY_FORMS: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0xFE30,
        hi: 0xFE4F,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static SMALL_FORM_VARIANTS: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0xFE50,
        hi: 0xFE6F,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static HALFWIDTH_AND_FULLWIDTH_FORMS: RangeTable = RangeTable {
    r16: &[Range16 {
        lo: 0xFF00,
        hi: 0xFFEF,
        stride: 1,
    }],
    r32: &[],
    latin_offset: 0,
};

static KANA_SUPPLEMENT: RangeTable = RangeTable {
    r16: &[],
    r32: &[Range32 {
        lo: 0x1B000,
        hi: 0x1B0FF,
        stride: 1,
    }],
    latin_offset: 0,
};

static KANA_EXTENDED_A: RangeTable = RangeTable {
    r16: &[],
    r32: &[Range32 {
        lo: 0x1B100,
        hi: 0x1B12F,
        stride: 1,
    }],
    latin_offset: 0,
};

static SMALL_KANA_EXTENSION: RangeTable = RangeTable {
    r16: &[],
    r32: &[Range32 {
        lo: 0x1B130,
        hi: 0x1B16F,
        stride: 1,
    }],
    latin_offset: 0,
};

static CJK_UNIFIED_IDEOGRAPHS_EXTENSION_B: RangeTable = RangeTable {
    r16: &[],
    r32: &[Range32 {
        lo: 0x20000,
        hi: 0x2A6DF,
        stride: 1,
    }],
    latin_offset: 0,
};

static CJK_UNIFIED_IDEOGRAPHS_EXTENSION_C: RangeTable = RangeTable {
    r16: &[],
    r32: &[Range32 {
        lo: 0x2A700,
        hi: 0x2B73F,
        stride: 1,
    }],
    latin_offset: 0,
};

static CJK_UNIFIED_IDEOGRAPHS_EXTENSION_D: RangeTable = RangeTable {
    r16: &[],
    r32: &[Range32 {
        lo: 0x2B740,
        hi: 0x2B81F,
        stride: 1,
    }],
    latin_offset: 0,
};

static CJK_UNIFIED_IDEOGRAPHS_EXTENSION_E: RangeTable = RangeTable {
    r16: &[],
    r32: &[Range32 {
        lo: 0x2B820,
        hi: 0x2CEAF,
        stride: 1,
    }],
    latin_offset: 0,
};

static CJK_UNIFIED_IDEOGRAPHS_EXTENSION_F: RangeTable = RangeTable {
    r16: &[],
    r32: &[Range32 {
        lo: 0x2CEB0,
        hi: 0x2EBEF,
        stride: 1,
    }],
    latin_offset: 0,
};

static CJK_COMPATIBILITY_IDEOGRAPHS_SUPPLEMENT: RangeTable = RangeTable {
    r16: &[],
    r32: &[Range32 {
        lo: 0x2F800,
        hi: 0x2FA1F,
        stride: 1,
    }],
    latin_offset: 0,
};

static CJK_UNIFIED_IDEOGRAPHS_EXTENSION_G: RangeTable = RangeTable {
    r16: &[],
    r32: &[Range32 {
        lo: 0x30000,
        hi: 0x3134F,
        stride: 1,
    }],
    latin_offset: 0,
};

// Go: util/util_cjk.go:IsEastAsianWideRune
/// IsEastAsianWideRune returns true if the given rune is an east asian wide character, otherwise false.
pub fn is_east_asian_wide_rune(r: Rune) -> bool {
    go_unicode::is(go_unicode::HIRAGANA, r)
        || go_unicode::is(go_unicode::KATAKANA, r)
        || go_unicode::is(go_unicode::HAN, r)
        || go_unicode::is(go_unicode::LM, r)
        || go_unicode::is(go_unicode::HANGUL, r)
        || go_unicode::is(&CJK_SYMBOLS_AND_PUNCTUATION, r)
}

// Go: util/util_cjk.go:IsSpaceDiscardingUnicodeRune
/// IsSpaceDiscardingUnicodeRune returns true if the given rune is space-discarding unicode character, otherwise false.
/// See https://www.w3.org/TR/2020/WD-css-text-3-20200429/#space-discard-set
pub fn is_space_discarding_unicode_rune(r: Rune) -> bool {
    go_unicode::is(&CJK_RADICALS_SUPPLEMENT, r)
        || go_unicode::is(&KANGXI_RADICALS, r)
        || go_unicode::is(&IDEOGRAPHIC_DESCRIPTION_CHARACTERS, r)
        || go_unicode::is(&CJK_SYMBOLS_AND_PUNCTUATION, r)
        || go_unicode::is(&HIRAGANA, r)
        || go_unicode::is(&KATAKANA, r)
        || go_unicode::is(&KANBUN, r)
        || go_unicode::is(&CJK_STROKES, r)
        || go_unicode::is(&KATAKANA_PHONETIC_EXTENSIONS, r)
        || go_unicode::is(&CJK_COMPATIBILITY, r)
        || go_unicode::is(&CJK_UNIFIED_IDEOGRAPHS_EXTENSION_A, r)
        || go_unicode::is(&CJK_UNIFIED_IDEOGRAPHS, r)
        || go_unicode::is(&YI_SYLLABLES, r)
        || go_unicode::is(&YI_RADICALS, r)
        || go_unicode::is(&CJK_COMPATIBILITY_IDEOGRAPHS, r)
        || go_unicode::is(&VERTICAL_FORMS, r)
        || go_unicode::is(&CJK_COMPATIBILITY_FORMS, r)
        || go_unicode::is(&SMALL_FORM_VARIANTS, r)
        || go_unicode::is(&HALFWIDTH_AND_FULLWIDTH_FORMS, r)
        || go_unicode::is(&KANA_SUPPLEMENT, r)
        || go_unicode::is(&KANA_EXTENDED_A, r)
        || go_unicode::is(&SMALL_KANA_EXTENSION, r)
        || go_unicode::is(&CJK_UNIFIED_IDEOGRAPHS_EXTENSION_B, r)
        || go_unicode::is(&CJK_UNIFIED_IDEOGRAPHS_EXTENSION_C, r)
        || go_unicode::is(&CJK_UNIFIED_IDEOGRAPHS_EXTENSION_D, r)
        || go_unicode::is(&CJK_UNIFIED_IDEOGRAPHS_EXTENSION_E, r)
        || go_unicode::is(&CJK_UNIFIED_IDEOGRAPHS_EXTENSION_F, r)
        || go_unicode::is(&CJK_COMPATIBILITY_IDEOGRAPHS_SUPPLEMENT, r)
        || go_unicode::is(&CJK_UNIFIED_IDEOGRAPHS_EXTENSION_G, r)
}

// Go: util/util_cjk.go:EastAsianWidth
/// EastAsianWidth returns the east asian width of the given rune.
/// See https://www.unicode.org/reports/tr11/tr11-36.html
pub fn east_asian_width(r: Rune) -> &'static str {
    if r == 0x3000 || (0xFF01..=0xFF60).contains(&r) || (0xFFE0..=0xFFE6).contains(&r) {
        "F"
    } else if r == 0x20A9
        || (0xFF61..=0xFFBE).contains(&r)
        || (0xFFC2..=0xFFC7).contains(&r)
        || (0xFFCA..=0xFFCF).contains(&r)
        || (0xFFD2..=0xFFD7).contains(&r)
        || (0xFFDA..=0xFFDC).contains(&r)
        || (0xFFE8..=0xFFEE).contains(&r)
    {
        "H"
    } else if (0x1100..=0x115F).contains(&r)
        || (0x11A3..=0x11A7).contains(&r)
        || (0x11FA..=0x11FF).contains(&r)
        || (0x2329..=0x232A).contains(&r)
        || (0x2E80..=0x2E99).contains(&r)
        || (0x2E9B..=0x2EF3).contains(&r)
        || (0x2F00..=0x2FD5).contains(&r)
        || (0x2FF0..=0x2FFB).contains(&r)
        || (0x3001..=0x303E).contains(&r)
        || (0x3041..=0x3096).contains(&r)
        || (0x3099..=0x30FF).contains(&r)
        || (0x3105..=0x312D).contains(&r)
        || (0x3131..=0x318E).contains(&r)
        || (0x3190..=0x31BA).contains(&r)
        || (0x31C0..=0x31E3).contains(&r)
        || (0x31F0..=0x321E).contains(&r)
        || (0x3220..=0x3247).contains(&r)
        || (0x3250..=0x32FE).contains(&r)
        || (0x3300..=0x4DBF).contains(&r)
        || (0x4E00..=0xA48C).contains(&r)
        || (0xA490..=0xA4C6).contains(&r)
        || (0xA960..=0xA97C).contains(&r)
        || (0xAC00..=0xD7A3).contains(&r)
        || (0xD7B0..=0xD7C6).contains(&r)
        || (0xD7CB..=0xD7FB).contains(&r)
        || (0xF900..=0xFAFF).contains(&r)
        || (0xFE10..=0xFE19).contains(&r)
        || (0xFE30..=0xFE52).contains(&r)
        || (0xFE54..=0xFE66).contains(&r)
        || (0xFE68..=0xFE6B).contains(&r)
        || (0x1B000..=0x1B001).contains(&r)
        || (0x1F200..=0x1F202).contains(&r)
        || (0x1F210..=0x1F23A).contains(&r)
        || (0x1F240..=0x1F248).contains(&r)
        || (0x1F250..=0x1F251).contains(&r)
        || (0x20000..=0x2F73F).contains(&r)
        || (0x2B740..=0x2FFFD).contains(&r)
        || (0x30000..=0x3FFFD).contains(&r)
    {
        "W"
    } else if (0x0020..=0x007E).contains(&r)
        || (0x00A2..=0x00A3).contains(&r)
        || (0x00A5..=0x00A6).contains(&r)
        || r == 0x00AC
        || r == 0x00AF
        || (0x27E6..=0x27ED).contains(&r)
        || (0x2985..=0x2986).contains(&r)
    {
        "Na"
    } else if r == 0x00A1
        || r == 0x00A4
        || (0x00A7..=0x00A8).contains(&r)
        || r == 0x00AA
        || (0x00AD..=0x00AE).contains(&r)
        || (0x00B0..=0x00B4).contains(&r)
        || (0x00B6..=0x00BA).contains(&r)
        || (0x00BC..=0x00BF).contains(&r)
        || r == 0x00C6
        || r == 0x00D0
        || (0x00D7..=0x00D8).contains(&r)
        || (0x00DE..=0x00E1).contains(&r)
        || r == 0x00E6
        || (0x00E8..=0x00EA).contains(&r)
        || (0x00EC..=0x00ED).contains(&r)
        || r == 0x00F0
        || (0x00F2..=0x00F3).contains(&r)
        || (0x00F7..=0x00FA).contains(&r)
        || r == 0x00FC
        || r == 0x00FE
        || r == 0x0101
        || r == 0x0111
        || r == 0x0113
        || r == 0x011B
        || (0x0126..=0x0127).contains(&r)
        || r == 0x012B
        || (0x0131..=0x0133).contains(&r)
        || r == 0x0138
        || (0x013F..=0x0142).contains(&r)
        || r == 0x0144
        || (0x0148..=0x014B).contains(&r)
        || r == 0x014D
        || (0x0152..=0x0153).contains(&r)
        || (0x0166..=0x0167).contains(&r)
        || r == 0x016B
        || r == 0x01CE
        || r == 0x01D0
        || r == 0x01D2
        || r == 0x01D4
        || r == 0x01D6
        || r == 0x01D8
        || r == 0x01DA
        || r == 0x01DC
        || r == 0x0251
        || r == 0x0261
        || r == 0x02C4
        || r == 0x02C7
        || (0x02C9..=0x02CB).contains(&r)
        || r == 0x02CD
        || r == 0x02D0
        || (0x02D8..=0x02DB).contains(&r)
        || r == 0x02DD
        || r == 0x02DF
        || (0x0300..=0x036F).contains(&r)
        || (0x0391..=0x03A1).contains(&r)
        || (0x03A3..=0x03A9).contains(&r)
        || (0x03B1..=0x03C1).contains(&r)
        || (0x03C3..=0x03C9).contains(&r)
        || r == 0x0401
        || (0x0410..=0x044F).contains(&r)
        || r == 0x0451
        || r == 0x2010
        || (0x2013..=0x2016).contains(&r)
        || (0x2018..=0x2019).contains(&r)
        || (0x201C..=0x201D).contains(&r)
        || (0x2020..=0x2022).contains(&r)
        || (0x2024..=0x2027).contains(&r)
        || r == 0x2030
        || (0x2032..=0x2033).contains(&r)
        || r == 0x2035
        || r == 0x203B
        || r == 0x203E
        || r == 0x2074
        || r == 0x207F
        || (0x2081..=0x2084).contains(&r)
        || r == 0x20AC
        || r == 0x2103
        || r == 0x2105
        || r == 0x2109
        || r == 0x2113
        || r == 0x2116
        || (0x2121..=0x2122).contains(&r)
        || r == 0x2126
        || r == 0x212B
        || (0x2153..=0x2154).contains(&r)
        || (0x215B..=0x215E).contains(&r)
        || (0x2160..=0x216B).contains(&r)
        || (0x2170..=0x2179).contains(&r)
        || r == 0x2189
        || (0x2190..=0x2199).contains(&r)
        || (0x21B8..=0x21B9).contains(&r)
        || r == 0x21D2
        || r == 0x21D4
        || r == 0x21E7
        || r == 0x2200
        || (0x2202..=0x2203).contains(&r)
        || (0x2207..=0x2208).contains(&r)
        || r == 0x220B
        || r == 0x220F
        || r == 0x2211
        || r == 0x2215
        || r == 0x221A
        || (0x221D..=0x2220).contains(&r)
        || r == 0x2223
        || r == 0x2225
        || (0x2227..=0x222C).contains(&r)
        || r == 0x222E
        || (0x2234..=0x2237).contains(&r)
        || (0x223C..=0x223D).contains(&r)
        || r == 0x2248
        || r == 0x224C
        || r == 0x2252
        || (0x2260..=0x2261).contains(&r)
        || (0x2264..=0x2267).contains(&r)
        || (0x226A..=0x226B).contains(&r)
        || (0x226E..=0x226F).contains(&r)
        || (0x2282..=0x2283).contains(&r)
        || (0x2286..=0x2287).contains(&r)
        || r == 0x2295
        || r == 0x2299
        || r == 0x22A5
        || r == 0x22BF
        || r == 0x2312
        || (0x2460..=0x24E9).contains(&r)
        || (0x24EB..=0x254B).contains(&r)
        || (0x2550..=0x2573).contains(&r)
        || (0x2580..=0x258F).contains(&r)
        || (0x2592..=0x2595).contains(&r)
        || (0x25A0..=0x25A1).contains(&r)
        || (0x25A3..=0x25A9).contains(&r)
        || (0x25B2..=0x25B3).contains(&r)
        || (0x25B6..=0x25B7).contains(&r)
        || (0x25BC..=0x25BD).contains(&r)
        || (0x25C0..=0x25C1).contains(&r)
        || (0x25C6..=0x25C8).contains(&r)
        || r == 0x25CB
        || (0x25CE..=0x25D1).contains(&r)
        || (0x25E2..=0x25E5).contains(&r)
        || r == 0x25EF
        || (0x2605..=0x2606).contains(&r)
        || r == 0x2609
        || (0x260E..=0x260F).contains(&r)
        || (0x2614..=0x2615).contains(&r)
        || r == 0x261C
        || r == 0x261E
        || r == 0x2640
        || r == 0x2642
        || (0x2660..=0x2661).contains(&r)
        || (0x2663..=0x2665).contains(&r)
        || (0x2667..=0x266A).contains(&r)
        || (0x266C..=0x266D).contains(&r)
        || r == 0x266F
        || (0x269E..=0x269F).contains(&r)
        || (0x26BE..=0x26BF).contains(&r)
        || (0x26C4..=0x26CD).contains(&r)
        || (0x26CF..=0x26E1).contains(&r)
        || r == 0x26E3
        || (0x26E8..=0x26FF).contains(&r)
        || r == 0x273D
        || r == 0x2757
        || (0x2776..=0x277F).contains(&r)
        || (0x2B55..=0x2B59).contains(&r)
        || (0x3248..=0x324F).contains(&r)
        || (0xE000..=0xF8FF).contains(&r)
        || (0xFE00..=0xFE0F).contains(&r)
        || r == 0xFFFD
        || (0x1F100..=0x1F10A).contains(&r)
        || (0x1F110..=0x1F12D).contains(&r)
        || (0x1F130..=0x1F169).contains(&r)
        || (0x1F170..=0x1F19A).contains(&r)
        || (0xE0100..=0xE01EF).contains(&r)
        || (0xF0000..=0xFFFFD).contains(&r)
        || (0x100000..=0x10FFFD).contains(&r)
    {
        "A"
    } else {
        "N"
    }
}
