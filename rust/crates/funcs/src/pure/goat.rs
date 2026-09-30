//! `diagrams_goat` (feature `goat`): ASCII diagrams to SVG with svgbob.
//!
//! Hugo's `diagrams.Goat` uses GoAT; svgbob reads the same ASCII-art conventions (lines, arrows,
//! corners, dots, text) and draws an equivalent picture. The SVG bytes differ from Go's (an
//! allowed difference, REWRITE_PLAN.md §7.3); the geometry does not: a character cell is 8×16
//! pixels and the size is GoAT's, one cell of margin more than the text.
//!
//! The drawing is styled by a `<style>` scoped to the `svgbob` class of the group that holds
//! it, in `currentColor` (as GoAT draws), so a page's text colour applies.

use std::fmt::Write as _;

use super::Registrar;
use super::value::text;
use svgbob::{CellBuffer, Node, Settings};
use tera::{Map, Value};

/// GoAT's cell size in pixels.
const CELL_WIDTH: usize = 8;
const CELL_HEIGHT: usize = 16;

pub(super) fn register(r: &mut Registrar<'_>) {
    r.function("diagrams_goat", |kw, _| {
        let input = kw.must_get::<Value>("text")?;
        let svg = goat(&text(&input, "diagrams_goat")?).map_err(tera::Error::message)?;
        let mut map = Map::new();
        map.insert("inner".into(), Value::safe_string(&svg.inner));
        map.insert("wrapped".into(), Value::safe_string(&svg.wrapped()));
        map.insert("width".into(), Value::from(svg.width));
        map.insert("height".into(), Value::from(svg.height));
        Ok(Value::from(map))
    });
}

/// A rendered diagram.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GoatSvg {
    /// The SVG content (without the `<svg>` element).
    pub inner: String,
    /// Width and height in pixels.
    pub width: usize,
    pub height: usize,
}

impl GoatSvg {
    /// A complete `<svg>` element, as Hugo's `.Wrapped`.
    #[must_use]
    pub fn wrapped(&self) -> String {
        format!(
            "<svg xmlns='http://www.w3.org/2000/svg' version='1.1' height='{}' width='{}' \
             font-family='Menlo,Lucida Console,monospace'>\n{}</svg>\n",
            self.height, self.width, self.inner
        )
    }
}

/// The SVG of the ASCII diagram `text`.
///
/// # Errors
/// When svgbob fails on the input (it panics on some degenerate shapes).
pub fn goat(text: &str) -> Result<GoatSvg, String> {
    // Tabs as GoAT reads them: one cell each is too narrow, svgbob expects spaces.
    let text = text.replace('\t', "    ");
    let lines: Vec<&str> = text.lines().collect();
    let columns = lines.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    let width = (columns + 1) * CELL_WIDTH;
    let height = (lines.len() + 1) * CELL_HEIGHT;

    let settings = Settings {
        font_size: 14,
        font_family: "Menlo,Lucida Console,monospace".into(),
        fill_color: "currentColor".into(),
        background: "white".into(),
        stroke_color: "currentColor".into(),
        stroke_width: 2.0,
        scale: 8.0,
        include_backdrop: false,
        include_styles: true,
        include_defs: true,
    };
    let rendered = std::panic::catch_unwind(|| {
        let buffer = CellBuffer::from(text.as_str());
        let (node, _, _): (Node<()>, f32, f32) = buffer.get_node_with_size(&settings);
        node.render_to_string()
    })
    .map_err(|_| "diagrams_goat: the diagram could not be drawn".to_owned())?;
    // The children of svgbob's `<svg>` element.
    let body = rendered
        .find('>')
        .map(|start| &rendered[start + 1..])
        .and_then(|rest| rest.strip_suffix("</svg>"))
        .unwrap_or_default();
    let mut inner = String::with_capacity(body.len() + 64);
    // svgbob centres a character in its cell, GoAT on the cell's corner with an offset of one
    // cell: half a cell of offset gives the same position.
    let _ = write!(
        inner,
        "<g class='svgbob' transform='translate({},{})'>",
        CELL_WIDTH / 2,
        CELL_HEIGHT / 2
    );
    inner.push_str(body);
    inner.push_str("</g>\n");
    Ok(GoatSvg {
        inner,
        width,
        height,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_and_structure() {
        let svg = goat("+--+\n|  |-->\n+--+").unwrap();
        assert_eq!((svg.width, svg.height), (8 * 8, 4 * 16));
        assert!(svg.inner.starts_with("<g class='svgbob'"), "{}", svg.inner);
        assert!(svg.inner.ends_with("</g>\n"));
        assert!(svg.inner.contains("<style"));
        assert!(!svg.inner.contains("<svg"));
        assert!(
            svg.wrapped()
                .starts_with("<svg xmlns='http://www.w3.org/2000/svg'")
        );
    }

    #[test]
    fn empty() {
        let svg = goat("").unwrap();
        assert_eq!((svg.width, svg.height), (8, 16));
    }
}
