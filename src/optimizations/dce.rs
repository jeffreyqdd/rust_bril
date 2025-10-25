/// Module for dead code elimination, make sure to run after local variable numbering
use std::{collections::HashSet, vec};

use crate::{
    dataflow::{run_dataflow_analysis, WorklistProperty, WorklistResult},
    representation::{AbstractFunction, BlockId, Code, ControlFlowGraph, Terminator},
};

// iterating until all variables are referenced
struct Dce {}

impl WorklistProperty for Dce {
    // the set of variables that are referenced in the future
    type Domain = HashSet<String>;

    fn init(_: usize, af: &AbstractFunction) -> Self::Domain {
        let mut top = HashSet::new();

        if let Some(arguments) = af.args.as_ref() {
            for arg in arguments {
                top.insert(arg.name.clone());
            }
        }

        for b in af.cfg.basic_blocks.iter() {
            for instruction in b.instructions.iter() {
                if let Some(dest) = instruction.get_destination() {
                    top.insert(dest.to_string());
                }
            }

            for phi in b.phi_nodes.iter() {
                top.insert(phi.dest.clone());
            }
        }

        top
    }

    fn is_forward() -> bool {
        false
    }

    fn merge(predecessors: Vec<(&BlockId, &Self::Domain)>) -> WorklistResult<Self::Domain> {
        // all variables live in successor block are live going into this block
        if predecessors.is_empty() {
            return Ok(HashSet::new());
        }

        let mut iter = predecessors.into_iter();
        let first = iter.next().unwrap().1.clone();

        Ok(iter.fold(first, |mut acc, elem| {
            acc.extend(elem.1.iter().cloned());
            acc
        }))
    }

    fn transfer(
        domain: Self::Domain,
        block_id: usize,
        cfg: &mut ControlFlowGraph,
        _: Option<&Vec<crate::representation::Argument>>,
    ) -> WorklistResult<Self::Domain> {
        // iterate backwards through the instructions
        //      1. process definitions first (remove from live set)
        //      2. then process arguments (add to live set)

        let block = &mut cfg.basic_blocks[block_id];
        let mut domain_view: HashSet<&str> = domain.iter().map(|s| s.as_str()).collect();

        match &block.terminator {
            Terminator::Ret(Code::Effect { args: Some(a), .. }) => {
                domain_view.extend(a.iter().map(|s| s.as_str()));
            }
            Terminator::Br(_, _, Code::Effect { args: Some(a), .. }) => {
                domain_view.extend(a.iter().map(|s| s.as_str()));
            }
            _ => (),
        }

        let mut new_instructions = vec![];
        for instructions in block.instructions.iter().rev() {
            if let Some(dest) = instructions.get_destination() {
                if !domain_view.contains(dest) {
                    continue;
                }
            }

            if let Some(dest) = instructions.get_destination() {
                domain_view.remove(dest);
            }

            if let Some(args) = instructions.get_arguments() {
                domain_view.extend(args.iter().map(|s| s.as_str()));
            }

            new_instructions.push(instructions.clone());
        }

        // phi nodes
        let mut new_phi = vec![];
        for phi in block.phi_nodes.iter() {
            if !domain_view.contains(phi.dest.as_str()) {
                continue;
            }

            domain_view.remove(phi.dest.as_str());

            for (var, _) in phi.phi_args.iter() {
                domain_view.insert(var.as_str());
            }

            new_phi.push(phi.clone());
        }

        new_phi.reverse();
        new_instructions.reverse();

        let ret = Ok(domain_view.into_iter().map(|s| s.to_string()).collect());
        block.phi_nodes = new_phi;
        block.instructions = new_instructions;
        return ret;
    }
}

pub fn dce(mut af: AbstractFunction) -> WorklistResult<AbstractFunction> {
    log::info!("running DCE on function {}", af.name);
    run_dataflow_analysis::<Dce>(&mut af)?;
    Ok(af)
}
