// Go: github.com/yuin/goldmark@v1.7.12/extension/ast/tasklist.go

use std::sync::LazyLock;

use super::custom_node;
use crate::ast::{Ast, NodeId, NodeKind, NodeType, new_node_kind};

/// A TaskCheckBox struct represents a checkbox of a task list.
#[derive(Debug, Clone, Default)]
pub struct TaskCheckBox {
    pub is_checked: bool,
}
custom_node!(TaskCheckBox, |n| vec![(
    "Checked".to_string(),
    n.is_checked.to_string()
)]);

/// KindTaskCheckBox is a NodeKind of the TaskCheckBox node.
pub static KIND_TASK_CHECK_BOX: LazyLock<NodeKind> =
    LazyLock::new(|| new_node_kind("TaskCheckBox"));

// Go: extension/ast/tasklist.go:NewTaskCheckBox
/// NewTaskCheckBox returns a new TaskCheckBox node.
pub fn new_task_check_box(ast: &mut Ast, checked: bool) -> NodeId {
    ast.new_custom_node(
        *KIND_TASK_CHECK_BOX,
        NodeType::Inline,
        Box::new(TaskCheckBox {
            is_checked: checked,
        }),
    )
}
