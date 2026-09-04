use slotmap::SlotMap;

use crate::{
    traverse::{NodeEdge, Traverse},
    tree::{NodeId, TreeEntry},
};

#[derive(Clone)]
/// An iterator of the IDs of a given node and its descendants, as a pre-order depth-first search where children are visited in insertion order.
///
/// i.e. node -> first child -> second child
pub struct Descendants<'a, T>(Traverse<'a, T>);

impl<'a, T> Descendants<'a, T> {
    pub(crate) fn new(arena: &'a SlotMap<NodeId, TreeEntry<T>>, current: NodeId) -> Self {
        Self(Traverse::new(arena, current))
    }
}

impl<'a, T> Iterator for Descendants<'a, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        while let Some(edge) = self.0.next() {
            if let NodeEdge::Start(node) = edge {
                return Some(node);
            }
        }
        None
    }
}

impl<'a, T> DoubleEndedIterator for Descendants<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        while let Some(edge) = self.0.next_back() {
            if let NodeEdge::End(node) = edge {
                return Some(node);
            }
        }
        None
    }
}
