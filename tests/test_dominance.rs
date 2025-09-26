use std::collections::HashSet;

use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use rust_bril::{blocks::CfgGraph, dominance::DominanceUtility, program::Program};

#[test]
fn test_dominance_analysis() {
    let entries =
        glob::glob(&format!("benchmarks/**/*.bril")).expect("Failed to read glob pattern");

    for entry in entries {
        let path = entry.expect("Failed to read entry");
        let filename = path.to_str().expect("Failed to convert path to string");

        println!("Testing dominance analysis on: {}", filename);

        let program = Program::from_file(filename);
        let function_blocks = program.basic_blocks();
        function_blocks
            .par_iter()
            .map(|x| CfgGraph::from(&x).prune_unreachable())
            .map(|x| (x.clone(), DominanceUtility::from(&x)))
            .for_each(|(graph, du)| {
                // Check 1: If A dominates B, then there is no path from INIT to B when we remove A
                for id in 0..du.len() {
                    let dominators = du.dominators(id);
                    for removable_node in 0..du.len() {
                        if dominators.contains(&removable_node) {
                            let restricted_reachable = graph.reachable(&0, &id, &HashSet::from([removable_node]));
                            assert!(
                                !restricted_reachable,
                                "Node {} should not be reachable from INIT when removing dominator {:?} {:?} {:#?}",
                                id, removable_node, graph, du
                            );
                        } else {
                            let reachable = graph.reachable(&0, &id, &HashSet::new());
                            assert!(
                                reachable,
                                "Node {} should be reachable from INIT {:?} {:#?}",
                                id, graph, du
                            );
                        }
                    }
                }
            });
    }
}

#[test]
fn test_dominance_tree() {
    let entries =
        glob::glob(&format!("benchmarks/**/*.bril")).expect("Failed to read glob pattern");

    for entry in entries {
        let path = entry.expect("Failed to read entry");
        let filename = path.to_str().expect("Failed to convert path to string");

        println!("Testing dominance tree on: {}", filename);

        let program = Program::from_file(filename);
        let function_blocks = program.basic_blocks();
        function_blocks
            .par_iter()
            .map(|x| CfgGraph::from(&x).prune_unreachable())
            .map(|x| (x.clone(), DominanceUtility::from(&x)))
            .for_each(|(graph, du)| {
                // Check 1: Every root to P path in the dominator tree only contains nodes that dominate P
                for id in 0..du.len() {
                    // println!("checking node {} {}", id, graph.function.name);
                    // go to parent in the dominator tree and see if the parent is one of id's dom
                    let mut current = id;
                    while let Some(parent) = du.parent(current) {
                        // println!("\t {} => {}", current, parent);
                        assert!(
                            du.dominators(id).contains(&parent),
                            "Parent {:?} of node {} in dominator tree should dominate it {:?} {:#?}",
                            parent, id, graph, du
                        );
                        current = parent;
                    }
                }

                // Check 2: Nodes not in the path do not dominate P
                for id in 0..du.len() {
                    let mut not_dominators: HashSet<usize> = (0..du.len()).collect();
                    for &d in du.dominators(id) {
                        not_dominators.remove(&d);
                    }
                    // println!("checking node {} {}", id, graph.function.name);
                    for &d in &not_dominators {
                        let mut current = id;
                        let mut found = false;
                        while let Some(parent) = du.parent(current) {
                            if parent == d {
                                found = true;
                                break;
                            }
                            current = parent;
                        }
                        assert!(
                            !found,
                            "Node {:?} not dominating node {} should not be in its dominator tree path {:?} {:#?}",
                            d, id, graph, du
                        );
                    }
                }
            });
    }
}

#[test]
fn test_dominance_frontier() {
    let entries =
        glob::glob(&format!("benchmarks/**/*.bril")).expect("Failed to read glob pattern");

    for entry in entries {
        let path = entry.expect("Failed to read entry");
        let filename = path.to_str().expect("Failed to convert path to string");

        println!("Testing dominance frontier on: {}", filename);

        let program = Program::from_file(filename);
        let function_blocks = program.basic_blocks();
        function_blocks
            .par_iter()
            .map(|x| CfgGraph::from(&x).prune_unreachable())
            .map(|x| (x.clone(), DominanceUtility::from(&x)))
            .for_each(|(graph, du)| {
                // A's **domination frontier** contains B if A does not dominate B, but A dominates a predecessor, P, of B
                for a in 0..du.len() {
                    let df_a = du.frontier(a);

                    for &b in df_a {
                        // A does not dominate B
                        assert!(
                            !du.dominators(b).contains(&a),
                            "Node {} should not dominate {} but is in its dominance frontier {:?} {:#?}",
                            a, b, graph, du
                        );

                        // A must dominate a predecessor of B
                        let preds = &graph.predecessors[b];
                        let mut found = false;
                        for &p in preds {
                            if du.dominators(p).contains(&a) {
                                found = true;
                                break;
                            }
                        }
                        assert!(
                            found,
                            "Node {} should dominate at least one predecessor of {} but does not {:?} {:#?}",
                            a, b, graph, du
                        );
                    }
                }
            });
    }
}
