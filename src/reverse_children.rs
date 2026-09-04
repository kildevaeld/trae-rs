use slotmap::SlotMap;

use crate::tree::{NodeId, TreeEntry};

/// An iterator over a node's children in reverse insertion order.
pub struct ReverseChildren<'a, T> {
    nodes: &'a SlotMap<NodeId, TreeEntry<T>>,
    front: Option<NodeId>,
    back: Option<NodeId>,
}

impl<'a, T> ReverseChildren<'a, T> {
    pub(super) fn new(
        nodes: &'a SlotMap<NodeId, TreeEntry<T>>,
        root: NodeId,
    ) -> ReverseChildren<'a, T> {
        let root = &nodes[root];
        ReverseChildren {
            nodes,
            front: root.last_child,
            back: root.first_child,
        }
    }
}

impl<'a, T> ReverseChildren<'a, T> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, T> Iterator for ReverseChildren<'a, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.front?;

        if self.back == Some(current) {
            self.front = None;
            self.back = None;
        } else {
            self.front = self.nodes[current].prev_sibling;
        }

        Some(current)
    }
}

impl<'a, T> DoubleEndedIterator for ReverseChildren<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let current = self.back?;

        if self.front == Some(current) {
            self.front = None;
            self.back = None;
        } else {
            self.back = self.nodes[current].next_sibling;
        }

        Some(current)
    }
}

impl<'a, T> ExactSizeIterator for ReverseChildren<'a, T> {
    fn len(&self) -> usize {
        let mut count = 0;
        let mut current = self.front;

        while let Some(id) = current {
            count += 1;
            current = self.nodes[id].prev_sibling;
        }

        count
    }
}
