use crate::ast::{AstNode, NodeId, NodeType, PdfAstGraph};
use crate::types::PdfValue;

/// Validates transformations before they are applied
pub struct TransformValidator {
    strict_mode: bool,
    preserve_structure: bool,
}

/// Validation result for a transformation
#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<ValidationError>,
    pub warnings: Vec<ValidationWarning>,
}

/// Validation error for a transformation
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub code: String,
    pub message: String,
    pub node_id: Option<NodeId>,
}

/// Validation warning for a transformation
#[derive(Debug, Clone)]
pub struct ValidationWarning {
    pub code: String,
    pub message: String,
    pub node_id: Option<NodeId>,
}

impl TransformValidator {
    /// Create a new validator
    pub fn new() -> Self {
        Self {
            strict_mode: false,
            preserve_structure: true,
        }
    }

    /// Create a strict validator
    pub fn strict() -> Self {
        Self {
            strict_mode: true,
            preserve_structure: true,
        }
    }

    /// Create a permissive validator
    pub fn permissive() -> Self {
        Self {
            strict_mode: false,
            preserve_structure: false,
        }
    }

    /// Enable strict mode
    pub fn with_strict_mode(mut self, strict: bool) -> Self {
        self.strict_mode = strict;
        self
    }

    /// Enable structure preservation
    pub fn with_structure_preservation(mut self, preserve: bool) -> Self {
        self.preserve_structure = preserve;
        self
    }

    /// Validate a transformation operation
    pub fn validate_operation(
        &self,
        operation: &super::operations::TransformOperation,
        graph: &PdfAstGraph,
    ) -> ValidationResult {
        let mut result = ValidationResult {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
        };

        match operation {
            super::operations::TransformOperation::ReplaceNode { target, new_node } => {
                self.validate_replace(&mut result, *target, new_node, graph);
            }
            super::operations::TransformOperation::InsertChild { parent, child, .. } => {
                self.validate_insert(&mut result, *parent, child, graph);
            }
            super::operations::TransformOperation::RemoveNode {
                target,
                preserve_children,
            } => {
                self.validate_remove(&mut result, *target, *preserve_children, graph);
            }
            super::operations::TransformOperation::MoveNode {
                target, new_parent, ..
            } => {
                self.validate_move(&mut result, *target, *new_parent, graph);
            }
            super::operations::TransformOperation::UpdateValue { target, new_value } => {
                self.validate_update(&mut result, *target, new_value, graph);
            }
            super::operations::TransformOperation::Batch(operations) => {
                for op in operations {
                    let op_result = self.validate_operation(op, graph);
                    result.errors.extend(op_result.errors);
                    result.warnings.extend(op_result.warnings);
                    if !op_result.is_valid {
                        result.is_valid = false;
                    }
                }
            }
        }

        result
    }

    fn validate_replace(
        &self,
        result: &mut ValidationResult,
        target: NodeId,
        new_node: &AstNode,
        graph: &PdfAstGraph,
    ) {
        // Check if target node exists
        if graph.get_node(target).is_none() {
            result.errors.push(ValidationError {
                code: "REPLACE_TARGET_NOT_FOUND".to_string(),
                message: format!("Target node {} not found", target.0),
                node_id: Some(target),
            });
            result.is_valid = false;
            return;
        }

        let old_node = graph.get_node(target).unwrap();

        // In strict mode, check type compatibility
        if self.strict_mode && old_node.node_type != new_node.node_type {
            result.errors.push(ValidationError {
                code: "REPLACE_TYPE_MISMATCH".to_string(),
                message: format!(
                    "Cannot replace {:?} with {:?} in strict mode",
                    old_node.node_type, new_node.node_type
                ),
                node_id: Some(target),
            });
            result.is_valid = false;
        }

        // Check if replacing root node
        if graph.get_root() == Some(target) && self.preserve_structure {
            result.warnings.push(ValidationWarning {
                code: "REPLACE_ROOT_NODE".to_string(),
                message: "Replacing root node may affect document structure".to_string(),
                node_id: Some(target),
            });
        }
    }

    fn validate_insert(
        &self,
        result: &mut ValidationResult,
        parent: NodeId,
        child: &AstNode,
        graph: &PdfAstGraph,
    ) {
        // Check if parent exists
        if graph.get_node(parent).is_none() {
            result.errors.push(ValidationError {
                code: "INSERT_PARENT_NOT_FOUND".to_string(),
                message: format!("Parent node {} not found", parent.0),
                node_id: Some(parent),
            });
            result.is_valid = false;
            return;
        }

        let parent_node = graph.get_node(parent).unwrap();

        // Check parent-child compatibility
        if self.strict_mode
            && !self.is_valid_parent_child_relationship(&parent_node.node_type, &child.node_type)
        {
            result.errors.push(ValidationError {
                code: "INSERT_INVALID_RELATIONSHIP".to_string(),
                message: format!(
                    "Invalid parent-child relationship: {:?} -> {:?}",
                    parent_node.node_type, child.node_type
                ),
                node_id: Some(parent),
            });
            result.is_valid = false;
        }
    }

    fn validate_remove(
        &self,
        result: &mut ValidationResult,
        target: NodeId,
        preserve_children: bool,
        graph: &PdfAstGraph,
    ) {
        // Check if target exists
        if graph.get_node(target).is_none() {
            result.errors.push(ValidationError {
                code: "REMOVE_TARGET_NOT_FOUND".to_string(),
                message: format!("Target node {} not found", target.0),
                node_id: Some(target),
            });
            result.is_valid = false;
            return;
        }

        // Check if removing root node
        if graph.get_root() == Some(target) {
            result.errors.push(ValidationError {
                code: "REMOVE_ROOT_NODE".to_string(),
                message: "Cannot remove root node".to_string(),
                node_id: Some(target),
            });
            result.is_valid = false;
        }

        // Check children
        let children = graph.get_children(target);
        if !children.is_empty() && !preserve_children {
            result.warnings.push(ValidationWarning {
                code: "REMOVE_WITH_CHILDREN".to_string(),
                message: format!("Removing node with {} children", children.len()),
                node_id: Some(target),
            });
        }
    }

    fn validate_move(
        &self,
        result: &mut ValidationResult,
        target: NodeId,
        new_parent: NodeId,
        graph: &PdfAstGraph,
    ) {
        // Check if both nodes exist
        if graph.get_node(target).is_none() {
            result.errors.push(ValidationError {
                code: "MOVE_TARGET_NOT_FOUND".to_string(),
                message: format!("Target node {} not found", target.0),
                node_id: Some(target),
            });
            result.is_valid = false;
            return;
        }

        if graph.get_node(new_parent).is_none() {
            result.errors.push(ValidationError {
                code: "MOVE_PARENT_NOT_FOUND".to_string(),
                message: format!("New parent node {} not found", new_parent.0),
                node_id: Some(new_parent),
            });
            result.is_valid = false;
            return;
        }

        // Check for circular reference
        if self.would_create_cycle(target, new_parent, graph) {
            result.errors.push(ValidationError {
                code: "MOVE_CREATES_CYCLE".to_string(),
                message: "Move operation would create a cycle".to_string(),
                node_id: Some(target),
            });
            result.is_valid = false;
        }

        // Check parent-child compatibility in strict mode
        if self.strict_mode {
            let target_node = graph.get_node(target).unwrap();
            let parent_node = graph.get_node(new_parent).unwrap();

            if !self
                .is_valid_parent_child_relationship(&parent_node.node_type, &target_node.node_type)
            {
                result.errors.push(ValidationError {
                    code: "MOVE_INVALID_RELATIONSHIP".to_string(),
                    message: format!(
                        "Invalid parent-child relationship: {:?} -> {:?}",
                        parent_node.node_type, target_node.node_type
                    ),
                    node_id: Some(target),
                });
                result.is_valid = false;
            }
        }
    }

    fn validate_update(
        &self,
        result: &mut ValidationResult,
        target: NodeId,
        new_value: &PdfValue,
        graph: &PdfAstGraph,
    ) {
        // Check if target exists
        if graph.get_node(target).is_none() {
            result.errors.push(ValidationError {
                code: "UPDATE_TARGET_NOT_FOUND".to_string(),
                message: format!("Target node {} not found", target.0),
                node_id: Some(target),
            });
            result.is_valid = false;
            return;
        }

        let node = graph.get_node(target).unwrap();

        // In strict mode, validate value compatibility with node type
        if self.strict_mode && !self.is_valid_value_for_type(&node.node_type, new_value) {
            result.warnings.push(ValidationWarning {
                code: "UPDATE_TYPE_MISMATCH".to_string(),
                message: format!(
                    "Value type may not be compatible with node type {:?}",
                    node.node_type
                ),
                node_id: Some(target),
            });
        }
    }

    fn is_valid_parent_child_relationship(
        &self,
        parent_type: &NodeType,
        child_type: &NodeType,
    ) -> bool {
        matches!(
            (parent_type, child_type),
            (NodeType::Catalog, NodeType::Pages)
                | (NodeType::Catalog, NodeType::Outline)
                | (NodeType::Catalog, NodeType::Metadata)
                | (NodeType::Pages, NodeType::Page)
                | (NodeType::Pages, NodeType::Pages)
                | (NodeType::Page, NodeType::ContentStream)
                | (NodeType::Page, NodeType::Annotation)
                | (NodeType::Page, NodeType::XObject)
                | (NodeType::Page, NodeType::Font)
        )
    }

    fn is_valid_value_for_type(&self, node_type: &NodeType, value: &PdfValue) -> bool {
        match (node_type, value) {
            (NodeType::Catalog, PdfValue::Dictionary(_)) => true,
            (NodeType::Pages, PdfValue::Dictionary(_)) => true,
            (NodeType::Page, PdfValue::Dictionary(_)) => true,
            (NodeType::ContentStream, PdfValue::Stream(_)) => true,
            (NodeType::Font, PdfValue::Dictionary(_)) => true,
            _ => true, // Allow other combinations in permissive mode
        }
    }

    fn would_create_cycle(&self, target: NodeId, new_parent: NodeId, graph: &PdfAstGraph) -> bool {
        // Check if new_parent is a descendant of target
        let mut to_check = vec![target];
        let mut visited = std::collections::HashSet::new();

        while let Some(current) = to_check.pop() {
            if visited.contains(&current) {
                continue;
            }
            visited.insert(current);

            if current == new_parent {
                return true;
            }

            let children = graph.get_children(current);
            to_check.extend(children);
        }

        false
    }
}

impl Default for TransformValidator {
    fn default() -> Self {
        Self::new()
    }
}
