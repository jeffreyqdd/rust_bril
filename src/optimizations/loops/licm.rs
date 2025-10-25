// Natural loops are strongly connected components in the CFG with a single entry.
// Natural loops are formed around backedges, which are edges from A to B where B dominates A.
//     (Side note: There are actually two common definitions of backedges: this one, and one that relies on a depth-first search (DFS). By the other definition, a backedge is any edge that takes you to an already-visited node during DFS. The relationship between these two definitions is not 100% clear to me, although they are certainly not equivalent, at least for irreducible CFGs.)
// A natural loop is the smallest set of vertices L including A and B such that, for every v in L, either all the predecessors of v are in L or v=B.
// A CFG is reducible iff every backedge has a natural loop.
//     A language that only has for, while, if, break, continue, etc. can only generate reducible CFGs. You need goto or something to generate irreducible CFGs.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    dataflow::{run_dataflow_analysis, WorklistResult},
    representation::{AbstractFunction, Code},
};

struct NaturalLoop {
    header: usize,
    nodes: HashSet<usize>,
}

pub fn loop_invariant_code_motion_pass(
    mut af: AbstractFunction,
) -> WorklistResult<AbstractFunction> {
    log::info!(
        "running loop invariant code motion pass on function {}",
        af.name
    );
    let start_time = std::time::Instant::now();

    // --- Step 0: calculate reaching definitions, made easy by SSA form

    // let reaching_definitions = run_dataflow_analysis::<ReachingDefinitions>(&mut af)?;

    // --- Step 1: grow loop candidates
    // key = natural loop header, value = set of nodes in the natural loop
    let mut natural_loops: Vec<NaturalLoop> = Vec::new();
    for source in 0..af.cfg.basic_blocks.len() {
        for &header in &af.cfg.successors[source] {
            if af.dominance_info.dominated_by(source, header) {
                let header_name = &af.cfg.basic_blocks[header].label;
                let source_name = &af.cfg.basic_blocks[source].label;
                log::error!(
                    "candidate header: '{}' dominates backedge source: '{}'",
                    header_name,
                    source_name
                );

                let loop_nodes = find_loop_nodes(&af, header, source);
                natural_loops.push(NaturalLoop {
                    header,
                    nodes: loop_nodes,
                });
            }
        }
    }

    // --- Step 2: filter only for natural loops
    natural_loops.retain(|candidate| is_natural_loop(&af, candidate));

    for nl in &natural_loops {
        let header_name = &af.cfg.basic_blocks[nl.header].label;
        log::trace!("found natural loop '{}'", header_name);
        for node in &nl.nodes {
            log::trace!("  {}", af.cfg.basic_blocks[*node].label);
        }
    }

    // -- Step 3: identify loop-invariant instructions
    // for nl in &natural_loops {
    //     let mut loop_invariant_instructions: HashSet<Code> = HashSet::new();
    //     let mut changed = true;
    //     while changed {
    //         changed = false;
    //         for &node in &nl.nodes {
    //             let block = &af.cfg.basic_blocks[node];
    //             for instruction in &block.instructions {
    //                 if loop_invariant_instructions.contains(instruction) {
    //                     continue;
    //                 }

    //                 if let Some(instruction_arguments) = instruction.get_arguments() {
    //                     for instruction_argument in instruction_arguments {
    //                         // all reaching definitions are outside of loop, or there is exactly one definition,
    //                         // and it is already marked as loop invariant
    //                         let reaching_defs =
    //                             &reaching_definitions[&block.id].1[instruction_argument];
    //                         log::error!(
    //                             "instruction argument: {} is reached from {:?}",
    //                             instruction_argument,
    //                             reaching_defs
    //                         );
    //                     }
    //                 }
    //             }
    //         }
    //     }
    // }

    // iterate to convergence:
    // for every instruction in the loop:
    //     mark it as LI iff, for all arguments x, either:
    //         all reaching defintions of x are outside of the loop, or
    //         there is exactly one definition, and it is already marked as
    //             loop invariant

    log::info!("finished in {:?}", start_time.elapsed());

    Ok(af)
}

fn find_loop_nodes(af: &AbstractFunction, header: usize, source: usize) -> HashSet<usize> {
    // minimal set of nodes including header and source such that for every node in the set,
    // either all its predecessors are in the set, or it is the header
    let mut loop_nodes = HashSet::from([header, source]);
    let mut worklist = VecDeque::from([source]);

    while let Some(node) = worklist.pop_front() {
        let node_name = &af.cfg.basic_blocks[node].label;
        log::trace!("  visiting node '{}'", node_name);
        for &pred in &af.cfg.predecessors[node] {
            if !loop_nodes.contains(&pred) && pred != header {
                loop_nodes.insert(pred);
                worklist.push_back(pred);
            }
        }
    }

    loop_nodes
}

/// Check if the given set of nodes form a natural loop
fn is_natural_loop(af: &AbstractFunction, candidate: &NaturalLoop) -> bool {
    // if the node is not the header, then all of its predecessors must be in the loop, or the header
    // otherwise, this is not an natural loop
    candidate
        .nodes
        .iter()
        .filter(|&&node| node != candidate.header)
        .all(|&node| {
            af.cfg.predecessors[node]
                .iter()
                .all(|pred| candidate.nodes.contains(pred) || *pred == candidate.header)
        })
}

/// trivialized by ssa form
fn reaching_definitions(af: &AbstractFunction) -> Vec<HashMap<String, HashSet<usize>>> {
    let ret = vec![];
    for (idx, block) in af.cfg.basic_blocks.iter().enumerate() {
        let mut reaching_definitions = HashMap::new();

        // argument for block 0
        if block.id == 0 {
            if let Some(arguments) = &af.args {
                for argument in arguments {
                    reaching_definitions.insert(argument.name.clone(), 0);
                }
            }
        }

        // for instruction in block.instructions {}
    }
    ret
}
