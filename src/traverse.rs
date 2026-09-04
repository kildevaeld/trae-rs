use slotmap::SlotMap;

use crate::tree::{NodeId, TreeEntry};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
/// Indicator if the node is at a start or endpoint of the tree
pub enum NodeEdge {
    /// Indicates that start of a node that has children.
    ///
    /// Yielded by `Traverse::next()` before the node’s descendants. In HTML or
    /// XML, this corresponds to an opening tag like `<div>`.
    Start(NodeId),

    /// Indicates that end of a node that has children.
    ///
    /// Yielded by `Traverse::next()` after the node’s descendants. In HTML or
    /// XML, this corresponds to a closing tag like `</div>`
    End(NodeId),
}

#[derive(Clone)]
/// An iterator of the "sides" of a node visited during a depth-first pre-order traversal,
/// where node sides are visited start to end and children are visited in insertion order.
///
/// i.e. node.start -> first child -> second child -> node.end
pub struct Traverse<'a, T> {
    arena: &'a SlotMap<NodeId, TreeEntry<T>>,
    root: NodeId,
    front: Option<NodeEdge>,
    back: Option<NodeEdge>,
}

impl<'a, T> Traverse<'a, T> {
    pub(crate) fn new(arena: &'a SlotMap<NodeId, TreeEntry<T>>, current: NodeId) -> Self {
        Self {
            arena,
            root: current,
            front: Some(NodeEdge::Start(current)),
            back: Some(NodeEdge::End(current)),
        }
    }

    /// Calculates the next node.
    fn next_of_next(&self, next: NodeEdge) -> Option<NodeEdge> {
        match next {
            NodeEdge::Start(node) => match self.arena[node].first_child {
                Some(first_child) => Some(NodeEdge::Start(first_child)),
                None => Some(NodeEdge::End(node)),
            },
            NodeEdge::End(node) => {
                if node == self.root {
                    return None;
                }
                let node = &self.arena[node];
                match node.next_sibling {
                    Some(next_sibling) => Some(NodeEdge::Start(next_sibling)),
                    // `node.parent()` here can only be `None` if the tree has
                    // been modified during iteration, but silently stoping
                    // iteration seems a more sensible behavior than panicking.
                    None => node.parent.map(NodeEdge::End),
                }
            }
        }
    }

    /// Calculates the previous node.
    fn prev_of_prev(&self, prev: NodeEdge) -> Option<NodeEdge> {
        match prev {
            NodeEdge::End(node) => match self.arena[node].last_child {
                Some(last_child) => Some(NodeEdge::End(last_child)),
                None => Some(NodeEdge::Start(node)),
            },
            NodeEdge::Start(node) => {
                if node == self.root {
                    return None;
                }
                let node = &self.arena[node];
                match node.prev_sibling {
                    Some(prev_sibling) => Some(NodeEdge::End(prev_sibling)),
                    // `node.parent()` here can only be `None` if the tree has
                    // been modified during iteration, but silently stoping
                    // iteration seems a more sensible behavior than panicking.
                    None => node.parent.map(NodeEdge::Start),
                }
            }
        }
    }
}

impl<'a, T> Iterator for Traverse<'a, T> {
    type Item = NodeEdge;

    fn next(&mut self) -> Option<NodeEdge> {
        let next = self.front?;

        if self.back == Some(next) {
            self.front = None;
            self.back = None;
        } else {
            self.front = self.next_of_next(next);
        }

        Some(next)
    }
}

impl<'a, T> DoubleEndedIterator for Traverse<'a, T> {
    fn next_back(&mut self) -> Option<Self::Item> {
        let prev = self.back?;

        if self.front == Some(prev) {
            self.front = None;
            self.back = None;
        } else {
            self.back = self.prev_of_prev(prev);
        }

        Some(prev)
    }
}
