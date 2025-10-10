use std::collections::HashSet;

use crate::{
    dataflow::{WorklistError, WorklistProperty, WorklistResult},
    representation::{AbstractFunction, Argument, BasicBlock, BlockId, Code, Terminator},
};

/// A dataflow analysis to determine which variables are definitely initialized at each program point.
/// Will throw at any point there is a use of an uninitialized variable.
pub struct DefinitelyInitialized {}

impl WorklistProperty for DefinitelyInitialized {
    type Domain = HashSet<String>;

    fn init(block_id: usize, abstract_function: &AbstractFunction) -> Self::Domain {
        let mut top = HashSet::new();

        if block_id == 0 {
            return top;
        }

        if let Some(arguments) = abstract_function.args.as_ref() {
            for arg in arguments {
                top.insert(arg.name.clone());
            }
        }

        for b in abstract_function.cfg.basic_blocks.iter() {
            for instruction in b.instructions.iter() {
                if let Some(dest) = instruction.get_destination() {
                    top.insert(dest.to_string());
                }
            }
        }

        top
    }

    fn is_forward() -> bool {
        true
    }

    fn merge(predecessors: Vec<(&BlockId, &Self::Domain)>) -> WorklistResult<Self::Domain> {
        // all variables live in successor block are live going into this block
        if predecessors.is_empty() {
            return Ok(HashSet::new());
        }

        let mut iter = predecessors.into_iter();
        let first = iter.next().unwrap().1.clone();

        Ok(iter.fold(first, |mut acc, elem| {
            acc.retain(|x| elem.1.contains(x));
            acc
        }))
    }

    fn transfer(
        mut domain: Self::Domain,
        block: &mut BasicBlock,
        args: Option<&Vec<Argument>>,
    ) -> WorklistResult<Self::Domain> {
        if block.id == 0 {
            if let Some(arguments) = args {
                for arg in arguments {
                    domain.insert(arg.name.clone());
                }
            }
        }

        for instructions in block.instructions.iter() {
            if let Some(dest) = instructions.get_destination() {
                domain.insert(dest.to_string());
            }
        }
        Ok(domain)
    }

    fn should_run_final_check() -> bool {
        true
    }

    fn final_check(
        domain: &Self::Domain,
        block: &BasicBlock,
        args: Option<&Vec<Argument>>,
    ) -> WorklistResult<()> {
        let mut d = domain.clone();

        let args_in_domain = |args: &Vec<String>, domain: &HashSet<String>| -> Option<String> {
            for arg in args {
                if !domain.contains(arg) {
                    return Some(arg.clone());
                }
            }
            None
        };

        if block.id == 0 {
            if let Some(arguments) = args {
                for arg in arguments {
                    d.insert(arg.name.clone());
                }
            }
        }

        for instructions in block.instructions.iter() {
            if let Some(args) = instructions.get_arguments() {
                if let Some(var) = args_in_domain(&args, &d) {
                    return Err(WorklistError::transfer_error(
                        block,
                        format!("using uninitialized variable: {}", var),
                        &instructions.get_position(),
                    ));
                }
            }

            if let Some(dest) = instructions.get_destination() {
                d.insert(dest.to_string());
            }
        }

        match &block.terminator {
            Terminator::Ret(Code::Effect {
                args: Some(a), pos, ..
            }) => {
                if let Some(var) = args_in_domain(a, &d) {
                    return Err(WorklistError::transfer_error(
                        block,
                        format!("returning uninitialized variable: {}", var),
                        pos,
                    ));
                }
            }
            _ => (),
        }

        Ok(())
    }
}
