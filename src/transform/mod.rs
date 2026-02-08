use crate::ast::{AstError, AstNode, AstResult, NodeId, NodeType, PdfAstGraph};
use crate::types::PdfValue;
use std::collections::HashMap;

pub mod builder;
pub mod operations;
pub mod validator;

pub use builder::*;
pub use operations::*;
pub use validator::*;

/// Trait for AST transformations
pub trait AstTransformer {
    /// Transform the AST
    fn transform(&self, graph: &mut PdfAstGraph) -> AstResult<TransformResult>;

    /// Get transformation description
    fn description(&self) -> &str;

    /// Check if transformation is reversible
    fn is_reversible(&self) -> bool {
        false
    }

    /// Get reverse transformation if available
    fn reverse_transform(&self) -> Option<Box<dyn AstTransformer>> {
        None
    }
}

/// Result of a transformation
#[derive(Debug, Clone)]
pub struct TransformResult {
    pub nodes_added: Vec<NodeId>,
    pub nodes_removed: Vec<NodeId>,
    pub nodes_modified: Vec<NodeId>,
    pub edges_added: usize,
    pub edges_removed: usize,
    pub metadata: HashMap<String, String>,
}

impl Default for TransformResult {
    fn default() -> Self {
        Self::new()
    }
}

impl TransformResult {
    pub fn new() -> Self {
        Self {
            nodes_added: Vec::new(),
            nodes_removed: Vec::new(),
            nodes_modified: Vec::new(),
            edges_added: 0,
            edges_removed: 0,
            metadata: HashMap::new(),
        }
    }

    pub fn with_added_node(mut self, node_id: NodeId) -> Self {
        self.nodes_added.push(node_id);
        self
    }

    pub fn with_removed_node(mut self, node_id: NodeId) -> Self {
        self.nodes_removed.push(node_id);
        self
    }

    pub fn with_modified_node(mut self, node_id: NodeId) -> Self {
        self.nodes_modified.push(node_id);
        self
    }
}

/// Extended mutation API for PdfAstGraph
impl PdfAstGraph {
    /// Transform the AST using a transformer
    pub fn transform<T: AstTransformer>(&mut self, transformer: T) -> AstResult<TransformResult> {
        transformer.transform(self)
    }

    /// Replace a node with a new one
    pub fn replace_node(&mut self, id: NodeId, new_node: AstNode) -> AstResult<AstNode> {
        if !self.contains_node(id) {
            return Err(AstError::InvalidReferenceString(format!(
                "Node {:?} not found",
                id
            )));
        }

        // Get the old node
        let old_node = self
            .get_node(id)
            .ok_or_else(|| AstError::InvalidReferenceString(format!("Node {:?} not found", id)))?
            .clone();

        // Update the node data
        if let Some(node_data) = self.get_node_mut(id) {
            node_data.node_type = new_node.node_type;
            node_data.value = new_node.value;
            node_data.metadata = new_node.metadata;
            // Keep existing children and references
        }

        Ok(old_node)
    }

    /// Insert a new node as child of parent
    pub fn insert_node(&mut self, parent: NodeId, node: AstNode) -> AstResult<NodeId> {
        if !self.contains_node(parent) {
            return Err(AstError::InvalidReferenceString(format!(
                "Parent node {:?} not found",
                parent
            )));
        }

        let new_id = self.create_node(node.node_type, node.value);

        // Set metadata
        if let Some(new_node) = self.get_node_mut(new_id) {
            new_node.metadata = node.metadata;
        }

        // Add edge from parent to new node
        self.add_edge(parent, new_id, crate::ast::EdgeType::Child);

        Ok(new_id)
    }

    /// Remove a node and its subtree
    pub fn remove_subtree(&mut self, root: NodeId) -> AstResult<Vec<AstNode>> {
        let mut removed_nodes = Vec::new();
        let mut to_remove = Vec::new();

        // Collect all nodes in subtree
        self.collect_subtree_nodes(root, &mut to_remove);

        // Remove nodes and collect them
        for node_id in to_remove {
            if let Some(node) = self.get_node(node_id).cloned() {
                removed_nodes.push(node);
                self.remove_node(node_id);

                // Remove all edges involving this node
                let all_edges = self.get_all_edges();
                for edge in all_edges {
                    if edge.from == node_id || edge.to == node_id {
                        self.remove_edge(edge.from, edge.to);
                    }
                }
            }
        }

        Ok(removed_nodes)
    }

    /// Collect all nodes in a subtree
    fn collect_subtree_nodes(&self, root: NodeId, result: &mut Vec<NodeId>) {
        if result.contains(&root) {
            return; // Avoid infinite loops
        }

        result.push(root);

        if let Some(node) = self.get_node(root) {
            for &child_id in &node.children {
                self.collect_subtree_nodes(child_id, result);
            }
        }
    }

    /// Move a subtree to a new parent
    pub fn move_subtree(&mut self, subtree_root: NodeId, new_parent: NodeId) -> AstResult<()> {
        if !self.contains_node(subtree_root) || !self.contains_node(new_parent) {
            return Err(AstError::InvalidReferenceString(
                "Invalid node reference".to_string(),
            ));
        }

        // Remove old parent-child edge
        if let Some(old_parent) = self.find_parent(subtree_root) {
            self.remove_edge(old_parent, subtree_root);
        }

        // Add new parent-child edge
        self.add_edge(new_parent, subtree_root, crate::ast::EdgeType::Child);

        Ok(())
    }

    /// Find parent of a node
    pub fn find_parent(&self, node_id: NodeId) -> Option<NodeId> {
        for edge in self.get_all_edges() {
            let (from, to, edge_type) = (edge.from, edge.to, edge.edge_type);
            if to == node_id && matches!(edge_type, crate::ast::EdgeType::Child) {
                return Some(from);
            }
        }
        None
    }

    /// Clone a subtree
    pub fn clone_subtree(&mut self, root: NodeId, new_parent: NodeId) -> AstResult<NodeId> {
        if !self.contains_node(root) || !self.contains_node(new_parent) {
            return Err(AstError::InvalidReferenceString(
                "Invalid node reference".to_string(),
            ));
        }

        let mut id_mapping = HashMap::new();
        let cloned_root = self.clone_subtree_recursive(root, &mut id_mapping)?;

        // Add cloned subtree to new parent
        self.add_edge(new_parent, cloned_root, crate::ast::EdgeType::Child);

        Ok(cloned_root)
    }

    /// Recursively clone subtree nodes
    fn clone_subtree_recursive(
        &mut self,
        node_id: NodeId,
        id_mapping: &mut HashMap<NodeId, NodeId>,
    ) -> AstResult<NodeId> {
        if let Some(&mapped_id) = id_mapping.get(&node_id) {
            return Ok(mapped_id);
        }

        let original_node = self
            .get_node(node_id)
            .ok_or_else(|| {
                AstError::InvalidReferenceString(format!("Node {:?} not found", node_id))
            })?
            .clone();

        // Create new node with same type and value
        let new_id = self.create_node(original_node.node_type.clone(), original_node.value.clone());

        // Copy metadata
        if let Some(new_node) = self.get_node_mut(new_id) {
            new_node.metadata = original_node.metadata.clone();
        }

        id_mapping.insert(node_id, new_id);

        // Clone children
        for &child_id in &original_node.children {
            let cloned_child = self.clone_subtree_recursive(child_id, id_mapping)?;
            self.add_edge(new_id, cloned_child, crate::ast::EdgeType::Child);
        }

        // Clone references
        for &ref_id in &original_node.references {
            if let Some(&cloned_ref) = id_mapping.get(&ref_id) {
                self.add_edge(new_id, cloned_ref, crate::ast::EdgeType::Reference);
            }
        }

        Ok(new_id)
    }

    /// Merge two nodes (combine their children and references)
    pub fn merge_nodes(&mut self, target: NodeId, source: NodeId) -> AstResult<()> {
        if !self.contains_node(target) || !self.contains_node(source) {
            return Err(AstError::InvalidReferenceString(
                "Invalid node reference".to_string(),
            ));
        }

        if target == source {
            return Ok(()); // Nothing to merge
        }

        // Get source node data
        let source_node = self
            .get_node(source)
            .ok_or_else(|| {
                AstError::InvalidReferenceString(format!("Source node {:?} not found", source))
            })?
            .clone();

        // Move children from source to target
        for &child_id in &source_node.children {
            self.remove_edge(source, child_id);
            self.add_edge(target, child_id, crate::ast::EdgeType::Child);
        }

        // Move references from source to target
        for &ref_id in &source_node.references {
            self.remove_edge(source, ref_id);
            self.add_edge(target, ref_id, crate::ast::EdgeType::Reference);
        }

        // Remove source node
        self.remove_subtree(source)?;

        Ok(())
    }

    /// Update node value
    pub fn update_node_value(
        &mut self,
        node_id: NodeId,
        new_value: PdfValue,
    ) -> AstResult<PdfValue> {
        let node = self.get_node_mut(node_id).ok_or_else(|| {
            AstError::InvalidReferenceString(format!("Node {:?} not found", node_id))
        })?;

        let old_value = std::mem::replace(&mut node.value, new_value);
        Ok(old_value)
    }

    /// Update node type
    pub fn update_node_type(&mut self, node_id: NodeId, new_type: NodeType) -> AstResult<NodeType> {
        let node = self.get_node_mut(node_id).ok_or_else(|| {
            AstError::InvalidReferenceString(format!("Node {:?} not found", node_id))
        })?;

        let old_type = std::mem::replace(&mut node.node_type, new_type);
        Ok(old_type)
    }

    /// Batch operations
    pub fn batch_transform<F>(&mut self, operations: F) -> AstResult<TransformResult>
    where
        F: FnOnce(&mut BatchOperations) -> AstResult<()>,
    {
        let mut batch = BatchOperations::new(self);
        operations(&mut batch)?;
        batch.execute()
    }
}

/// Batch operations for efficient multiple transformations
pub struct BatchOperations<'a> {
    graph: &'a mut PdfAstGraph,
    operations: Vec<Box<dyn BatchOperation>>,
}

impl<'a> BatchOperations<'a> {
    pub fn new(graph: &'a mut PdfAstGraph) -> Self {
        Self {
            graph,
            operations: Vec::new(),
        }
    }

    pub fn add_node(&mut self, parent: NodeId, node: AstNode) -> NodeId {
        let temp_id = NodeId(self.operations.len() + 1000000); // Temporary ID
        self.operations.push(Box::new(AddNodeOp {
            parent,
            node: Some(node),
            result_id: temp_id,
        }));
        temp_id
    }

    pub fn remove_node(&mut self, node_id: NodeId) {
        self.operations.push(Box::new(RemoveNodeOp { node_id }));
    }

    pub fn update_value(&mut self, node_id: NodeId, value: PdfValue) {
        self.operations.push(Box::new(UpdateValueOp {
            node_id,
            value: Some(value),
        }));
    }

    pub fn execute(self) -> AstResult<TransformResult> {
        let mut result = TransformResult::new();

        for operation in self.operations {
            operation.execute(self.graph, &mut result)?;
        }

        Ok(result)
    }
}

trait BatchOperation {
    fn execute(&self, graph: &mut PdfAstGraph, result: &mut TransformResult) -> AstResult<()>;
}

#[allow(dead_code)]
struct AddNodeOp {
    parent: NodeId,
    node: Option<AstNode>,
    result_id: NodeId,
}

impl BatchOperation for AddNodeOp {
    fn execute(&self, graph: &mut PdfAstGraph, result: &mut TransformResult) -> AstResult<()> {
        if let Some(node) = &self.node {
            let new_id = graph.insert_node(self.parent, node.clone())?;
            result.nodes_added.push(new_id);
            result.edges_added += 1;
        }
        Ok(())
    }
}

struct RemoveNodeOp {
    node_id: NodeId,
}

impl BatchOperation for RemoveNodeOp {
    fn execute(&self, graph: &mut PdfAstGraph, result: &mut TransformResult) -> AstResult<()> {
        let removed = graph.remove_subtree(self.node_id)?;
        for node in removed {
            result.nodes_removed.push(node.id);
        }
        Ok(())
    }
}

struct UpdateValueOp {
    node_id: NodeId,
    value: Option<PdfValue>,
}

impl BatchOperation for UpdateValueOp {
    fn execute(&self, graph: &mut PdfAstGraph, result: &mut TransformResult) -> AstResult<()> {
        if let Some(value) = &self.value {
            graph.update_node_value(self.node_id, value.clone())?;
            result.nodes_modified.push(self.node_id);
        }
        Ok(())
    }
}
