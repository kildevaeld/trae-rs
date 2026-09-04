use crate::tree::{NodeId, TreeEntry};

/// An iterator over a node's following siblings, including the node itself.
pub struct ForwardSiblings<'a, T> {
    nodes: &'a slotmap::SlotMap<NodeId, TreeEntry<T>>,
    current: Option<NodeId>,
}

impl<'a, T> ForwardSiblings<'a, T> {
    pub(crate) fn new(nodes: &'a slotmap::SlotMap<NodeId, TreeEntry<T>>, node: NodeId) -> Self {
        Self {
            nodes,
            current: Some(node),
        }
    }
}

impl<'a, T> Iterator for ForwardSiblings<'a, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        self.current = self.nodes[current].next_sibling;
        Some(current)
    }
}

impl<'a, T> ExactSizeIterator for ForwardSiblings<'a, T> {
    fn len(&self) -> usize {
        let mut count = 0;
        let mut current = self.current;
        while let Some(id) = current {
            count += 1;
            current = self.nodes[id].next_sibling;
        }
        count
    }
}
