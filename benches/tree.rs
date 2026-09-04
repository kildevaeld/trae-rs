use criterion::{BatchSize, BenchmarkId, Criterion, criterion_group, criterion_main};
use trae::{NodeId, Tree};

const SIZES: [usize; 3] = [100, 1_000, 10_000];

/// Builds a flat tree: one root with `n` direct children.
fn build_flat_tree(n: usize) -> (Tree<usize>, NodeId) {
    let mut tree = Tree::new();
    let root = tree.alloc(0);
    for i in 0..n {
        let child = tree.alloc(i);
        tree.append(root, child);
    }
    (tree, root)
}

/// Builds a deep, single-child-per-level chain of length `n` rooted at `root`.
fn build_chain_tree(n: usize) -> (Tree<usize>, NodeId) {
    let mut tree = Tree::new();
    let root = tree.alloc(0);
    let mut parent = root;
    for i in 0..n {
        let child = tree.alloc(i);
        tree.append(parent, child);
        parent = child;
    }
    (tree, root)
}

/// Builds a balanced binary tree with roughly `n` nodes, returning the root and the
/// deepest-first leaf (useful for ancestor walks).
fn build_binary_tree(n: usize) -> (Tree<usize>, NodeId, NodeId) {
    let mut tree = Tree::new();
    let root = tree.alloc(0);
    let mut frontier = vec![root];
    let mut count = 1usize;
    let mut deepest_leaf = root;

    'outer: while count < n {
        let mut next_frontier = Vec::new();
        for parent in frontier {
            for _ in 0..2 {
                if count >= n {
                    break 'outer;
                }
                let child = tree.alloc(count);
                tree.append(parent, child);
                next_frontier.push(child);
                deepest_leaf = child;
                count += 1;
            }
        }
        frontier = next_frontier;
    }

    (tree, root, deepest_leaf)
}

fn bench_alloc(c: &mut Criterion) {
    let mut group = c.benchmark_group("alloc");
    for size in SIZES {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                Tree::new,
                |mut tree: Tree<usize>| {
                    for i in 0..size {
                        std::hint::black_box(tree.alloc(i));
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_append_flat(c: &mut Criterion) {
    let mut group = c.benchmark_group("append_flat");
    for size in SIZES {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || {
                    let mut tree: Tree<usize> = Tree::new();
                    let root = tree.alloc(0);
                    let children: Vec<_> = (0..size).map(|i| tree.alloc(i)).collect();
                    (tree, root, children)
                },
                |(mut tree, root, children)| {
                    for child in children {
                        tree.append(root, child);
                    }
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_detach(c: &mut Criterion) {
    let mut group = c.benchmark_group("detach_middle_child");
    for size in SIZES {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || build_flat_tree(size),
                |(mut tree, root)| {
                    let target = tree.children(root).nth(size / 2).unwrap();
                    tree.detach(target, false);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_remove_single(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove_single_node");
    for size in SIZES {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || build_flat_tree(size),
                |(mut tree, root)| {
                    let target = tree.children(root).nth(size / 2).unwrap();
                    tree.remove(target, false);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_remove_subtree(c: &mut Criterion) {
    let mut group = c.benchmark_group("remove_with_descendants");
    for size in SIZES {
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, &size| {
            b.iter_batched(
                || build_chain_tree(size),
                |(mut tree, root)| {
                    tree.remove(root, true);
                },
                BatchSize::SmallInput,
            );
        });
    }
    group.finish();
}

fn bench_traverse(c: &mut Criterion) {
    let mut group = c.benchmark_group("traverse");
    for size in SIZES {
        let (tree, root, _) = build_binary_tree(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                for edge in tree.traverse(root) {
                    std::hint::black_box(edge);
                }
            });
        });
    }
    group.finish();
}

fn bench_descendants(c: &mut Criterion) {
    let mut group = c.benchmark_group("descendants");
    for size in SIZES {
        let (tree, root, _) = build_binary_tree(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                for id in tree.decendents(root) {
                    std::hint::black_box(id);
                }
            });
        });
    }
    group.finish();
}

fn bench_children_iteration(c: &mut Criterion) {
    let mut group = c.benchmark_group("children_iteration");
    for size in SIZES {
        let (tree, root) = build_flat_tree(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                for id in tree.children(root) {
                    std::hint::black_box(id);
                }
            });
        });
    }
    group.finish();
}

fn bench_ancestors(c: &mut Criterion) {
    let mut group = c.benchmark_group("ancestors_from_deepest_leaf");
    for size in SIZES {
        let (tree, _root, leaf) = build_binary_tree(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                for id in tree.ancestors(leaf) {
                    std::hint::black_box(id);
                }
            });
        });
    }
    group.finish();
}

fn bench_iter(c: &mut Criterion) {
    let mut group = c.benchmark_group("iter_all_nodes");
    for size in SIZES {
        let (tree, _root) = build_flat_tree(size);
        group.bench_with_input(BenchmarkId::from_parameter(size), &size, |b, _| {
            b.iter(|| {
                for entry in tree.iter() {
                    std::hint::black_box(entry);
                }
            });
        });
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_alloc,
    bench_append_flat,
    bench_detach,
    bench_remove_single,
    bench_remove_subtree,
    bench_traverse,
    bench_descendants,
    bench_children_iteration,
    bench_ancestors,
    bench_iter,
);
criterion_main!(benches);
