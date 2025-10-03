use std::collections::{HashMap, HashSet};

use crate::{
    blocks::{BasicBlock, CfgGraph, Terminator},
    dominance::DominanceUtility,
    program::{Code, Type, ValueOp},
};

fn rename_vars(
    block_id: usize,
    graph: &mut CfgGraph,
    stack: &mut HashMap<String, Vec<String>>,
    dom: &DominanceUtility,
    names_given: &mut HashMap<String, usize>,
    phi_mapping: &mut HashMap<String, String>,
    type_mapping: &mut HashMap<String, Vec<Type>>,
) {
    eprintln!("Renaming vars in block {}", block_id);
    let stack_before = stack.clone();
    let type_before = type_mapping.clone();

    // this function takes in an arg list and a mapping form old var name to a stack of new var names
    // and modifies the arg list in place
    let replace = |args: &mut Vec<String>, s: &HashMap<String, Vec<String>>| {
        for arg in args.iter_mut() {
            if let Some(v) = s.get(arg) {
                if let Some(top) = v.last() {
                    *arg = top.clone();
                }
            }
        }
    };

    // iterate through each instruction in the block
    // TODO: Phi nodes are incorrectly being renamed, but this should be fine when converting out of SSA?
    for instr in &mut graph.function.basic_blocks[block_id].block {
        eprintln!("\texecuting: {:?}", instr);

        let is_phi = if let Code::Value {
            op: ValueOp::Phi, ..
        } = instr
        {
            true
        } else {
            false
        };

        let tmp: Option<(Option<&mut String>, Option<&mut Vec<String>>)> = match instr {
            Code::Value {
                dest,
                args,
                value_type,
                ..
            } => {
                type_mapping
                    .entry(dest.clone())
                    .or_insert_with(Vec::new)
                    .push(value_type.clone());

                Some((Some(dest), args.as_mut()))
            }
            Code::Memory {
                dest,
                args,
                ptr_type,
                ..
            } => {
                if let Some(d) = dest {
                    type_mapping.entry(d.clone()).or_insert_with(Vec::new).push(
                        ptr_type
                            .as_ref()
                            .expect("ptr_type must not be None if dest is Some")
                            .clone(),
                    );
                }
                Some((dest.as_mut(), args.as_mut()))
            }
            Code::Effect { args, .. } => Some((None, args.as_mut())),
            Code::Constant { dest, .. } => {
                type_mapping
                    .entry(dest.clone())
                    .or_insert_with(Vec::new)
                    .push(Type::Int); // constants are always int for now
                Some((Some(dest), None))
            }
            Code::Label { .. } | Code::Noop { .. } => None,
        };

        if let Some((dest_opt, args_opt)) = tmp {
            // for each argument to instr, replace with stack[old name]
            if let Some(args) = args_opt {
                replace(args, stack);
            }

            // replace instr's destination with a new name and push that onto the stack[old name]
            // Rename should only be called once as we're going down the dominance tree
            if let Some(old_name) = dest_opt {
                // get fresh new suffix
                let new_suffix = names_given
                    .entry(old_name.clone())
                    .and_modify(|x| *x += 1)
                    .or_default();

                let new_name = format!("{}_{}", old_name, new_suffix);

                // eprintln!("replacing {} with {}", old_name, new_name);
                stack
                    .entry(old_name.clone())
                    .or_insert_with(Vec::new)
                    .push(new_name.clone());

                let item = type_mapping
                    .get(old_name)
                    .and_then(|v| v.last())
                    .cloned()
                    .unwrap_or(Type::None)
                    .clone();

                type_mapping
                    .entry(new_name.clone())
                    .or_insert_with(Vec::new)
                    .push(item);

                if is_phi {
                    phi_mapping.insert(new_name.clone(), old_name.clone());
                    // eprintln!("new phi mapping {:?}", phi_mapping);
                }

                *old_name = new_name;
            }
        }
    }

    // for s in block's successors and for p in s's phi-nodes,
    // eprintln!("currently in block: {}", block_id);
    // eprintln!("successor in block: {:?}", graph.edges[block_id]);
    for succ in &mut graph.edges[block_id] {
        eprintln!("\tsucc: {}", succ);
        let parent_label = graph.function.basic_blocks[block_id].label.clone();
        for p in &mut graph.function.basic_blocks[*succ].block {
            eprintln!("\t\tcode: {:?}", p);
            match p {
                Code::Value {
                    op: ValueOp::Phi,
                    dest,
                    args,
                    labels,
                    value_type,
                    ..
                } => {
                    let original_name = phi_mapping.get(dest).unwrap_or(dest);
                    let stack_name = stack.get(original_name);

                    if stack_name == None {
                        continue;
                    }

                    if let Some(v) = stack_name {
                        if let Some(name) = v.last() {
                            if let Some(vec) = args {
                                vec.push(name.clone());
                            } else {
                                *args = Some(vec![name.clone()]);
                            }

                            if let Some(vec) = labels {
                                vec.push(parent_label.clone());
                            } else {
                                *labels = Some(vec![parent_label.clone()]);
                            }

                            // update type
                            if let Some(t) = type_mapping.get(name).and_then(|v| v.last()) {
                                *value_type = t.clone();
                            }
                        }
                    }

                    // eprintln!("PHI: {} {:?}", &original_name, args);

                    // eprintln!("PHJI AFTEr {:?}", p);
                }
                _ => continue,
            }
        }
    }

    //   for s in block's successors:
    //     for p in s's ϕ-nodes:
    //       Assuming p is for a variable v, make it read from stack[v].

    // for block b that is immediately dominated by the current block

    for b in dom.dominating(block_id) {
        // from_ssa        // eprintln!("recursing on {} => {}", block_id, b);
        rename_vars(
            *b,
            graph,
            stack,
            dom,
            names_given,
            phi_mapping,
            type_mapping,
        );
    }

    // pop all the names we just pushed onto the stack
    // eprintln!("now {:?} orig {:?}", stack, stack_before);
    *stack = stack_before;
    *type_mapping = type_before;
}

pub fn to_ssa(graph: &CfgGraph) -> CfgGraph {
    let mut g = graph.clone().prune_unreachable();

    // --- step 1: insert phi nodes
    // for each block, find assignment to variable v
    let mut assignment: HashMap<&String, HashSet<usize>> = HashMap::new();

    for (idx, block) in graph.function.basic_blocks.iter().enumerate() {
        for instr in block.block.iter() {
            match instr {
                Code::Constant { dest, .. } => {
                    assignment
                        .entry(dest)
                        .or_insert_with(|| HashSet::new())
                        .insert(idx);
                }

                Code::Value { dest, .. } => {
                    assignment
                        .entry(dest)
                        .or_insert_with(|| HashSet::new())
                        .insert(idx);
                }
                Code::Memory { dest: Some(d), .. } => {
                    assignment
                        .entry(d)
                        .or_insert_with(|| HashSet::new())
                        .insert(idx);
                }
                _ => continue,
            }
        }
    }

    // iterate through all blocks, and for each assignment in that block, push phi nodes into each block in its
    // dominance frontier
    let dom = DominanceUtility::from(&g);
    let mut phi_inserted: HashMap<&String, HashSet<usize>> = HashMap::new();
    loop {
        let mut to_insert = vec![];
        for &var in assignment.keys() {
            for d in assignment.get(var).unwrap_or(&HashSet::new()) {
                for block in dom.frontier(*d) {
                    if phi_inserted.get(var).map_or(false, |s| s.contains(block)) {
                        continue;
                    }

                    // if first element is a label, insert after
                    let index = if let Some(Code::Label { .. }) =
                        g.function.basic_blocks[*block].block.first()
                    {
                        1
                    } else {
                        0
                    };

                    g.function.basic_blocks[*block].block.insert(
                        index,
                        Code::Value {
                            op: ValueOp::Phi,
                            dest: var.clone(),
                            args: None,
                            labels: None,
                            value_type: Type::None,
                            funcs: None,
                        },
                    );

                    eprintln!(
                        "From {} Inserting Phi for {} in block id {}",
                        *d, var, block
                    );
                    phi_inserted
                        .entry(var)
                        .or_insert_with(|| HashSet::new())
                        .insert(*block);
                    to_insert.push((var, *block));

                    // if block has a back-edge to itself, we should insert phi nodes for all
                    // variables in this block
                    if g.edges.iter().any(|edges| edges.contains(block)) {
                        for (k, v) in assignment.iter() {
                            if v.contains(block)
                                && !phi_inserted.get(k).map_or(false, |s| s.contains(block))
                            {
                                g.function.basic_blocks[*block].block.insert(
                                    index,
                                    Code::Value {
                                        op: ValueOp::Phi,
                                        dest: k.to_string(),
                                        args: None,
                                        labels: None,
                                        value_type: Type::None,
                                        funcs: None,
                                    },
                                );

                                eprintln!(
                                    "From {} Inserting Phi for {} in block id {} due to back-edge",
                                    *d, k, block
                                );
                                phi_inserted
                                    .entry(k)
                                    .or_insert_with(|| HashSet::new())
                                    .insert(*block);
                                to_insert.push((k, *block));
                            }
                        }
                    }
                }
            }
        }

        if to_insert.len() == 0 {
            break;
        }

        for (var, block) in to_insert {
            assignment
                .entry(var)
                .or_insert_with(|| HashSet::new())
                .insert(block);
        }
    }

    // --- step 2: rename variables, modifying phi nodes as needed
    let mut name_stack: HashMap<String, Vec<String>> = HashMap::new();
    let mut names_given: HashMap<String, usize> = HashMap::new();
    let mut phi_mapping: HashMap<String, String> = HashMap::new();
    let mut type_mapping: HashMap<String, Vec<Type>> = HashMap::new();

    // should insert function arguments into stack
    for arg in g.function.args.iter().flatten() {
        name_stack.insert(arg.name.clone(), vec![arg.name.clone()]);
    }

    rename_vars(
        0,
        &mut g,
        &mut name_stack,
        &dom,
        &mut names_given,
        &mut phi_mapping,
        &mut type_mapping,
    );
    eprintln!("{:#?}", g);

    // prune: remove all phi nodes with 1 argument and label
    // for block in &mut g.function.basic_blocks {
    //     block.block.retain(|item| {
    //         if let Code::Value {
    //             op: ValueOp::Phi,
    //             args,
    //             ..
    //         } = item
    //         {
    //             if let Some(a) = args {
    //                 if a.len() <= 1 {
    //                     return false;
    //                 }
    //             } else {
    //                 return false;
    //             }
    //         }

    //         true
    //     });
    // }

    g
    // un_abstractify(g)
}

pub fn from_ssa(mut graph: CfgGraph) -> CfgGraph {
    for block_id in 0..graph.num_blocks() {
        let curr_block = &mut graph.function.basic_blocks[block_id];
        let mut phis = Vec::new();
        curr_block.block.retain(|item| {
            if let Code::Value {
                op: ValueOp::Phi,
                dest,
                args,
                labels,
                value_type,
                ..
            } = item
            {
                phis.push((
                    dest.clone(),
                    args.clone().unwrap_or_default(),
                    labels.clone().unwrap_or_default(),
                    value_type.clone(),
                ));
                false
            } else {
                true
            }
        });

        // for each phi node in this block, for each predecessor, insert an assignment from the appropriate arg to dest
        for (dest, args, labels, t) in phis {
            for (i, label) in labels.iter().enumerate() {
                let pred_block_id = graph
                    .function
                    .basic_blocks
                    .iter()
                    .position(|b| &b.label == label)
                    .expect("Label in phi node not found in predecessors");
                let arg = &args[i];
                let assign_instr = Code::Value {
                    op: ValueOp::Id,
                    dest: dest.clone(),
                    args: Some(vec![arg.clone()]),
                    value_type: t.clone(),
                    funcs: None,
                    labels: None,
                };
                let pred_block = &mut graph.function.basic_blocks[pred_block_id];

                // insert before the terminator
                let is_passthrough = pred_block.terminator == Terminator::Passthrough;
                if is_passthrough || pred_block.block.is_empty() {
                    pred_block.block.push(assign_instr);
                } else {
                    let term_index = pred_block.block.len() - 1;
                    pred_block.block.insert(term_index, assign_instr);
                }
            }
        }
    }
    // for (dest, args, labels) in phis {
    //     for i in 0..args.size() {}
    // }
    // }

    // for (idx, block) in graph.function.basic_blocks.iter().enumerate() {

    graph
}
