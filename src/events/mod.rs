use crate::ast::{DocumentRevision, EdgeType, NodeId, NodeType};

/// Listener for parser/AST events. Default methods are no-ops.
pub trait AstEventListener {
    fn on_node_added(&mut self, _node_id: NodeId, _node_type: &NodeType) {}
    fn on_edge_added(&mut self, _from: NodeId, _to: NodeId, _edge_type: EdgeType) {}
    fn on_incremental_applied(&mut self, _revision: &DocumentRevision) {}
}
