use std::collections::HashSet;

use crate::representation::{BlockId, ControlFlowGraph};

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DominanceInfo {
    dom: Vec<HashSet<usize>>,
    tree: Vec<Option<usize>>,
    tree_children: Vec<HashSet<usize>>,
    df: Vec<HashSet<usize>>,
}

impl From<&ControlFlowGraph> for DominanceInfo {
    fn from(graph: &ControlFlowGraph) -> Self {
        let dom_now = std::time::Instant::now();
        let dom = DominanceInfo::dom_relationship(graph);
        let tree = DominanceInfo::dom_tree(&dom);
        let tree_children = tree.iter().enumerate().fold(
            vec![HashSet::new(); tree.len()],
            |mut acc, (child, &parent)| {
                if let Some(p) = parent {
                    acc[p].insert(child);
                }
                acc
            },
        );

        let df = DominanceInfo::dom_frontier(&dom, graph);
        log::debug!("computed dominance info in {:?}", dom_now.elapsed());
        Self {
            dom,
            tree,
            tree_children,
            df,
        }
    }
}

impl DominanceInfo {
    fn reverse_post_order(graph: &ControlFlowGraph) -> Vec<usize> {
        let mut visited = vec![false; graph.successors.len()];
        let mut post_order = Vec::with_capacity(graph.successors.len());

        fn dfs(curr: usize, graph: &ControlFlowGraph, visited: &mut [bool], po: &mut Vec<usize>) {
            if visited[curr] {
                return;
            }
            visited[curr] = true;

            graph.successors[curr].iter().for_each(|&child| {
                dfs(child, graph, visited, po);
            });

            po.push(curr);
        }

        dfs(0, graph, &mut visited, &mut post_order);
        post_order.reverse();
        post_order
    }
    fn dom_relationship(graph: &ControlFlowGraph) -> Vec<HashSet<usize>> {
        let rpo = DominanceInfo::reverse_post_order(graph);
        let n = graph.successors.len();

        // init: all nodes
        let mut dom: Vec<HashSet<usize>> = vec![(0..n).collect(); n];
        // entry only dominates itself
        dom[0] = [0].iter().cloned().collect();

        let mut changed = true;
        while changed {
            changed = false;

            for &vertex in &rpo {
                if vertex == 0 {
                    continue; // skip entry
                }

                // start with "all nodes" and intersect with preds
                let mut new_set: Option<HashSet<usize>> = None;
                for &pred in &graph.predecessors[vertex] {
                    let s = dom[pred].clone();
                    new_set = Some(match new_set {
                        None => s,
                        Some(acc) => &acc & &s,
                    });
                }

                let mut new_dom = new_set.unwrap_or_else(|| (0..n).collect());
                new_dom.insert(vertex);

                if new_dom != dom[vertex] {
                    dom[vertex] = new_dom;
                    changed = true;
                }
            }
        }

        dom
    }
    fn dom_tree(dom: &Vec<HashSet<usize>>) -> Vec<Option<usize>> {
        let n = dom.len();
        let mut tree = vec![None; n];

        for id in 0..n {
            // strict dominators = dom[id] \ {id}
            let strict: Vec<_> = dom[id].iter().copied().filter(|&d| d != id).collect();

            if !strict.is_empty() {
                // immediate dominator = the strict dominator that is not dominated by any other
                let idom = strict
                    .iter()
                    .find(|&&d| {
                        strict
                            .iter()
                            .all(|&other| other == d || !dom[other].contains(&d))
                    })
                    .unwrap();

                tree[id] = Some(*idom);
            }
        }

        tree
    }
    fn dom_frontier(dom: &Vec<HashSet<usize>>, graph: &ControlFlowGraph) -> Vec<HashSet<usize>> {
        let mut df = vec![HashSet::new(); dom.len()];

        // A's **domination frontier** contains B if A does not dominate B, but A dominates a predecessor, P, of B
        for b in 0..dom.len() {
            log::trace!("fixing B = {} dominated by {:?}", b, dom[b]);
            for &p in &graph.predecessors[b] {
                let all_a = &dom[p];
                log::trace!("\tchecking pred P = {} dominated by A={:?}", p, all_a);

                // a by definition, dominates a predecessor of P
                for &a in all_a.iter() {
                    // a must not dominate b
                    if !dom[b].contains(&a) || a == b {
                        log::trace!("\t\tDF(A={}) += {}", a, b);
                        df[a].insert(b);
                    }
                }
            }
        }

        df
    }
    /// return block ids that are in the dominance frontier of the given block iod
    pub fn get_dominance_frontier(&self, block_id: BlockId) -> &HashSet<usize> {
        &self.df[block_id]
    }
    /// return the block ids that are immediately dominated by the given block id
    pub fn get_immediate_dominated(&self, block_id: BlockId) -> &HashSet<usize> {
        &self.tree_children[block_id]
    }
}
