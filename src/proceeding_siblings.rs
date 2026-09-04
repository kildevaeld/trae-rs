use crate::tree::{NodeId, TreeEntry};

/// An iterator over a node's proceeding siblings, including the node itself,
/// in reverse order (nearest sibling first).
pub struct ProceedingSiblings<'a, T> {
    nodes: &'a slotmap::SlotMap<NodeId, TreeEntry<T>>,
    current: Option<NodeId>,
}

impl<'a, T> ProceedingSiblings<'a, T> {
    pub(crate) fn new(nodes: &'a slotmap::SlotMap<NodeId, TreeEntry<T>>, node: NodeId) -> Self {
        Self {
            nodes,
            current: Some(node),
        }
    }
}

impl<'a, T> Iterator for ProceedingSiblings<'a, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current?;
        self.current = self.nodes[current].prev_sibling;
        Some(current)
    }
}

impl<'a, T> ExactSizeIterator for ProceedingSiblings<'a, T> {
    fn len(&self) -> usize {
        let mut count = 0;
        let mut current = self.current;
        while let Some(id) = current {
            count += 1;
            current = self.nodes[id].prev_sibling;
        }
        count
    }
}
