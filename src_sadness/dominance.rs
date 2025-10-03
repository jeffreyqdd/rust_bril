use crate::blocks::CfgGraph;
use std::{collections::HashSet, usize};

#[derive(Debug, Clone)]
pub struct DominanceUtility {
    dom: Vec<HashSet<usize>>,
    tree: Vec<Option<usize>>,
    tree_children: Vec<HashSet<usize>>,
    df: Vec<HashSet<usize>>,
}

impl DominanceUtility {
    fn reverse_post_order(graph: &CfgGraph) -> Vec<usize> {
        let mut visited = vec![false; graph.edges.len()];
        let mut post_order = Vec::with_capacity(graph.edges.len());

        fn dfs(curr: usize, graph: &CfgGraph, visited: &mut [bool], po: &mut Vec<usize>) {
            if visited[curr] {
                return;
            }
            visited[curr] = true;

            graph.edges[curr].iter().for_each(|&child| {
                dfs(child, graph, visited, po);
            });

            po.push(curr);
        }

        dfs(0, graph, &mut visited, &mut post_order);
        post_order.reverse();
        post_order
    }
    fn dom_relationship(graph: &CfgGraph) -> Vec<HashSet<usize>> {
        let rpo = DominanceUtility::reverse_post_order(graph);
        let n = graph.num_blocks();

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
    fn dom_frontier(dom: &Vec<HashSet<usize>>, graph: &CfgGraph) -> Vec<HashSet<usize>> {
        let mut df = vec![HashSet::new(); dom.len()];

        // source: https://pages.cs.wisc.edu/~fischer/cs701.f05/lectures/Lecture22.pdf
        // DF(N) = { Z | M → Z  ∧  (N dom M)  ∧  ¬(N strictly_dom Z) }
        // A's **domination frontier** contains B if A does not dominate B, but A dominates a predecessor, P, of B

        for b in 0..dom.len() {
            // println!("fixing B = {} dominated by {:?}", b, dom[b]);
            for &p in &graph.predecessors[b] {
                let all_a = &dom[p];
                // println!("\tchecking pred P = {} dominated by {:?}", p, all_a);

                // a by definition, dominates a predecessor of P
                for &a in all_a.iter() {
                    // a must not dominate b
                    if !dom[b].contains(&a) {
                        df[a].insert(b);
                    }
                }
            }
        }

        df
    }
    pub fn from(graph: &CfgGraph) -> Self {
        let dom = DominanceUtility::dom_relationship(graph);
        let tree = DominanceUtility::dom_tree(&dom);
        let tree_children = tree.iter().enumerate().fold(
            vec![HashSet::new(); tree.len()],
            |mut acc, (child, &parent)| {
                if let Some(p) = parent {
                    acc[p].insert(child);
                }
                acc
            },
        );

        let df = DominanceUtility::dom_frontier(&dom, graph);

        Self {
            dom,
            tree,
            tree_children,
            df,
        }
    }
    /// Return a set of nodes that dominate id
    pub fn dominators(&self, id: usize) -> &HashSet<usize> {
        &self.dom[id]
    }

    /// Return a set of nodes that this node immediately dominates
    pub fn dominating(&self, id: usize) -> &HashSet<usize> {
        &self.tree_children[id]
    }

    /// Return the number of nodes in the graph
    pub fn len(&self) -> usize {
        self.dom.len()
    }
    /// Return the parent of id in the dominator tree
    pub fn parent(&self, id: usize) -> Option<usize> {
        self.tree[id]
    }
    /// Return the dominance frontier of id
    pub fn frontier(&self, id: usize) -> &HashSet<usize> {
        &self.df[id]
    }
}
