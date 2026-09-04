# trae-rs

An arena-allocated tree data structure for Rust, `#![no_std]` (uses `alloc`).

Nodes are stored in a [`slotmap::SlotMap`](https://docs.rs/slotmap) and addressed
by a generational `NodeId` key rather than by pointer or reference. This means:

- No lifetime-based parent/child borrowing issues — a `Tree<T>` owns all of its
  nodes directly.
- Removing a node is safe against dangling references: a stale `NodeId` simply
  fails to look up rather than causing undefined behavior.

This follows the same design family as crates like
[`indextree`](https://crates.io/crates/indextree).

## Usage

```rust
use trae_rs::Tree;

let mut tree = Tree::new();

let root = tree.alloc("root");
let child1 = tree.alloc("child1");
let child2 = tree.alloc("child2");

tree.append(root, child1);
tree.append(root, child2);

for value in tree.children(root) {
    println!("{value}");
}
```

## Features

- **Mutation**: `alloc`/`alloc_with`/`try_alloc_with`, `append`,
  `insert_before`/`insert_after`, `detach`, `remove`.
- **Detach modes**: `detach` takes a `children: impl Into<ChildDetach>`
  parameter, so children can be kept in place (`false` / `KeepChildren`),
  detached along with their parent (`true` / `DetachChildren`), or reparented
  to another node (`NodeId` / `AppendToParent`).
- **Traversal**: double-ended iterators over `children`, `ancestors`,
  `decendents`, and a full pre/post-order `traverse`, plus flat `iter`/`iter_mut`
  over every node in the arena. Each supports forward and backward iteration
  (`Iterator` + `DoubleEndedIterator`).

## Commands

- Build: `cargo build`
- Test: `cargo test`
- Bench: `cargo bench` (or `cargo bench -- <filter>` for a subset, e.g. `cargo bench -- traverse`)
- Lint: `cargo clippy`
- Format: `cargo fmt`

## Testing

All tests live inline in `#[cfg(test)]` modules — currently all in `src/tree.rs`,
organized by operation: accessors, alloc, append/insert/remove/detach (including
moving an existing node between parents and subtree removal via
`remove(node, true)`), and iterators (children/ancestors/decendents/traverse),
each covering forward, backward, and mixed front/back iteration plus edge cases
like empty trees and single-node subtrees.

## Benchmarks

Criterion benchmarks live in `benches/tree.rs`, covering allocation, mutation
(`append`, `detach`, `remove` with and without descendants), and iteration
(`traverse`, `decendents`, `children`, `ancestors`, `iter`) across flat, chain,
and balanced-binary tree shapes at a few sizes (100 / 1,000 / 10,000 nodes).
Run `cargo bench` and check `target/criterion/report/index.html` for the full
HTML report.

## Architecture

- `src/tree.rs` — the `Tree<T>` struct and all mutation logic. The only module
  aware of the private `TreeEntry<T>` layout (`parent`, `prev_sibling`,
  `next_sibling`, `first_child`, `last_child`, `value`).
- `src/children.rs`, `src/ancestors.rs`, `src/decendents.rs`, `src/traverse.rs`,
  `src/iter.rs` — read-only, double-ended iterators over the arena.
- `src/lib.rs` — public re-exports: `Tree`, `NodeId`, `Children`, `Descendants`,
  `Iter`/`IterMut`, `Traverse`.
- `benches/tree.rs` — Criterion benchmarks (dev-only; the crate itself stays
  `#![no_std]`).

## License

No license specified yet.
