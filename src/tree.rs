use alloc::vec;

use crate::ancestors::Ancestors;

use super::{
    children::Children,
    decendents::Descendants,
    forward_siblings::ForwardSiblings,
    iter::{Iter, IterMut},
    orphans::Orphans,
    previous_siblings::PreviousSiblings,
    reverse_children::ReverseChildren,
    traverse::Traverse,
};

// pub struct NodeId(DefaultKey);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChildDetach {
    DetachChildren,
    KeepChildren,
    AppendToParent(NodeId),
}

impl From<bool> for ChildDetach {
    fn from(value: bool) -> Self {
        if value {
            ChildDetach::DetachChildren
        } else {
            ChildDetach::KeepChildren
        }
    }
}

impl From<NodeId> for ChildDetach {
    fn from(value: NodeId) -> Self {
        ChildDetach::AppendToParent(value)
    }
}

slotmap::new_key_type! {
  pub struct NodeId;
}

#[derive(Debug, Clone)]
pub(crate) struct TreeEntry<T> {
    pub(super) parent: Option<NodeId>,
    pub(super) prev_sibling: Option<NodeId>,
    pub(super) next_sibling: Option<NodeId>,
    pub(super) first_child: Option<NodeId>,
    pub(super) last_child: Option<NodeId>,
    pub(super) value: T,
}

#[derive(Debug, Clone)]
pub struct Tree<T> {
    nodes: slotmap::SlotMap<NodeId, TreeEntry<T>>,
}

impl<T> Default for Tree<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Tree<T> {
    pub fn new() -> Self {
        Self {
            nodes: slotmap::SlotMap::with_key(),
        }
    }

    pub fn alloc(&mut self, entry: T) -> NodeId {
        self.nodes.insert(TreeEntry {
            parent: None,
            prev_sibling: None,
            next_sibling: None,
            first_child: None,
            last_child: None,
            value: entry,
        })
    }

    pub fn alloc_with<F: FnOnce(NodeId) -> T>(&mut self, entry: F) -> NodeId {
        self.nodes.insert_with_key(|key| TreeEntry {
            parent: None,
            prev_sibling: None,
            next_sibling: None,
            first_child: None,
            last_child: None,
            value: entry(key),
        })
    }

    pub fn try_alloc_with<F: FnOnce(NodeId) -> Result<T, E>, E>(
        &mut self,
        entry: F,
    ) -> Result<NodeId, E> {
        self.nodes.try_insert_with_key(|key| {
            Ok(TreeEntry {
                parent: None,
                prev_sibling: None,
                next_sibling: None,
                first_child: None,
                last_child: None,
                value: entry(key)?,
            })
        })
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn get(&self, node: NodeId) -> Option<&T> {
        self.nodes.get(node).map(|entry| &entry.value)
    }

    pub fn get_mut(&mut self, node: NodeId) -> Option<&mut T> {
        self.nodes.get_mut(node).map(|entry| &mut entry.value)
    }

    pub fn parent(&self, node: NodeId) -> Option<NodeId> {
        let parent_id = self.nodes.get(node)?.parent?;
        Some(parent_id)
    }

    pub fn contains(&self, node: NodeId) -> bool {
        self.nodes.contains_key(node)
    }

    pub fn append(&mut self, parent: NodeId, child: NodeId) {
        self.detach(child, false);

        let old_last_child = self.nodes[parent].last_child;

        self.nodes[child].parent = Some(parent);
        self.nodes[child].prev_sibling = old_last_child;
        self.nodes[child].next_sibling = None;

        match old_last_child {
            Some(old_last) => self.nodes[old_last].next_sibling = Some(child),
            None => self.nodes[parent].first_child = Some(child),
        }

        self.nodes[parent].last_child = Some(child);
    }

    pub fn insert_after(&mut self, parent: NodeId, anchor: NodeId, child: NodeId) {
        self.detach(child, false);

        let old_next = self.nodes[anchor].next_sibling;

        self.nodes[child].parent = Some(parent);
        self.nodes[child].prev_sibling = Some(anchor);
        self.nodes[child].next_sibling = old_next;

        match old_next {
            Some(next) => self.nodes[next].prev_sibling = Some(child),
            None => self.nodes[parent].last_child = Some(child),
        }

        self.nodes[anchor].next_sibling = Some(child);
    }

    pub fn insert_before(&mut self, parent: NodeId, anchor: NodeId, child: NodeId) {
        self.detach(child, false);

        let old_prev = self.nodes[anchor].prev_sibling;

        self.nodes[child].parent = Some(parent);
        self.nodes[child].next_sibling = Some(anchor);
        self.nodes[child].prev_sibling = old_prev;

        match old_prev {
            Some(prev) => self.nodes[prev].next_sibling = Some(child),
            None => self.nodes[parent].first_child = Some(child),
        }

        self.nodes[anchor].prev_sibling = Some(child);
    }

    pub fn remove(&mut self, node: NodeId, decendents: bool) -> Option<T> {
        if !self.nodes.contains_key(node) {
            return None;
        }

        if !decendents {
            self.detach(node, ChildDetach::DetachChildren);
            return self.nodes.remove(node).map(|entry| entry.value);
        }

        self.detach(node, ChildDetach::KeepChildren);

        let mut stack = vec![node];
        let mut removed = None;
        while let Some(current) = stack.pop() {
            let entry = self.nodes.remove(current)?;

            let mut child = entry.first_child;
            while let Some(c) = child {
                child = self.nodes[c].next_sibling;
                stack.push(c);
            }

            if current == node {
                removed = Some(entry.value);
            }
        }

        removed
    }

    pub fn orphans(&self) -> Orphans<'_, T> {
        Orphans::new(self.nodes.iter())
    }

    /// Detaches a node from its parent and siblings, but does not remove it from the tree.
    pub fn detach(&mut self, node: NodeId, children: impl Into<ChildDetach>) {
        let children = children.into();
        let parent = self.nodes[node].parent;
        let prev = self.nodes[node].prev_sibling;
        let next = self.nodes[node].next_sibling;

        if let Some(prev) = prev {
            self.nodes[prev].next_sibling = next;
        } else if let Some(parent) = parent {
            self.nodes[parent].first_child = next;
        }

        if let Some(next) = next {
            self.nodes[next].prev_sibling = prev;
        } else if let Some(parent) = parent {
            self.nodes[parent].last_child = prev;
        }

        self.nodes[node].parent = None;
        self.nodes[node].prev_sibling = None;
        self.nodes[node].next_sibling = None;

        if let ChildDetach::DetachChildren = children {
            let mut child = self.nodes[node].first_child;
            while let Some(c) = child {
                self.nodes[c].parent = None;
                child = self.nodes[c].next_sibling;
            }
            self.nodes[node].first_child = None;
            self.nodes[node].last_child = None;
        } else if let ChildDetach::AppendToParent(new_parent) = children {
            let mut child = self.nodes[node].first_child;
            while let Some(c) = child {
                self.nodes[c].parent = Some(new_parent);
                child = self.nodes[c].next_sibling;
            }

            if let Some(first_child) = self.nodes[node].first_child {
                if let Some(last_child) = self.nodes[node].last_child {
                    if let Some(old_last) = self.nodes[new_parent].last_child {
                        self.nodes[old_last].next_sibling = Some(first_child);
                        self.nodes[first_child].prev_sibling = Some(old_last);
                    } else {
                        self.nodes[new_parent].first_child = Some(first_child);
                    }
                    self.nodes[last_child].next_sibling = None;
                    self.nodes[new_parent].last_child = Some(last_child);
                }
            }

            self.nodes[node].first_child = None;
            self.nodes[node].last_child = None;
        }
    }

    pub fn children<'a>(&'a self, node: NodeId) -> Children<'a, T> {
        Children::new(&self.nodes, node)
    }

    pub fn reverse_children<'a>(&'a self, node: NodeId) -> ReverseChildren<'a, T> {
        ReverseChildren::new(&self.nodes, node)
    }

    pub fn traverse<'a>(&'a self, node: NodeId) -> Traverse<'a, T> {
        Traverse::new(&self.nodes, node)
    }

    pub fn decendents<'a>(&'a self, node: NodeId) -> Descendants<'a, T> {
        Descendants::new(&self.nodes, node)
    }

    pub fn ancestors<'a>(&'a self, node: NodeId) -> Ancestors<'a, T> {
        Ancestors::new(&self.nodes, node)
    }

    pub fn forward_siblings<'a>(&'a self, node: NodeId) -> ForwardSiblings<'a, T> {
        ForwardSiblings::new(&self.nodes, node)
    }

    pub fn previous_siblings<'a>(&'a self, node: NodeId) -> PreviousSiblings<'a, T> {
        PreviousSiblings::new(&self.nodes, node)
    }

    pub fn iter(&self) -> Iter<'_, T> {
        Iter::new(self.nodes.iter())
    }

    pub fn iter_mut(&mut self) -> IterMut<'_, T> {
        IterMut::new(self.nodes.iter_mut())
    }
}

impl<R> core::ops::Index<NodeId> for Tree<R> {
    type Output = R;

    fn index(&self, index: NodeId) -> &Self::Output {
        self.get(index).unwrap()
    }
}

impl<R> core::ops::IndexMut<NodeId> for Tree<R> {
    fn index_mut(&mut self, index: NodeId) -> &mut Self::Output {
        self.get_mut(index).unwrap()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec;
    use alloc::vec::Vec;

    fn new_tree() -> Tree<&'static str> {
        Tree {
            nodes: slotmap::SlotMap::with_key(),
        }
    }

    fn children<T>(tree: &Tree<T>, parent: NodeId) -> Vec<NodeId> {
        let mut result = Vec::new();
        let mut cur = tree.nodes[parent].first_child;
        while let Some(id) = cur {
            result.push(id);
            cur = tree.nodes[id].next_sibling;
        }
        result
    }

    #[test]
    fn append_single_child() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let child = tree.alloc("child");
        tree.append(parent, child);

        assert_eq!(tree.nodes[parent].first_child, Some(child));
        assert_eq!(tree.nodes[parent].last_child, Some(child));
        assert_eq!(tree.nodes[child].parent, Some(parent));
        assert_eq!(tree.nodes[child].prev_sibling, None);
        assert_eq!(tree.nodes[child].next_sibling, None);
    }

    #[test]
    fn append_multiple_children() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        assert_eq!(children(&tree, parent), vec![a, b, c]);
        assert_eq!(tree.nodes[parent].first_child, Some(a));
        assert_eq!(tree.nodes[parent].last_child, Some(c));
        assert_eq!(tree.nodes[b].prev_sibling, Some(a));
        assert_eq!(tree.nodes[b].next_sibling, Some(c));
    }

    #[test]
    fn insert_after_middle() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let c = tree.alloc("c");
        let b = tree.alloc("b");
        tree.append(parent, a);
        tree.append(parent, c);
        tree.insert_after(parent, a, b); // a -> b -> c

        assert_eq!(children(&tree, parent), vec![a, b, c]);
        assert_eq!(tree.nodes[a].next_sibling, Some(b));
        assert_eq!(tree.nodes[b].prev_sibling, Some(a));
        assert_eq!(tree.nodes[b].next_sibling, Some(c));
        assert_eq!(tree.nodes[c].prev_sibling, Some(b));
    }

    #[test]
    fn insert_after_last() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(parent, a);
        tree.insert_after(parent, a, b);

        assert_eq!(children(&tree, parent), vec![a, b]);
        assert_eq!(tree.nodes[parent].last_child, Some(b));
    }

    #[test]
    fn insert_before_middle() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let c = tree.alloc("c");
        let b = tree.alloc("b");
        tree.append(parent, a);
        tree.append(parent, c);
        tree.insert_before(parent, c, b); // a -> b -> c

        assert_eq!(children(&tree, parent), vec![a, b, c]);
        assert_eq!(tree.nodes[b].prev_sibling, Some(a));
        assert_eq!(tree.nodes[b].next_sibling, Some(c));
    }

    #[test]
    fn insert_before_first() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let b = tree.alloc("b");
        let a = tree.alloc("a");
        tree.append(parent, b);
        tree.insert_before(parent, b, a);

        assert_eq!(children(&tree, parent), vec![a, b]);
        assert_eq!(tree.nodes[parent].first_child, Some(a));
    }

    #[test]
    fn remove_only_child() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let child = tree.alloc("child");
        tree.append(parent, child);
        let val = tree.remove(child, false);

        assert_eq!(val, Some("child"));
        assert_eq!(tree.nodes[parent].first_child, None);
        assert_eq!(tree.nodes[parent].last_child, None);
    }

    #[test]
    fn remove_middle_child() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);
        tree.remove(b, false);

        assert_eq!(children(&tree, parent), vec![a, c]);
        assert_eq!(tree.nodes[a].next_sibling, Some(c));
        assert_eq!(tree.nodes[c].prev_sibling, Some(a));
    }

    #[test]
    fn remove_first_child() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.remove(a, false);

        assert_eq!(children(&tree, parent), vec![b]);
        assert_eq!(tree.nodes[parent].first_child, Some(b));
        assert_eq!(tree.nodes[b].prev_sibling, None);
    }

    #[test]
    fn remove_last_child() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.remove(b, false);

        assert_eq!(children(&tree, parent), vec![a]);
        assert_eq!(tree.nodes[parent].last_child, Some(a));
        assert_eq!(tree.nodes[a].next_sibling, None);
    }

    #[test]
    fn remove_nonexistent_returns_none() {
        let mut tree = new_tree();
        let a = tree.alloc("a");
        tree.remove(a, false);
        assert_eq!(tree.remove(a, false), None);
    }

    // --- Detach ---

    #[test]
    fn detach_middle_child() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        tree.detach(b, false);

        assert_eq!(children(&tree, parent), vec![a, c]);
        assert_eq!(tree.nodes[parent].first_child, Some(a));
        assert_eq!(tree.nodes[parent].last_child, Some(c));
        assert_eq!(tree.nodes[a].next_sibling, Some(c));
        assert_eq!(tree.nodes[c].prev_sibling, Some(a));
        assert_eq!(tree.nodes[b].parent, None);
        assert_eq!(tree.nodes[b].prev_sibling, None);
        assert_eq!(tree.nodes[b].next_sibling, None);
    }

    #[test]
    fn detach_first_child() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(parent, a);
        tree.append(parent, b);

        tree.detach(a, false);

        assert_eq!(children(&tree, parent), vec![b]);
        assert_eq!(tree.nodes[parent].first_child, Some(b));
        assert_eq!(tree.nodes[parent].last_child, Some(b));
        assert_eq!(tree.nodes[b].prev_sibling, None);
        assert_eq!(tree.nodes[b].next_sibling, None);
    }

    #[test]
    fn detach_only_child() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let child = tree.alloc("child");
        tree.append(parent, child);

        tree.detach(child, false);

        assert_eq!(children(&tree, parent), Vec::<NodeId>::new());
        assert_eq!(tree.nodes[parent].first_child, None);
        assert_eq!(tree.nodes[parent].last_child, None);
        assert_eq!(tree.nodes[child].parent, None);
        assert_eq!(tree.nodes[child].prev_sibling, None);
        assert_eq!(tree.nodes[child].next_sibling, None);
    }

    #[test]
    fn detach_last_child() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(parent, a);
        tree.append(parent, b);

        tree.detach(b, false);

        assert_eq!(children(&tree, parent), vec![a]);
        assert_eq!(tree.nodes[parent].first_child, Some(a));
        assert_eq!(tree.nodes[parent].last_child, Some(a));
        assert_eq!(tree.nodes[a].prev_sibling, None);
        assert_eq!(tree.nodes[a].next_sibling, None);
        assert_eq!(tree.nodes[b].parent, None);
        assert_eq!(tree.nodes[b].prev_sibling, None);
        assert_eq!(tree.nodes[b].next_sibling, None);
    }

    #[test]
    fn detach_keeps_subtree_attached_to_node() {
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let parent = tree.alloc("parent");
        let child = tree.alloc("child");
        let grandchild = tree.alloc("grandchild");
        tree.append(root, parent);
        tree.append(parent, child);
        tree.append(child, grandchild);

        tree.detach(child, false);

        assert_eq!(children(&tree, parent), Vec::<NodeId>::new());
        assert_eq!(tree.nodes[child].parent, None);
        assert_eq!(tree.nodes[child].first_child, Some(grandchild));
        assert_eq!(tree.nodes[grandchild].parent, Some(child));
    }

    #[test]
    fn detach_detaches_children_when_requested() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let child = tree.alloc("child");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(parent, child);
        tree.append(child, a);
        tree.append(child, b);

        tree.detach(child, true);

        assert_eq!(children(&tree, parent), Vec::<NodeId>::new());
        assert_eq!(tree.nodes[child].parent, None);
        assert_eq!(tree.nodes[child].first_child, None);
        assert_eq!(tree.nodes[child].last_child, None);
        assert_eq!(tree.nodes[a].parent, None);
        assert_eq!(tree.nodes[b].parent, None);
        assert_eq!(tree.nodes[a].next_sibling, Some(b));
        assert_eq!(tree.nodes[b].prev_sibling, Some(a));
    }

    #[test]
    fn detach_appends_children_to_new_parent() {
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let before = tree.alloc("before");
        let child = tree.alloc("child");
        let c1 = tree.alloc("c1");
        let c2 = tree.alloc("c2");
        let target_parent = tree.alloc("target_parent");
        let existing = tree.alloc("existing");

        tree.append(root, before);
        tree.append(root, child);
        tree.append(child, c1);
        tree.append(child, c2);
        tree.append(target_parent, existing);

        tree.detach(child, target_parent);

        assert_eq!(children(&tree, root), vec![before]);
        assert_eq!(tree.nodes[child].parent, None);
        assert_eq!(tree.nodes[child].first_child, None);
        assert_eq!(tree.nodes[child].last_child, None);

        assert_eq!(children(&tree, target_parent), vec![existing, c1, c2]);
        assert_eq!(tree.nodes[c1].parent, Some(target_parent));
        assert_eq!(tree.nodes[c2].parent, Some(target_parent));
        assert_eq!(tree.nodes[existing].next_sibling, Some(c1));
        assert_eq!(tree.nodes[c1].prev_sibling, Some(existing));
        assert_eq!(tree.nodes[c2].next_sibling, None);
        assert_eq!(tree.nodes[target_parent].last_child, Some(c2));
    }

    #[test]
    fn detach_already_detached_node_is_noop() {
        let mut tree = new_tree();
        let a = tree.alloc("a");

        tree.detach(a, false);

        assert_eq!(tree.nodes[a].parent, None);
        assert_eq!(tree.nodes[a].prev_sibling, None);
        assert_eq!(tree.nodes[a].next_sibling, None);
    }

    // --- Iterators ---

    #[test]
    fn iter_yields_node_ids_and_values() {
        let mut tree = Tree::new();
        let a = tree.alloc("a");
        let b = tree.alloc("b");

        let mut iter: crate::Iter<'_, _> = tree.iter();
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next(), Some((a, &"a")));
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next(), Some((b, &"b")));
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_mut_yields_node_ids_and_mutable_values() {
        let mut tree = Tree::new();
        let a = tree.alloc(1);
        let b = tree.alloc(2);

        let mut iter: crate::IterMut<'_, _> = tree.iter_mut();
        assert_eq!(iter.len(), 2);
        for (id, value) in iter.by_ref() {
            *value += if id == a { 10 } else { 20 };
        }
        assert_eq!(iter.len(), 0);

        assert_eq!(tree[a], 11);
        assert_eq!(tree[b], 22);
    }

    // --- Ancestors ---

    #[test]
    fn ancestors_yields_node_then_parents_up_to_root() {
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let child = tree.alloc("child");
        let grandchild = tree.alloc("grandchild");
        let sibling = tree.alloc("sibling");
        tree.append(root, child);
        tree.append(child, grandchild);
        tree.append(root, sibling);

        let ids: Vec<_> = tree.ancestors(grandchild).collect();
        assert_eq!(ids, vec![grandchild, child, root]);
    }

    #[test]
    fn ancestors_of_root_contains_only_root() {
        let mut tree = new_tree();
        let root = tree.alloc("root");

        let ids: Vec<_> = tree.ancestors(root).collect();
        assert_eq!(ids, vec![root]);
    }

    #[test]
    fn ancestors_len_tracks_remaining_nodes() {
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let child = tree.alloc("child");
        let grandchild = tree.alloc("grandchild");
        tree.append(root, child);
        tree.append(child, grandchild);

        let mut iter = tree.ancestors(grandchild);
        assert_eq!(iter.len(), 3);
        assert_eq!(iter.next(), Some(grandchild));
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next(), Some(child));
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next(), Some(root));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
    }

    // --- Siblings ---

    #[test]
    fn forward_siblings_includes_node_and_following_siblings() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        let ids: Vec<_> = tree.forward_siblings(b).collect();
        assert_eq!(ids, vec![b, c]);
    }

    #[test]
    fn forward_siblings_of_only_child_yields_only_node() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        tree.append(parent, a);

        let ids: Vec<_> = tree.forward_siblings(a).collect();
        assert_eq!(ids, vec![a]);
    }

    #[test]
    fn forward_siblings_len_tracks_remaining_nodes() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        let mut iter = tree.forward_siblings(a);
        assert_eq!(iter.len(), 3);
        assert_eq!(iter.next(), Some(a));
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next(), Some(b));
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next(), Some(c));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn previous_siblings_includes_node_and_preceding_siblings_in_reverse() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        let ids: Vec<_> = tree.previous_siblings(b).collect();
        assert_eq!(ids, vec![b, a]);
    }

    #[test]
    fn previous_siblings_of_only_child_yields_only_node() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        tree.append(parent, a);

        let ids: Vec<_> = tree.previous_siblings(a).collect();
        assert_eq!(ids, vec![a]);
    }

    #[test]
    fn previous_siblings_len_tracks_remaining_nodes() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        let mut iter = tree.previous_siblings(c);
        assert_eq!(iter.len(), 3);
        assert_eq!(iter.next(), Some(c));
        assert_eq!(iter.len(), 2);
        assert_eq!(iter.next(), Some(b));
        assert_eq!(iter.len(), 1);
        assert_eq!(iter.next(), Some(a));
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
    }

    // --- Children ---

    #[test]
    fn reverse_children_yields_children_in_reverse_order() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        let ids: Vec<_> = tree.reverse_children(parent).collect();
        assert_eq!(ids, vec![c, b, a]);
    }

    #[test]
    fn reverse_children_next_back_yields_forward_order() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        let ids: Vec<_> = tree.reverse_children(parent).rev().collect();
        assert_eq!(ids, vec![a, b, c]);
    }

    #[test]
    fn reverse_children_mixed_front_and_back() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        let mut iter = tree.reverse_children(parent);
        assert_eq!(iter.next(), Some(c));
        assert_eq!(iter.next_back(), Some(a));
        assert_eq!(iter.next(), Some(b));
        assert_eq!(iter.next_back(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn reverse_children_single_item_front_back() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        tree.append(parent, a);

        let mut iter = tree.reverse_children(parent);
        assert_eq!(iter.next_back(), Some(a));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn reverse_children_len_matches_child_count() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(parent, a);
        tree.append(parent, b);

        let mut iter = tree.reverse_children(parent);
        assert_eq!(iter.len(), 2);
        iter.next();
        assert_eq!(iter.len(), 1);
        iter.next();
        assert_eq!(iter.len(), 0);
    }

    #[test]
    fn children_next_back_reverse_order() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        let ids: Vec<_> = tree.children(parent).rev().collect();
        assert_eq!(ids, vec![c, b, a]);
    }

    #[test]
    fn children_mixed_front_and_back() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        let mut iter = tree.children(parent);
        assert_eq!(iter.next(), Some(a));
        assert_eq!(iter.next_back(), Some(c));
        assert_eq!(iter.next(), Some(b));
        assert_eq!(iter.next_back(), None);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn children_single_item_front_back() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        tree.append(parent, a);

        let mut iter = tree.children(parent);
        assert_eq!(iter.next_back(), Some(a));
        assert_eq!(iter.next(), None);
    }

    // --- Descendants ---

    #[test]
    fn descendants_leaf_node() {
        let mut tree = new_tree();
        let a = tree.alloc("a");
        let ids: Vec<_> = tree.decendents(a).collect();
        assert_eq!(ids, vec![a]);
    }

    #[test]
    fn descendants_flat_children() {
        // root -> [a, b, c]
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(root, a);
        tree.append(root, b);
        tree.append(root, c);

        let ids: Vec<_> = tree.decendents(root).collect();
        assert_eq!(ids, vec![root, a, b, c]);
    }

    #[test]
    fn descendants_nested() {
        // root -> a -> [a1, a2], b
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let a1 = tree.alloc("a1");
        let a2 = tree.alloc("a2");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(a, a1);
        tree.append(a, a2);
        tree.append(root, b);

        let ids: Vec<_> = tree.decendents(root).collect();
        assert_eq!(ids, vec![root, a, a1, a2, b]);
    }

    #[test]
    fn descendants_subtree() {
        // root -> a -> [a1, a2], b  — start from `a`
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let a1 = tree.alloc("a1");
        let a2 = tree.alloc("a2");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(a, a1);
        tree.append(a, a2);
        tree.append(root, b);

        let ids: Vec<_> = tree.decendents(a).collect();
        assert_eq!(ids, vec![a, a1, a2]);
    }

    #[test]
    fn descendants_next_back_reverse_order() {
        // root -> a -> [a1, a2], b
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let a1 = tree.alloc("a1");
        let a2 = tree.alloc("a2");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(a, a1);
        tree.append(a, a2);
        tree.append(root, b);

        let ids: Vec<_> = tree.decendents(root).rev().collect();
        assert_eq!(ids, vec![root, b, a, a2, a1]);
    }

    #[test]
    fn descendants_mixed_front_and_back() {
        // root -> a -> a1, b
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let a1 = tree.alloc("a1");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(a, a1);
        tree.append(root, b);

        let mut it = tree.decendents(root);
        assert_eq!(it.next(), Some(root));
        assert_eq!(it.next_back(), Some(root));
        assert_eq!(it.next(), Some(a));
        assert_eq!(it.next_back(), Some(b));
        assert_eq!(it.next(), Some(a1));
        assert_eq!(it.next_back(), Some(a));
        assert_eq!(it.next(), None);
    }

    #[test]
    fn descendants_leaf_front_back() {
        let mut tree = new_tree();
        let a = tree.alloc("a");

        let mut it = tree.decendents(a);
        assert_eq!(it.next_back(), Some(a));
        assert_eq!(it.next(), Some(a));
        assert_eq!(it.next_back(), None);
    }

    // --- Traverse ---

    #[test]
    fn traverse_leaf_node() {
        use crate::traverse::NodeEdge;
        let mut tree = new_tree();
        let a = tree.alloc("a");
        let edges: Vec<_> = tree.traverse(a).collect();
        assert_eq!(edges, vec![NodeEdge::Start(a), NodeEdge::End(a)]);
    }

    #[test]
    fn traverse_flat_children() {
        use crate::traverse::NodeEdge;
        // root -> [a, b]
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(root, b);

        let edges: Vec<_> = tree.traverse(root).collect();
        assert_eq!(
            edges,
            vec![
                NodeEdge::Start(root),
                NodeEdge::Start(a),
                NodeEdge::End(a),
                NodeEdge::Start(b),
                NodeEdge::End(b),
                NodeEdge::End(root),
            ]
        );
    }

    #[test]
    fn traverse_nested() {
        use crate::traverse::NodeEdge;
        // root -> a -> a1, b
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let a1 = tree.alloc("a1");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(a, a1);
        tree.append(root, b);

        let edges: Vec<_> = tree.traverse(root).collect();
        assert_eq!(
            edges,
            vec![
                NodeEdge::Start(root),
                NodeEdge::Start(a),
                NodeEdge::Start(a1),
                NodeEdge::End(a1),
                NodeEdge::End(a),
                NodeEdge::Start(b),
                NodeEdge::End(b),
                NodeEdge::End(root),
            ]
        );
    }

    #[test]
    fn traverse_subtree_does_not_escape_root() {
        use crate::traverse::NodeEdge;
        // root -> a -> [a1, a2], b  — traverse from `a`, must not yield `b`
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let a1 = tree.alloc("a1");
        let a2 = tree.alloc("a2");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(a, a1);
        tree.append(a, a2);
        tree.append(root, b);

        let edges: Vec<_> = tree.traverse(a).collect();
        assert_eq!(
            edges,
            vec![
                NodeEdge::Start(a),
                NodeEdge::Start(a1),
                NodeEdge::End(a1),
                NodeEdge::Start(a2),
                NodeEdge::End(a2),
                NodeEdge::End(a),
            ]
        );
    }

    #[test]
    fn traverse_next_back_reverse_order() {
        use crate::traverse::NodeEdge;
        // root -> [a, b]
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(root, b);

        let edges: Vec<_> = tree.traverse(root).rev().collect();
        assert_eq!(
            edges,
            vec![
                NodeEdge::End(root),
                NodeEdge::End(b),
                NodeEdge::Start(b),
                NodeEdge::End(a),
                NodeEdge::Start(a),
                NodeEdge::Start(root),
            ]
        );
    }

    #[test]
    fn traverse_mixed_front_and_back() {
        use crate::traverse::NodeEdge;
        // root -> a
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        tree.append(root, a);

        let mut it = tree.traverse(root);
        assert_eq!(it.next(), Some(NodeEdge::Start(root)));
        assert_eq!(it.next_back(), Some(NodeEdge::End(root)));
        assert_eq!(it.next(), Some(NodeEdge::Start(a)));
        assert_eq!(it.next_back(), Some(NodeEdge::End(a)));
        assert_eq!(it.next(), None);
        assert_eq!(it.next_back(), None);
    }

    #[test]
    fn traverse_leaf_front_back() {
        use crate::traverse::NodeEdge;
        let mut tree = new_tree();
        let a = tree.alloc("a");

        let mut it = tree.traverse(a);
        assert_eq!(it.next_back(), Some(NodeEdge::End(a)));
        assert_eq!(it.next(), Some(NodeEdge::Start(a)));
        assert_eq!(it.next_back(), None);
    }

    #[test]
    fn traverse_deep_mixed_tree_preorder_edges() {
        use crate::traverse::NodeEdge;
        // root -> [a, b, c]
        // a -> [a1, a2], a2 -> [a21]
        // c -> [c1]
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let a1 = tree.alloc("a1");
        let a2 = tree.alloc("a2");
        let a21 = tree.alloc("a21");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        let c1 = tree.alloc("c1");

        tree.append(root, a);
        tree.append(a, a1);
        tree.append(a, a2);
        tree.append(a2, a21);
        tree.append(root, b);
        tree.append(root, c);
        tree.append(c, c1);

        let edges: Vec<_> = tree.traverse(root).collect();
        assert_eq!(
            edges,
            vec![
                NodeEdge::Start(root),
                NodeEdge::Start(a),
                NodeEdge::Start(a1),
                NodeEdge::End(a1),
                NodeEdge::Start(a2),
                NodeEdge::Start(a21),
                NodeEdge::End(a21),
                NodeEdge::End(a2),
                NodeEdge::End(a),
                NodeEdge::Start(b),
                NodeEdge::End(b),
                NodeEdge::Start(c),
                NodeEdge::Start(c1),
                NodeEdge::End(c1),
                NodeEdge::End(c),
                NodeEdge::End(root),
            ]
        );
    }

    #[test]
    fn traverse_reverse_is_exact_forward_reverse() {
        // Build a tree where reverse traversal has to cross both depth and sibling boundaries.
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let a1 = tree.alloc("a1");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        let c1 = tree.alloc("c1");

        tree.append(root, a);
        tree.append(a, a1);
        tree.append(root, b);
        tree.append(root, c);
        tree.append(c, c1);

        let forward: Vec<_> = tree.traverse(root).collect();
        let backward: Vec<_> = tree.traverse(root).rev().collect();
        let expected_backward: Vec<_> = forward.iter().copied().rev().collect();

        assert_eq!(backward, expected_backward);
    }

    #[test]
    fn traverse_subtree_reverse_does_not_escape_root() {
        use crate::traverse::NodeEdge;
        // root -> [a, b], a -> [a1, a2]. Traversing from `a` in reverse must not yield `b`.
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let a1 = tree.alloc("a1");
        let a2 = tree.alloc("a2");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(a, a1);
        tree.append(a, a2);
        tree.append(root, b);

        let edges: Vec<_> = tree.traverse(a).rev().collect();
        assert_eq!(
            edges,
            vec![
                NodeEdge::End(a),
                NodeEdge::End(a2),
                NodeEdge::Start(a2),
                NodeEdge::End(a1),
                NodeEdge::Start(a1),
                NodeEdge::Start(a),
            ]
        );
    }

    #[test]
    fn traverse_mixed_front_back_flat_three_children() {
        use crate::traverse::NodeEdge;
        // root -> [a, b, c]
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(root, a);
        tree.append(root, b);
        tree.append(root, c);

        let mut it = tree.traverse(root);
        assert_eq!(it.next(), Some(NodeEdge::Start(root)));
        assert_eq!(it.next_back(), Some(NodeEdge::End(root)));
        assert_eq!(it.next(), Some(NodeEdge::Start(a)));
        assert_eq!(it.next_back(), Some(NodeEdge::End(c)));
        assert_eq!(it.next(), Some(NodeEdge::End(a)));
        assert_eq!(it.next_back(), Some(NodeEdge::Start(c)));
        assert_eq!(it.next(), Some(NodeEdge::Start(b)));
        assert_eq!(it.next_back(), Some(NodeEdge::End(b)));
        assert_eq!(it.next(), None);
        assert_eq!(it.next_back(), None);
    }

    #[test]
    fn traverse_fused_after_exhaustion() {
        use crate::traverse::NodeEdge;
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        tree.append(root, a);

        let mut it = tree.traverse(root);
        assert_eq!(it.next_back(), Some(NodeEdge::End(root)));
        assert_eq!(it.next_back(), Some(NodeEdge::End(a)));
        assert_eq!(it.next_back(), Some(NodeEdge::Start(a)));
        assert_eq!(it.next_back(), Some(NodeEdge::Start(root)));
        assert_eq!(it.next_back(), None);

        // Once exhausted, both ends should keep yielding None.
        assert_eq!(it.next(), None);
        assert_eq!(it.next_back(), None);
        assert_eq!(it.next(), None);
    }

    // --- Basic accessors ---

    #[test]
    fn new_tree_is_empty() {
        let tree: Tree<&str> = Tree::new();
        assert!(tree.is_empty());
        assert_eq!(tree.len(), 0);
    }

    #[test]
    fn alloc_increases_len_and_is_not_empty() {
        let mut tree = new_tree();
        assert!(tree.is_empty());
        tree.alloc("a");
        assert!(!tree.is_empty());
        assert_eq!(tree.len(), 1);
        tree.alloc("b");
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn alloc_with_receives_its_own_node_id() {
        let mut tree: Tree<(NodeId, &str)> = Tree::new();
        let id = tree.alloc_with(|id| (id, "a"));
        assert_eq!(tree.get(id), Some(&(id, "a")));
    }

    #[test]
    fn try_alloc_with_propagates_ok() {
        let mut tree: Tree<i32> = Tree::new();
        let id = tree.try_alloc_with::<_, ()>(|_| Ok(42)).unwrap();
        assert_eq!(tree.get(id), Some(&42));
        assert_eq!(tree.len(), 1);
    }

    #[test]
    fn try_alloc_with_propagates_err_without_inserting() {
        let mut tree: Tree<i32> = Tree::new();
        let result = tree.try_alloc_with::<_, &str>(|_| Err("boom"));
        assert_eq!(result, Err("boom"));
        assert!(tree.is_empty());
    }

    #[test]
    fn get_and_get_mut_roundtrip() {
        let mut tree = new_tree();
        let a = tree.alloc("a");

        assert_eq!(tree.get(a), Some(&"a"));
        *tree.get_mut(a).unwrap() = "changed";
        assert_eq!(tree.get(a), Some(&"changed"));
    }

    #[test]
    fn get_after_remove_returns_none() {
        let mut tree = new_tree();
        let a = tree.alloc("a");
        tree.remove(a, false);

        assert_eq!(tree.get(a), None);
        assert_eq!(tree.get_mut(a), None);
    }

    #[test]
    fn contains_reflects_node_lifetime() {
        let mut tree = new_tree();
        let a = tree.alloc("a");
        assert!(tree.contains(a));

        tree.remove(a, false);
        assert!(!tree.contains(a));
    }

    #[test]
    fn parent_of_root_and_child() {
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let child = tree.alloc("child");
        tree.append(root, child);

        assert_eq!(tree.parent(root), None);
        assert_eq!(tree.parent(child), Some(root));
    }

    #[test]
    fn index_and_index_mut_operators() {
        let mut tree = new_tree();
        let a = tree.alloc("a");

        assert_eq!(tree[a], "a");
        tree[a] = "b";
        assert_eq!(tree[a], "b");
    }

    #[test]
    #[should_panic]
    fn index_panics_on_removed_node() {
        let mut tree = new_tree();
        let a = tree.alloc("a");
        tree.remove(a, false);

        let _ = tree[a];
    }

    // --- Moving existing nodes ---

    #[test]
    fn append_moves_node_from_previous_parent() {
        let mut tree = new_tree();
        let parent_a = tree.alloc("parent_a");
        let parent_b = tree.alloc("parent_b");
        let child = tree.alloc("child");
        let sibling = tree.alloc("sibling");

        tree.append(parent_a, child);
        tree.append(parent_a, sibling);
        tree.append(parent_b, child);

        assert_eq!(children(&tree, parent_a), vec![sibling]);
        assert_eq!(children(&tree, parent_b), vec![child]);
        assert_eq!(tree.nodes[child].parent, Some(parent_b));
    }

    #[test]
    fn insert_before_moves_node_from_previous_location() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        // Move `a` to sit right before `c`: b -> a -> c
        tree.insert_before(parent, c, a);

        assert_eq!(children(&tree, parent), vec![b, a, c]);
        assert_eq!(tree.nodes[parent].first_child, Some(b));
    }

    #[test]
    fn insert_after_moves_node_from_previous_location() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        tree.append(parent, a);
        tree.append(parent, b);
        tree.append(parent, c);

        // Move `c` to sit right after `a`: a -> c -> b
        tree.insert_after(parent, a, c);

        assert_eq!(children(&tree, parent), vec![a, c, b]);
        assert_eq!(tree.nodes[parent].last_child, Some(b));
    }

    // --- Remove with descendants ---

    #[test]
    fn remove_with_descendants_removes_whole_subtree() {
        // root -> [a, b]; a -> [a1, a2]
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let a1 = tree.alloc("a1");
        let a2 = tree.alloc("a2");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(a, a1);
        tree.append(a, a2);
        tree.append(root, b);

        let result = tree.remove(a, true);

        assert_eq!(result, Some("a"));
        assert_eq!(children(&tree, root), vec![b]);
        assert!(!tree.contains(a));
        assert!(!tree.contains(a1));
        assert!(!tree.contains(a2));
        assert!(tree.contains(b));
        assert_eq!(tree.len(), 2);
    }

    #[test]
    fn remove_with_descendants_on_leaf_removes_only_leaf() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let child = tree.alloc("child");
        tree.append(parent, child);

        tree.remove(child, true);

        assert!(!tree.contains(child));
        assert!(tree.contains(parent));
        assert_eq!(children(&tree, parent), Vec::<NodeId>::new());
    }

    #[test]
    fn remove_with_descendants_deep_tree() {
        // root -> a -> b -> c -> d
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let c = tree.alloc("c");
        let d = tree.alloc("d");
        tree.append(root, a);
        tree.append(a, b);
        tree.append(b, c);
        tree.append(c, d);

        tree.remove(a, true);

        assert!(!tree.contains(a));
        assert!(!tree.contains(b));
        assert!(!tree.contains(c));
        assert!(!tree.contains(d));
        assert_eq!(tree.len(), 1);
        assert_eq!(children(&tree, root), Vec::<NodeId>::new());
    }

    #[test]
    fn remove_nonexistent_with_descendants_is_noop() {
        let mut tree = new_tree();
        let a = tree.alloc("a");
        tree.remove(a, false);

        assert_eq!(tree.remove(a, true), None);
        assert!(tree.is_empty());
    }

    // --- Children edge cases ---

    #[test]
    fn children_of_leaf_is_empty() {
        let mut tree = new_tree();
        let a = tree.alloc("a");

        let mut iter = tree.children(a);
        assert!(iter.is_empty());
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
        assert_eq!(iter.next_back(), None);
    }

    #[test]
    fn children_len_matches_child_count() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(parent, a);
        tree.append(parent, b);

        let iter = tree.children(parent);
        assert!(!iter.is_empty());
        assert_eq!(iter.len(), 2);
    }

    // --- Ancestors edge cases ---

    #[test]
    fn ancestors_len_for_single_node() {
        let mut tree = new_tree();
        let a = tree.alloc("a");

        let iter = tree.ancestors(a);
        assert_eq!(iter.len(), 1);
    }

    // --- Iter / IterMut edge cases ---

    #[test]
    fn iter_over_empty_tree_yields_nothing() {
        let tree: Tree<&str> = Tree::new();
        let mut iter = tree.iter();
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_mut_over_empty_tree_yields_nothing() {
        let mut tree: Tree<&str> = Tree::new();
        let mut iter = tree.iter_mut();
        assert_eq!(iter.len(), 0);
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn iter_visits_all_nodes_regardless_of_tree_shape() {
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        tree.append(root, a);
        tree.append(a, b);

        let ids: Vec<_> = tree.iter().map(|(id, _)| id).collect();
        assert_eq!(ids.len(), 3);
        for expected in [root, a, b] {
            assert!(ids.contains(&expected));
        }
    }

    // --- Orphans ---

    #[test]
    fn orphans_over_empty_tree_yields_nothing() {
        let tree: Tree<&str> = Tree::new();
        assert_eq!(tree.orphans().next(), None);
    }

    #[test]
    fn orphans_yields_only_parentless_nodes() {
        let mut tree = new_tree();
        let root = tree.alloc("root");
        let a = tree.alloc("a");
        let b = tree.alloc("b");
        let detached = tree.alloc("detached");
        tree.append(root, a);
        tree.append(root, b);

        let ids: Vec<_> = tree.orphans().collect();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&root));
        assert!(ids.contains(&detached));
        assert!(!ids.contains(&a));
        assert!(!ids.contains(&b));
    }

    #[test]
    fn orphans_reflects_detach() {
        let mut tree = new_tree();
        let parent = tree.alloc("parent");
        let child = tree.alloc("child");
        tree.append(parent, child);
        assert_eq!(tree.orphans().count(), 1);

        tree.detach(child, false);
        assert_eq!(tree.orphans().count(), 2);
    }
}
