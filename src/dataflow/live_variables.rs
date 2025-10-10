use std::collections::HashSet;

use crate::{
    dataflow::{WorklistProperty, WorklistResult},
    representation::{AbstractFunction, Argument, BasicBlock, BlockId, Code},
};

pub struct LiveVariables {}

impl WorklistProperty for LiveVariables {
    type Domain = HashSet<String>;

    fn init(_: usize, _: &AbstractFunction) -> Self::Domain {
        Self::Domain::default()
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
        block: &mut BasicBlock,
        _: Option<&Vec<Argument>>,
    ) -> WorklistResult<Self::Domain> {
        // iterate backwards through the instructions
        //      1. process definitions first (remove from live set)
        //      2. then process arguments (add to live set)

        let mut domain_view: HashSet<&str> = domain.iter().map(|s| s.as_str()).collect();

        match &block.terminator {
            crate::representation::Terminator::Ret(Code::Effect { args: Some(a), .. }) => {
                domain_view.extend(a.iter().map(|s| s.as_str()));
            }
            crate::representation::Terminator::Br(_, _, Code::Effect { args: Some(a), .. }) => {
                domain_view.extend(a.iter().map(|s| s.as_str()));
            }
            _ => (),
        }

        for instructions in block.instructions.iter().rev() {
            if let Some(dest) = instructions.get_destination() {
                log::trace!("    removing dest: {}", dest);
                domain_view.remove(dest);
            }
            if let Some(args) = instructions.get_arguments() {
                log::trace!("    adding args: {:?}", args);
                domain_view.extend(args.iter().map(|s| s.as_str()));
            }
            log::trace!("    instr: {:?}, live: {:?}", instructions, domain_view);
        }

        // phi nodes
        for phi in block.phi_nodes.iter() {
            domain_view.remove(phi.dest.as_str());
            for (var, _) in phi.phi_args.iter() {
                domain_view.insert(var.as_str());
            }
            log::trace!("    phi: {:?}, live: {:?}", phi, domain_view);
        }

        Ok(domain_view.into_iter().map(|s| s.to_string()).collect())
    }
}
