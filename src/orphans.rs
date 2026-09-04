use core::iter::FusedIterator;

use slotmap::basic::Iter as SlotMapIter;

use crate::tree::{NodeId, TreeEntry};

/// An iterator over the ids of nodes that currently have no parent.
pub struct Orphans<'a, T> {
    inner: SlotMapIter<'a, NodeId, TreeEntry<T>>,
}

impl<'a, T> Orphans<'a, T> {
    pub(crate) fn new(inner: SlotMapIter<'a, NodeId, TreeEntry<T>>) -> Self {
        Self { inner }
    }
}

impl<T> Clone for Orphans<'_, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<T> Iterator for Orphans<'_, T> {
    type Item = NodeId;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let (id, entry) = self.inner.next()?;
            if entry.parent.is_none() {
                return Some(id);
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (0, self.inner.size_hint().1)
    }
}

impl<T> FusedIterator for Orphans<'_, T> {}
