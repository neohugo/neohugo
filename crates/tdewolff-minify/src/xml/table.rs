//! Go: minify/xml/table.go

use tdewolff_parse::{EntityMap, RevEntityMap};

/// Go: xml.EntitiesMap — all named character entities.
pub struct EntitiesMapT;

/// Go: xml.EntitiesMap
pub static EntitiesMap: EntitiesMapT = EntitiesMapT;

impl EntityMap for EntitiesMapT {
    fn lookup_entity(&self, name: &[u8]) -> Option<&[u8]> {
        match name {
            b"apos" => Some(b"'"),
            b"gt" => Some(b">"),
            b"quot" => Some(b"\""),
            _ => None,
        }
    }
}

/// Go: xml.TextRevEntitiesMap — a map of escapes.
pub struct TextRevEntitiesMapT;

/// Go: xml.TextRevEntitiesMap
pub static TextRevEntitiesMap: TextRevEntitiesMapT = TextRevEntitiesMapT;

impl RevEntityMap for TextRevEntitiesMapT {
    fn lookup_rev(&self, c: u8) -> Option<&[u8]> {
        match c {
            b'<' => Some(b"&lt;"),
            b'&' => Some(b"&amp;"),
            _ => None,
        }
    }
}
