//! Module `prose`.
//!
//! PORT jdkato/prose@v1.2.1 transform/title.go (AP style, including the rune-count/byte-offset bug)
//!
//! Owner: Wave B task T26 (common-thirdparty-ports).


//! Port of `github.com/jdkato/prose@v1.2.1/transform/title.go` (AP / Chicago title case), used by
//! `helpers.GetTitleFunc` (`CreateTitle`). Keep the `RuneCount`-as-byte-offset bug and the ASCII-only
//! `\s` of Go RE2 (see specs/content-model.md §6.1).

/// Go: `transform.IgnoreCase`/`APStyle`/`ChicagoStyle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TitleStyle {
    Ap,
    Chicago,
}

/// Go: `transform.TitleConverter`.
#[derive(Clone, Debug)]
pub struct TitleConverter {
    pub style: TitleStyle,
}

impl TitleConverter {
    // Go: prose transform/title.go:NewTitleConverter
    pub fn new(style: TitleStyle) -> Self {
        TitleConverter { style }
    }

    // Go: prose transform/title.go:Title
    pub fn title(&self, s: &str) -> String {
        todo!()
    }
}
