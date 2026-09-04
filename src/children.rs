use slotmap::SlotMap;

use crate::tree::{NodeId, TreeEntry};

pub struct Children<'a, T> {
    nodes: &'a slotmap::SlotMap<NodeId, TreeEntry<T>>,
    front: Option<NodeId>,
    back: Option<NodeId>,
}

impl<'a, T> Children<'a, T> {
    pub(super) fn new(nodes: &'a SlotMap<NodeId, TreeEntry<T>>, root: NodeId) -> Children<'a, T> {
        let root = &nodes[root];
        Children {
            nodes,
            front: root.first_child,
            back: root.last_child,
        }
    }
}

impl<'a, T> Children<'a, T> {
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl<'a, T> Iterator for Children<'a, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.front?;

        if self.back == Some(current) {
            self.front = None;
            self.back = None;
        } else {
            self.front = self.nodes[current].next_sibling;
        }

        Some(current)
    }
}

impl<'a, T> DoubleEndedIterator for Children<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let current = self.back?;

        if self.front == Some(current) {
            self.front = None;
            self.back = None;
        } else {
            self.back = self.nodes[current].prev_sibling;
        }

        Some(current)
    }
}

impl<'a, T> ExactSizeIterator for Children<'a, T> {
    fn len(&self) -> usize {
        let mut count = 0;
        let mut current = self.front;

        while let Some(id) = current {
            count += 1;
            current = self.nodes[id].next_sibling;
        }

        count
    }
}
