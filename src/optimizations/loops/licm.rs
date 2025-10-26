// Natural loops are strongly connected components in the CFG with a single entry.
// Natural loops are formed around backedges, which are edges from A to B where B dominates A.
//     (Side note: There are actually two common definitions of backedges: this one, and one that relies on a depth-first search (DFS). By the other definition, a backedge is any edge that takes you to an already-visited node during DFS. The relationship between these two definitions is not 100% clear to me, although they are certainly not equivalent, at least for irreducible CFGs.)
// A natural loop is the smallest set of vertices L including A and B such that, for every v in L, either all the predecessors of v are in L or v=B.
// A CFG is reducible iff every backedge has a natural loop.
//     A language that only has for, while, if, break, continue, etc. can only generate reducible CFGs. You need goto or something to generate irreducible CFGs.

use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    dataflow::{run_dataflow_analysis, ReachingDefinitions, WorklistResult},
    representation::{AbstractFunction, Code},
};

struct NaturalLoop {
    header: usize,
    nodes: HashSet<usize>,
    backedge_source: usize,
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

    let reaching_definitions = run_dataflow_analysis::<ReachingDefinitions>(&mut af)?;

    // --- Step 1: grow loop candidates
    // key = natural loop header, value = set of nodes in the natural loop
    let mut natural_loops: Vec<NaturalLoop> = Vec::new();
    for source in 0..af.cfg.basic_blocks.len() {
        for &header in &af.cfg.successors[source] {
            if af.dominance_info.dominated_by(source, header) {
                let header_name = &af.cfg.basic_blocks[header].label;
                let source_name = &af.cfg.basic_blocks[source].label;
                log::debug!(
                    "candidate header: '{}' dominates backedge source: '{}'",
                    header_name,
                    source_name
                );

                let loop_nodes = find_loop_nodes(&af, header, source);
                natural_loops.push(NaturalLoop {
                    header,
                    nodes: loop_nodes,
                    backedge_source: source,
                });
            }
        }
    }

    // --- Step 2: filter only for natural loops
    natural_loops.retain(|candidate| is_natural_loop(&af, candidate));

    for nl in &natural_loops {
        let header_name = &af.cfg.basic_blocks[nl.header].label;
        log::info!("found natural loop '{}'", header_name);
        for node in &nl.nodes {
            log::trace!("  {}", af.cfg.basic_blocks[*node].label);
        }
    }

    // Step 3: identify loop-invariant instructions
    let mut final_licm = vec![];
    for nl in &natural_loops {
        let mut loop_invariant_instructions: HashMap<String, (Code, usize)> = HashMap::new();
        let mut loop_invariant_instructions_ordered = vec![];
        let mut changed = true;

        // Iterate to convergence
        while changed {
            changed = false;
            for &node in &nl.nodes {
                let block = &af.cfg.basic_blocks[node];
                for instruction in &block.instructions {
                    let dest = match instruction.get_destination() {
                        Some(dest) => dest,
                        None => continue,
                    };

                    if loop_invariant_instructions.contains_key(dest) {
                        continue;
                    }

                    // unless we can prove that the call function is side effect free, we cannot process it
                    if instruction.has_side_effects() {
                        continue;
                    }

                    let is_invariant = if instruction.is_constant() {
                        true
                    } else if let Some(args) = instruction.get_arguments() {
                        args.iter().all(|arg| {
                            let reaching_defs = &reaching_definitions[&block.id].1[arg];
                            // Either all defs outside loop OR single def already marked invariant
                            (&nl.nodes & reaching_defs).is_empty()
                                || (reaching_defs.len() == 1
                                    && loop_invariant_instructions.contains_key(arg))
                        })
                    } else {
                        false
                    };

                    if is_invariant {
                        loop_invariant_instructions
                            .insert(dest.to_owned(), (instruction.clone(), block.id));
                        loop_invariant_instructions_ordered.push((instruction.clone(), block.id));
                        changed = true;
                        log::info!(
                            "found loop-invariant: {} in natural loop '{}' in block '{}'",
                            dest,
                            af.cfg.basic_blocks[nl.header].label,
                            af.cfg.basic_blocks[block.id].label
                        );
                    }
                }
            }
        }

        log::info!(
            "loop '{}' has {} invariant instructions",
            af.cfg.basic_blocks[nl.header].label,
            loop_invariant_instructions.len()
        );
        final_licm.push((nl, loop_invariant_instructions_ordered));
    }

    // Step 4: Actually move the loop-invariant code
    let mut already_removed = HashSet::new();
    for (nl, licm_instructions_ordered) in final_licm {
        if licm_instructions_ordered.is_empty() {
            continue;
        }
        // Move instructions to preheader
        for (instruction, source_block_id) in licm_instructions_ordered {
            // remove instruction from original location
            if already_removed.contains(&instruction) {
                continue;
            }

            let s = format!(
                "instruction '{:?}' not found in block '{}'",
                instruction, af.cfg.basic_blocks[source_block_id].label
            );
            assert!(
                af.cfg.basic_blocks[source_block_id]
                    .instructions
                    .contains(&instruction),
                "{}{}\n{:#?}",
                s,
                af.cfg.basic_blocks[source_block_id].label,
                af.cfg.basic_blocks[source_block_id].instructions
            );

            af.cfg.basic_blocks[source_block_id]
                .instructions
                .retain(|instr| instr != &instruction);

            already_removed.insert(instruction.clone());

            // Add to preheader
            af.cfg.basic_blocks[nl.header].preheader.push(instruction);
        }

        af.cfg.basic_blocks[nl.backedge_source].natural_loop_return = true;
    }

    log::info!("finished in {:?}", start_time.elapsed());
    Ok(af)
}

fn find_loop_nodes(af: &AbstractFunction, header: usize, source: usize) -> HashSet<usize> {
    // minimal set of nodes including header and source such that for every node in the set,
    // either all its predecessors are in the set, or it is the header
    let mut loop_nodes = HashSet::from([header, source]);
    let mut worklist = VecDeque::new();

    if header != source {
        worklist.push_back(source);
    }

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
    log::trace!(
        "checking if candidate with header '{}' is a natural loop",
        af.cfg.basic_blocks[candidate.header].label
    );
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
