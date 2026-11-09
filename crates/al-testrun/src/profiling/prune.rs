//! A sampling profile keeps only the call paths a sample landed in (al-runner issue 6924). The session's profiler
//! notes every call path it enters, in both modes; in sampling mode a path no sample landed in has no time and no
//! hit count, and BC's sampling profiler, which builds its tree from the sampled stacks, has no node for it. One
//! codeunit of Tests-ERM-Purchase entered 703 000 paths for 5 077 samples, and every test codeunit's tree is kept
//! until the run ends, so the paths without a sample were most of the run's memory.

use al_vm::ProfileTree;

/// `tree` without the paths of a sampling profile that hold no sample (theirs or below them), and without the record
/// operations no sample landed in (their sampled time is 0). The root stays. An instrumentation tree is returned as is.
pub fn sampled_paths(mut tree: ProfileTree) -> ProfileTree {
    if !tree.sampling || tree.nodes.is_empty() {
        return tree;
    }
    for n in &mut tree.nodes {
        n.db.retain(|d| d.samples > 0);
    }
    // A child is entered after its parent, so its index is higher: from the end, a node's children are known first.
    let mut keep = vec![false; tree.nodes.len()];
    for i in (0..tree.nodes.len()).rev() {
        let n = &tree.nodes[i];
        keep[i] = i == 0 || n.samples > 0 || !n.db.is_empty() || n.children.iter().any(|&c| keep[c]);
    }
    let mut new_index = vec![usize::MAX; tree.nodes.len()];
    let mut next = 0;
    for (i, k) in keep.iter().enumerate() {
        if *k {
            new_index[i] = next;
            next += 1;
        }
    }
    let nodes = std::mem::take(&mut tree.nodes);
    tree.nodes = nodes
        .into_iter()
        .enumerate()
        .filter(|(i, _)| keep[*i])
        .map(|(_, mut n)| {
            n.parent = n.parent.map(|p| new_index[p]);
            n.children = n.children.iter().filter(|&&c| keep[c]).map(|&c| new_index[c]).collect();
            n
        })
        .collect();
    tree
}
