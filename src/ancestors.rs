use crate::NodeId;

pub struct Ancestors<'a, T> {
    arena: &'a slotmap::SlotMap<NodeId, crate::tree::TreeEntry<T>>,
    current: Option<NodeId>,
}

impl<'a, T> Ancestors<'a, T> {
    pub(crate) fn new(
        arena: &'a slotmap::SlotMap<NodeId, crate::tree::TreeEntry<T>>,
        current: NodeId,
    ) -> Self {
        Self {
            arena,
            current: Some(current),
        }
    }
}

impl<'a, T> Iterator for Ancestors<'a, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<NodeId> {
        if let Some(current) = self.current {
            let parent = self.arena.get(current).and_then(|entry| entry.parent);
            self.current = parent;
            Some(current)
        } else {
            None
        }
    }
}

impl<'a, T> ExactSizeIterator for Ancestors<'a, T> {
    fn len(&self) -> usize {
        let mut count = 0;
        let mut current = self.current;
        while let Some(node) = current {
            count += 1;
            current = self.arena.get(node).and_then(|entry| entry.parent);
        }
        count
    }
}
