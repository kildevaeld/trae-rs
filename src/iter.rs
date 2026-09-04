use core::iter::FusedIterator;

use slotmap::basic::{Iter as SlotMapIter, IterMut as SlotMapIterMut};

use crate::tree::{NodeId, TreeEntry};

/// An iterator over the node IDs and values in a tree.
pub struct Iter<'a, T> {
    inner: SlotMapIter<'a, NodeId, TreeEntry<T>>,
}

impl<'a, T> Iter<'a, T> {
    pub(crate) fn new(inner: SlotMapIter<'a, NodeId, TreeEntry<T>>) -> Self {
        Self { inner }
    }
}

impl<T> Clone for Iter<'_, T> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
        }
    }
}

impl<'a, T> Iterator for Iter<'a, T> {
    type Item = (NodeId, &'a T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(id, entry)| (id, &entry.value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for Iter<'_, T> {}
impl<T> FusedIterator for Iter<'_, T> {}

/// A mutable iterator over the node IDs and values in a tree.
pub struct IterMut<'a, T> {
    inner: SlotMapIterMut<'a, NodeId, TreeEntry<T>>,
}

impl<'a, T> IterMut<'a, T> {
    pub(crate) fn new(inner: SlotMapIterMut<'a, NodeId, TreeEntry<T>>) -> Self {
        Self { inner }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = (NodeId, &'a mut T);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|(id, entry)| (id, &mut entry.value))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

impl<T> ExactSizeIterator for IterMut<'_, T> {}
impl<T> FusedIterator for IterMut<'_, T> {}
