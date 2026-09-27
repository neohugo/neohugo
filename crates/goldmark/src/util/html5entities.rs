// Go: github.com/yuin/goldmark@v1.7.12/util/html5entities.go

use super::html5entities_gen::{HTML5_ENTITIES, HTML5_ENTITIES_LENGTH};

/// HTML5Entity struct represents HTML5 entitites.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HTML5Entity {
    pub name: &'static str,
    pub characters: &'static [u8],
}

// Go: util/html5entities.go:LookUpHTML5EntityByName
/// LookUpHTML5EntityByName returns (an HTML5Entity, true) if an entity named
/// given name is found, otherwise (nil, false).
///
/// The table is goldmark's generated table (2124 entries), sorted by name for
/// a binary search instead of Go's map.
pub fn lookup_html5_entity_by_name(name: &[u8]) -> Option<HTML5Entity> {
    debug_assert_eq!(HTML5_ENTITIES.len(), HTML5_ENTITIES_LENGTH);
    HTML5_ENTITIES
        .binary_search_by(|(n, _)| n.as_bytes().cmp(name))
        .ok()
        .map(|i| HTML5Entity {
            name: HTML5_ENTITIES[i].0,
            characters: HTML5_ENTITIES[i].1,
        })
}
