# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

- Build: `cargo build`
- Test (all): `cargo test`
- Test (single): `cargo test <test_name>` (e.g. `cargo test detach_appends_children_to_new_parent`)
- Test (single module): `cargo test tree::tests::`
- Lint: `cargo clippy`
- Format: `cargo fmt`
- Bench (all): `cargo bench`
- Bench (filtered): `cargo bench -- <substring>` (e.g. `cargo bench -- traverse`)

There is no test harness beyond `#[cfg(test)]` modules — all tests currently live inline in `src/tree.rs`. Benchmarks (Criterion, dev-dependency only — the crate itself stays `#![no_std]`) live in `benches/tree.rs` and cover alloc/append/detach/remove/traverse/decendents/children/ancestors/iter across flat, chain, and balanced-binary tree shapes.

## Architecture

`trae-rs` (crate name `trae_rs`) is a `#![no_std]` arena-allocated tree data structure, using `alloc` and backed by `slotmap::SlotMap` for node storage. It follows the same design family as crates like `indextree`: nodes are addressed by a generational `NodeId` key rather than by pointer/reference, so the tree has no lifetime-based parent/child borrowing issues and node removal is safe against dangling references (stale `NodeId`s just fail lookups).

### Core layout

- `src/tree.rs` — the `Tree<T>` struct and all mutation logic (`alloc`, `append`, `insert_before`/`insert_after`, `detach`, `remove`). This is the only module that holds the `SlotMap` and is aware of the private `TreeEntry<T>` layout (`parent`, `prev_sibling`, `next_sibling`, `first_child`, `last_child`, `value`). All other modules borrow the arena immutably through crate-visible fields on `TreeEntry`. All unit tests currently live here, organized by operation (append/insert/remove/detach/iterators/ancestors/children/descendants/traverse).
- `src/children.rs`, `src/ancestors.rs`, `src/decendents.rs`, `src/traverse.rs`, `src/iter.rs` — read-only, double-ended iterators over the arena. Each holds a `&SlotMap` reference plus small cursor state (e.g. `front`/`back` `NodeId`s) rather than cloning data, and each implements `Iterator` + `DoubleEndedIterator` (and `ExactSizeIterator` where cheaply computable) so traversal can run forwards and backwards symmetrically.
- `Descendants` (`src/decendents.rs`) is implemented as a thin filter over `Traverse`'s `NodeEdge::Start`/`NodeEdge::End` events, not as an independent walk — when changing traversal semantics, `traverse.rs` is the source of truth and `decendents.rs` should stay in sync with it.
- `src/lib.rs` re-exports the public surface: `Tree`, `NodeId`, `Children`, `Descendants`, `Iter`/`IterMut`, `Traverse`.

### Key invariants to preserve when touching `tree.rs`

- Every structural mutation (`append`, `insert_before`, `insert_after`) starts by detaching the node being moved first, so a node is never linked into two places in the sibling/parent graph at once.
- `detach` takes a `children: impl Into<ChildDetach>` parameter with three modes: `KeepChildren`/`DetachChildren` (via `bool`) or `AppendToParent(NodeId)` (via `NodeId`) — decide the right one when adding new call sites rather than always defaulting to `false`.
- `Traverse` supports simultaneous forward and backward iteration on the same cursor (`next()`/`next_back()` interleaved) and must stay fused (return `None` forever once `front`/`back` meet) — several tests (`traverse_mixed_front_and_back`, `traverse_fused_after_exhaustion`) exist specifically to guard this.
- Iterating a subtree (`traverse(node)`, `decendents(node)` where `node` isn't the tree root) must never escape upward past `node` — `next_of_next`/`prev_of_prev` check `node == self.root` before following `parent`.
