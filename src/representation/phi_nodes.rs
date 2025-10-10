use std::collections::{HashMap, HashSet, VecDeque};

use crate::{
    dataflow::{
        run_dataflow_analysis, LiveVariables, WorklistError, WorklistProperty, WorklistResult,
    },
    representation::{
        AbstractFunction, Argument, BasicBlock, BlockId, Code, Label, Position, Terminator, Type,
        ValueOp, Variable,
    },
};

#[derive(Debug, Clone)]
pub struct PhiNode {
    /// The destination variable that this phi node defines
    pub dest: Variable,
    /// The original name of the phi node
    pub original_name: Variable,
    /// The type of the phi node result
    pub phi_type: Type,
    /// Vector of incoming values for this phi node
    pub phi_args: Vec<(Variable, Label)>,
}

impl PhiNode {
    pub fn empty(dest: Variable) -> Self {
        Self {
            dest: dest.clone(),
            original_name: dest,
            phi_type: Type::None,
            phi_args: vec![],
        }
    }
}

struct PhiTypeWorklist {}
impl WorklistProperty for PhiTypeWorklist {
    type Domain = HashMap<Variable, (Type, Option<Position>)>;

    fn init(_: usize, _: &AbstractFunction) -> Self::Domain {
        Self::Domain::default()
    }

    fn is_forward() -> bool {
        true
    }

    fn merge(predecessors: Vec<(&BlockId, &Self::Domain)>) -> WorklistResult<Self::Domain> {
        if predecessors.is_empty() {
            return Ok(Self::Domain::default());
        }

        let result = predecessors
            .into_iter()
            .fold(Self::Domain::default(), |mut acc, (_, d)| {
                d.iter().for_each(|(var, e)| {
                    acc.entry(var.clone()).insert_entry(e.clone());
                });
                acc
            });

        Ok(result)
    }

    fn transfer(
        mut domain: Self::Domain,
        block: &mut BasicBlock,
        _: Option<&Vec<Argument>>,
    ) -> WorklistResult<Self::Domain> {
        // process phi nodes
        for phi in &mut block.phi_nodes {
            let argument_types = phi
                .phi_args
                .iter()
                .map(|(v, _)| domain.get(v))
                .flatten()
                .collect::<Vec<_>>();

            if argument_types.is_empty() {
                continue;
            }

            log::trace!("phi candidates: {:?}", argument_types);
            // If there are conflicting types, raise error
            let mut seen = HashSet::new();
            for (t, p) in argument_types.iter() {
                seen.insert(t);
                if seen.len() > 1 {
                    return Err(WorklistError::transfer_error(
                        &block,
                        format!("phi node has conflicting types: {:?}", seen),
                        p,
                    ));
                }
            }
            phi.phi_type = seen.into_iter().next().unwrap().clone();
            domain.insert(phi.dest.clone(), (phi.phi_type.clone(), None));
            log::trace!("assigning type to phi: {}", phi);
        }

        for instruction in &block.instructions {
            if let Some(t) = instruction.get_type() {
                if let Some(d) = instruction.get_destination() {
                    domain.insert(d.to_string(), (t.clone(), instruction.get_position()));
                }
            }
        }

        Ok(domain)
    }
}

impl std::fmt::Display for PhiNode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let args: Vec<String> = self
            .phi_args
            .iter()
            .map(|(var, label)| format!("{} from {}", var, label))
            .collect();
        write!(
            f,
            "[{}:{:?}= φ({})]",
            self.dest,
            self.phi_type,
            args.join(", ")
        )
    }
}

fn lookup_in_stack<'a>(
    old_name: impl Iterator<Item = &'a String>,
    stack: &HashMap<String, Vec<String>>,
) -> Vec<String> {
    old_name
        .map(|old_name| {
            stack
                .get(old_name)
                .expect(&format!("Failed to find stack entry for {}", old_name))
                .last()
                .expect(&format!("Failed to find last entry for {}", old_name))
                .to_string()
        })
        .collect()
}

fn rename(
    current_block_id: BlockId,
    abstract_function: &mut AbstractFunction,
    stack: &mut HashMap<String, Vec<String>>,
    counter: &mut HashMap<String, usize>,
    debug_stack: &mut Vec<String>,
) {
    // save context
    let stack_saved = stack.clone();
    let cbl = abstract_function.cfg.basic_blocks[current_block_id]
        .label
        .clone();
    let cb = &mut abstract_function.cfg.basic_blocks[current_block_id];
    debug_stack.push(cbl.clone());

    log::trace!("rename stack: {:?}", debug_stack);

    // for every phi node in the current block
    for phi in &mut cb.phi_nodes {
        let var_name = &phi.dest;
        let count = counter
            .entry(var_name.to_string())
            .and_modify(|x| *x += 1)
            .or_default();

        let new_name = format!("{}_{}", var_name, count);

        stack
            .entry(var_name.to_string())
            .and_modify(|v| v.push(new_name.clone()))
            .or_insert(vec![new_name.clone()]);

        phi.dest = new_name;
        log::trace!("rename phi node: {}", phi);
    }

    // for every instruction in the current block
    //  1. replace argument to instruction with stack[old name]
    //  2. replace instruction's destination with a new name
    //  3. stack[old name: destination].push(new_name)
    for instruction in &mut cb.instructions {
        let instruction_arguments: Option<&Vec<String>> = instruction.get_arguments();

        log::trace!("before: {}", instruction);
        // --- step 1.
        if let Some(original_args) = instruction_arguments {
            let renamed_arguments = lookup_in_stack(original_args.into_iter(), stack);
            instruction.replace_arguments(renamed_arguments);
        }

        // --- step 2 & 3.
        if let Some(destination) = instruction.get_destination() {
            let count = counter
                .entry(destination.to_string())
                .and_modify(|x| *x += 1)
                .or_default();

            let new_name = format!("{}_{}", destination, count);

            stack
                .entry(destination.to_string())
                .and_modify(|v| v.push(new_name.clone()))
                .or_insert(vec![new_name.clone()]);

            instruction.replace_destination(new_name);
        }
        log::trace!("after:  {}", instruction);
    }

    // rename return
    if let Terminator::Ret(code) = &mut cb.terminator {
        if let Some(original_args) = code.get_arguments() {
            let renamed_arguments = lookup_in_stack(original_args.into_iter(), stack);
            code.replace_arguments(renamed_arguments);
        }
    }

    if let Terminator::Br(_, _, code) = &mut cb.terminator {
        if let Some(original_args) = code.get_arguments() {
            let renamed_arguments = lookup_in_stack(original_args.into_iter(), stack);
            code.replace_arguments(renamed_arguments);
        }
    }

    // rename branch

    // for s in the current block's successors
    // for ϕ in s's phi nodes
    // if ϕ is for a variable v, it will read from stack[v]

    for successor in abstract_function.cfg.successors[current_block_id].iter() {
        log::trace!("updating successor block {}", successor);
        let sb = &mut abstract_function.cfg.basic_blocks[*successor];
        for phi in &mut sb.phi_nodes {
            let var_name = phi.dest.as_str();
            let ori_name = phi.original_name.as_str();
            let stack_entry = stack.get(ori_name).expect(&format!(
                "Failed to find stack entry for variable '{}' in phi node for block '{}'",
                ori_name, sb.label
            ));
            let incoming_value = stack_entry
                .last()
                .expect(&format!(
                    "Failed to find last entry for variable {} in phi node",
                    var_name
                ))
                .to_string();
            phi.phi_args.push((incoming_value, cbl.clone()));
            log::trace!("update block {}: {} phi node: {}", sb.id, sb.label, phi);
        }
    }

    //   for b in blocks immediately dominated by block:
    //     # That is, children in the dominance tree.
    //     rename(b)
    let dominated = abstract_function
        .dominance_info
        .get_immediate_dominated(current_block_id)
        .into_iter()
        .copied()
        .collect::<Vec<BlockId>>();

    log::trace!(
        "block {}: {} dominates blocks {:?}",
        current_block_id,
        cbl,
        dominated
    );

    for b in dominated {
        let sbl = &abstract_function.cfg.basic_blocks[b].label;
        log::trace!("renaming dominated block {}: {}", b, sbl);
        rename(b, abstract_function, stack, counter, debug_stack);
    }

    // restore context
    *stack = stack_saved;
    debug_stack.pop();
}

pub fn insert_phi_nodes(mut af: AbstractFunction) -> WorklistResult<AbstractFunction> {
    // Perform liveness analysis which will return used variables in the future
    // Merge: union of all successors
    // Transfer:  merge result - kill(def) + use, iterating backwards
    let live_start = std::time::Instant::now();
    let liveness_result = run_dataflow_analysis::<LiveVariables>(&mut af)?;
    log::debug!("adding phi nodes for {}", af.name);
    log::debug!("lva took {:?}", live_start.elapsed());
    log::trace!("live variable analysis result: {:?}", liveness_result);

    let mut definition_queue: VecDeque<(BlockId, String)> = VecDeque::new();

    // first record all definitions
    for (idx, block) in af.cfg.basic_blocks.iter().enumerate() {
        for instruction in block.instructions.iter() {
            if let Some(destination) = instruction.get_destination() {
                definition_queue.push_back((idx, destination.to_string()));
            }
        }
    }

    // copy arguments in the preamble
    for var in af.args.iter().flatten() {
        definition_queue.push_back((0, var.name.to_string()));
        af.cfg.basic_blocks[0].instructions.insert(
            0,
            Code::Value {
                op: ValueOp::Id,
                dest: var.name.clone(),
                value_type: var.arg_type.clone(),
                args: Some(vec![var.name.clone()]),
                funcs: None,
                labels: None,
                pos: None,
            },
        );
    }

    // we will propagate reachable definitions R and insert a phi node
    //  1. In the current block if R is defined in the block && we are revisiting R (cycle)
    //  2. In the dominance frontier of the current block if R is live there (pruned SSA)

    // current block that defines R
    let mut inserted_phi_nodes: HashSet<(BlockId, String)> = HashSet::new();
    let mut seen: HashSet<(BlockId, String)> = HashSet::new();
    log::trace!("initial definition queue: {:?}", definition_queue);

    while !definition_queue.is_empty() {
        let definition = definition_queue.pop_front().unwrap();

        log::trace!("block {}: has assignment '{}'", definition.0, definition.1);

        if !seen.insert(definition.clone()) {
            log::trace!("\tskipping: already seen");
            continue;
        }

        let (definition_id, definition_ident) = definition.clone();

        for frontier_id in af.dominance_info.get_dominance_frontier(definition_id) {
            // if the variable is not live, we skip it
            log::trace!("\tchecking frontier block {}", frontier_id);
            if !liveness_result
                .get(frontier_id)
                .is_some_and(|(_, o)| o.contains(&definition_ident))
            {
                log::trace!("\t\tskipping: not live in frontier");
                continue;
            }

            if inserted_phi_nodes.insert((*frontier_id, definition_ident.clone())) {
                af.cfg.basic_blocks[*frontier_id]
                    .phi_nodes
                    .push(PhiNode::empty(definition_ident.clone()));
                definition_queue.push_back((*frontier_id, definition_ident.clone()));
            }
        }
    }

    for block in af.cfg.basic_blocks.iter() {
        for phi in block.phi_nodes.iter() {
            log::debug!("block {}: {} phi node: {}", block.id, block.label, phi);
        }
    }

    log::debug!("renaming variables");

    let mut stack: HashMap<String, Vec<String>> = HashMap::new();

    for var in af.args.iter().flatten() {
        stack
            .entry(var.name.clone())
            .or_insert_with(Vec::new)
            .push(var.name.clone());
    }

    // log::trace!("initial stack for {}: {:?}", abstract_function.name, stack);
    let mut assignment_counter: HashMap<String, usize> = HashMap::new();
    let mut debug_stack: Vec<String> = vec![];
    rename(
        0,
        &mut af,
        &mut stack,
        &mut assignment_counter,
        &mut debug_stack,
    );

    // run worklist top converge on types for phi nodes
    log::trace!("running type inference for phi nodes in {}", af.name);

    run_dataflow_analysis::<PhiTypeWorklist>(&mut af)?;

    Ok(af)
}

pub fn remove_phi_nodes(abstract_function: &mut AbstractFunction) {
    // let mut bb = abstract_function.basic_blocks;

    // let's very quickly build the mapping from label to basic block index
    let label_to_index = abstract_function
        .cfg
        .basic_blocks
        .iter()
        .enumerate()
        .map(|(idx, block)| (block.label.clone(), idx))
        .collect::<HashMap<String, usize>>();

    let mut phi_nodes = vec![];
    for block in &mut abstract_function.cfg.basic_blocks {
        // ok to take, clear out the phi nodes
        phi_nodes.extend(std::mem::take(&mut block.phi_nodes));
    }

    // for each phi node, push assignment into blocks with its labels
    for p in phi_nodes.into_iter() {
        // for each phi node, push assignment into blocks with its labels
        for (var, label) in p.phi_args {
            // for each phi node, push assignment into blocks with its labels
            let b_idx = label_to_index.get(&label).expect("should never be here");
            abstract_function.cfg.basic_blocks[*b_idx]
                .instructions
                .push(Code::Value {
                    op: ValueOp::Id,
                    dest: p.dest.clone(),
                    value_type: p.phi_type.clone(),
                    args: Some(vec![var]),
                    funcs: None,
                    labels: None,
                    pos: None,
                });
        }
    }
}
