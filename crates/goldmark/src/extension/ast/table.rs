// Go: github.com/yuin/goldmark@v1.7.12/extension/ast/table.go

use std::sync::LazyLock;

use super::custom_node;
use crate::ast::{Ast, NodeId, NodeKind, NodeType, new_node_kind};

/// Alignment is a text alignment of table cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Alignment {
    /// AlignLeft indicates text should be left justified.
    Left = 1,
    /// AlignRight indicates text should be right justified.
    Right = 2,
    /// AlignCenter indicates text should be centered.
    Center = 3,
    /// AlignNone indicates text should be aligned by default manner.
    None = 4,
}

impl Alignment {
    // Go: extension/ast/table.go:Alignment.String
    /// The Go `String()` value: "left", "right", "center", "none".
    pub fn as_str(self) -> &'static str {
        match self {
            Alignment::Left => "left",
            Alignment::Right => "right",
            Alignment::Center => "center",
            Alignment::None => "none",
        }
    }
}

impl std::fmt::Display for Alignment {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

fn alignments_dump(a: &[Alignment]) -> String {
    let v: Vec<&str> = a.iter().map(|x| x.as_str()).collect();
    format!("[{}]", v.join(" "))
}

/// A Table struct represents a table of Markdown(GFM) text.
#[derive(Debug, Clone, Default)]
pub struct Table {
    /// Alignments returns alignments of the columns.
    pub alignments: Vec<Alignment>,
}
custom_node!(Table, |n| vec![(
    "Alignments".to_string(),
    alignments_dump(&n.alignments)
)]);

/// KindTable is a NodeKind of the Table node.
pub static KIND_TABLE: LazyLock<NodeKind> = LazyLock::new(|| new_node_kind("Table"));

// Go: extension/ast/table.go:NewTable
/// NewTable returns a new Table node.
pub fn new_table(ast: &mut Ast) -> NodeId {
    ast.new_custom_node(
        *KIND_TABLE,
        NodeType::Block,
        Box::new(Table {
            alignments: Vec::new(),
        }),
    )
}

/// A TableRow struct represents a table row of Markdown(GFM) text.
#[derive(Debug, Clone, Default)]
pub struct TableRow {
    pub alignments: Vec<Alignment>,
}
custom_node!(TableRow);

/// KindTableRow is a NodeKind of the TableRow node.
pub static KIND_TABLE_ROW: LazyLock<NodeKind> = LazyLock::new(|| new_node_kind("TableRow"));

// Go: extension/ast/table.go:NewTableRow
/// NewTableRow returns a new TableRow node.
pub fn new_table_row(ast: &mut Ast, alignments: Vec<Alignment>) -> NodeId {
    ast.new_custom_node(
        *KIND_TABLE_ROW,
        NodeType::Block,
        Box::new(TableRow { alignments }),
    )
}

/// A TableHeader struct represents a table header of Markdown(GFM) text.
#[derive(Debug, Clone, Default)]
pub struct TableHeader {
    /// Go leaves this nil (NewTableHeader never sets it).
    pub alignments: Vec<Alignment>,
}
custom_node!(TableHeader);

/// KindTableHeader is a NodeKind of the TableHeader node.
pub static KIND_TABLE_HEADER: LazyLock<NodeKind> = LazyLock::new(|| new_node_kind("TableHeader"));

// Go: extension/ast/table.go:NewTableHeader
/// NewTableHeader returns a new TableHeader node (moving the row's cells).
pub fn new_table_header(ast: &mut Ast, row: NodeId) -> NodeId {
    let n = ast.new_custom_node(
        *KIND_TABLE_HEADER,
        NodeType::Block,
        Box::new(TableHeader::default()),
    );
    let mut c = ast.first_child(row);
    while let Some(cid) = c {
        let next = ast.next_sibling(cid);
        ast.append_child(n, cid);
        c = next;
    }
    n
}

/// A TableCell struct represents a table cell of a Markdown(GFM) text.
#[derive(Debug, Clone)]
pub struct TableCell {
    pub alignment: Alignment,
}
custom_node!(TableCell);

/// KindTableCell is a NodeKind of the TableCell node.
pub static KIND_TABLE_CELL: LazyLock<NodeKind> = LazyLock::new(|| new_node_kind("TableCell"));

// Go: extension/ast/table.go:NewTableCell
/// NewTableCell returns a new TableCell node.
pub fn new_table_cell(ast: &mut Ast) -> NodeId {
    ast.new_custom_node(
        *KIND_TABLE_CELL,
        NodeType::Block,
        Box::new(TableCell {
            alignment: Alignment::None,
        }),
    )
}

/// The Table payload of `n`, if it is a Table.
pub fn table(ast: &Ast, n: NodeId) -> Option<&Table> {
    ast.custom::<Table>(n)
}

/// The TableRow payload of `n`, if it is a TableRow.
pub fn table_row(ast: &Ast, n: NodeId) -> Option<&TableRow> {
    ast.custom::<TableRow>(n)
}

/// The TableCell payload of `n`, if it is a TableCell.
pub fn table_cell(ast: &Ast, n: NodeId) -> Option<&TableCell> {
    ast.custom::<TableCell>(n)
}
