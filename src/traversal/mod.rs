use crate::ast::{AstNode, PdfAstGraph, PdfDocument};
use crate::visitor::{AstWalker as VisitorAstWalker, Visitor};

/// Trait for walking AST nodes in a document graph.
pub trait AstWalker {
    /// Walk nodes in depth-first order from the graph root.
    fn walk_nodes<V: Visitor>(&self, visitor: &mut V);

    /// Walk nodes with a lightweight callback.
    fn walk_nodes_with<F>(&self, f: F)
    where
        F: FnMut(&AstNode);
}

/// Trait for walking the graph structure (nodes + edges).
pub trait GraphWalker {
    /// Walk nodes by iterating over all nodes in the graph.
    fn walk_all_nodes<F>(&self, f: F)
    where
        F: FnMut(&AstNode);

    /// Walk edges by iterating over all edges in the graph.
    fn walk_edges<F>(&self, f: F)
    where
        F: FnMut(&crate::ast::EdgeInfo);
}

/// Trait for iterating incremental timeline steps.
pub trait TimelineWalker {
    /// Walk document revisions in order.
    fn walk_revisions<F>(&self, f: F)
    where
        F: FnMut(&crate::ast::DocumentRevision);
}

impl AstWalker for PdfAstGraph {
    fn walk_nodes<V: Visitor>(&self, visitor: &mut V) {
        let mut walker = VisitorAstWalker::new(self);
        walker.walk(visitor);
    }

    fn walk_nodes_with<F>(&self, mut f: F)
    where
        F: FnMut(&AstNode),
    {
        self.walk_nodes(&mut CallbackVisitor { callback: &mut f });
    }
}

impl GraphWalker for PdfAstGraph {
    fn walk_all_nodes<F>(&self, mut f: F)
    where
        F: FnMut(&AstNode),
    {
        for node in self.get_all_nodes() {
            f(node);
        }
    }

    fn walk_edges<F>(&self, mut f: F)
    where
        F: FnMut(&crate::ast::EdgeInfo),
    {
        for edge in self.get_all_edges() {
            f(&edge);
        }
    }
}

impl TimelineWalker for PdfDocument {
    fn walk_revisions<F>(&self, mut f: F)
    where
        F: FnMut(&crate::ast::DocumentRevision),
    {
        for revision in &self.revisions {
            f(revision);
        }
    }
}

struct CallbackVisitor<'a, F> {
    callback: &'a mut F,
}

impl<'a, F> Visitor for CallbackVisitor<'a, F>
where
    F: FnMut(&AstNode),
{
    fn visit_node(&mut self, node: &AstNode) -> crate::visitor::VisitorAction {
        (self.callback)(node);
        crate::visitor::VisitorAction::Continue
    }
}
