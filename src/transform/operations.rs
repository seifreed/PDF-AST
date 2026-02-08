use super::*;
use crate::ast::{AstNode, NodeId, PdfAstGraph};
use crate::types::PdfValue;

/// Transform operation that can be applied to AST nodes
#[derive(Debug, Clone)]
pub enum TransformOperation {
    /// Replace a node with a new node
    ReplaceNode { target: NodeId, new_node: AstNode },
    /// Insert a new node as a child
    InsertChild {
        parent: NodeId,
        child: AstNode,
        position: Option<usize>,
    },
    /// Remove a node
    RemoveNode {
        target: NodeId,
        preserve_children: bool,
    },
    /// Move a node to a new parent
    MoveNode {
        target: NodeId,
        new_parent: NodeId,
        position: Option<usize>,
    },
    /// Update node value
    UpdateValue { target: NodeId, new_value: PdfValue },
    /// Batch operation containing multiple operations
    Batch(Vec<TransformOperation>),
}

impl TransformOperation {
    /// Apply this operation to the graph
    pub fn apply(&self, graph: &mut PdfAstGraph) -> AstResult<()> {
        match self {
            TransformOperation::ReplaceNode { target, new_node } => {
                graph.replace_node(*target, new_node.clone())?;
            }
            TransformOperation::InsertChild {
                parent,
                child,
                position,
            } => {
                let child_id = graph.create_node(child.node_type.clone(), child.value.clone());
                graph.add_edge(*parent, child_id, crate::ast::EdgeType::Child);

                // TODO: Handle position parameter for ordered insertion
                let _ = position;
            }
            TransformOperation::RemoveNode {
                target,
                preserve_children,
            } => {
                if *preserve_children {
                    // Move children to parent before removing
                    let children = graph.get_children(*target);
                    if let Some(parent_id) = graph.get_parent(*target) {
                        for child_id in children {
                            graph.remove_edge(*target, child_id);
                            graph.add_edge(parent_id, child_id, crate::ast::EdgeType::Child);
                        }
                    }
                }
                graph.remove_node(*target);
            }
            TransformOperation::MoveNode {
                target,
                new_parent,
                position,
            } => {
                // Remove from current parent
                if let Some(old_parent) = graph.get_parent(*target) {
                    graph.remove_edge(old_parent, *target);
                }

                // Add to new parent
                graph.add_edge(*new_parent, *target, crate::ast::EdgeType::Child);

                // TODO: Handle position parameter
                let _ = position;
            }
            TransformOperation::UpdateValue { target, new_value } => {
                if let Some(node) = graph.get_node_mut(*target) {
                    node.value = new_value.clone();
                } else {
                    return Err(AstError::NodeNotFound(format!("Node {:?}", target)));
                }
            }
            TransformOperation::Batch(operations) => {
                for operation in operations {
                    operation.apply(graph)?;
                }
            }
        }
        Ok(())
    }

    /// Create a replace operation
    pub fn replace(target: NodeId, new_node: AstNode) -> Self {
        TransformOperation::ReplaceNode { target, new_node }
    }

    /// Create an insert operation
    pub fn insert(parent: NodeId, child: AstNode) -> Self {
        TransformOperation::InsertChild {
            parent,
            child,
            position: None,
        }
    }

    /// Create an insert operation at specific position
    pub fn insert_at(parent: NodeId, child: AstNode, position: usize) -> Self {
        TransformOperation::InsertChild {
            parent,
            child,
            position: Some(position),
        }
    }

    /// Create a remove operation
    pub fn remove(target: NodeId) -> Self {
        TransformOperation::RemoveNode {
            target,
            preserve_children: false,
        }
    }

    /// Create a remove operation that preserves children
    pub fn remove_preserve_children(target: NodeId) -> Self {
        TransformOperation::RemoveNode {
            target,
            preserve_children: true,
        }
    }

    /// Create a move operation
    pub fn move_node(target: NodeId, new_parent: NodeId) -> Self {
        TransformOperation::MoveNode {
            target,
            new_parent,
            position: None,
        }
    }

    /// Create a move operation to specific position
    pub fn move_to_position(target: NodeId, new_parent: NodeId, position: usize) -> Self {
        TransformOperation::MoveNode {
            target,
            new_parent,
            position: Some(position),
        }
    }

    /// Create an update value operation
    pub fn update_value(target: NodeId, new_value: PdfValue) -> Self {
        TransformOperation::UpdateValue { target, new_value }
    }

    /// Create a batch operation
    pub fn batch(operations: Vec<TransformOperation>) -> Self {
        TransformOperation::Batch(operations)
    }
}
