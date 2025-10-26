use std::collections::{HashMap, HashSet};

use crate::{
    dataflow::{WorklistProperty, WorklistResult},
    representation::{AbstractFunction, Argument, BlockId, ControlFlowGraph},
};

pub struct ReachingDefinitions {}

impl WorklistProperty for ReachingDefinitions {
    /// In SSA form with phi nodes, we can simplify to track definitions more efficiently
    /// mapping from variable name to the set of block IDs where it is defined
    type Domain = HashMap<String, HashSet<usize>>;

    fn init(_: usize, _: &AbstractFunction) -> Self::Domain {
        Self::Domain::default()
    }

    fn is_forward() -> bool {
        true
    }

    fn merge(predecessors: Vec<(&BlockId, &Self::Domain)>) -> WorklistResult<Self::Domain> {
        let mut result: Self::Domain = HashMap::new();

        for (_, domain) in predecessors {
            for (var, defs) in domain.iter() {
                result
                    .entry(var.clone())
                    .or_insert_with(HashSet::new)
                    .extend(defs);
            }
        }

        Ok(result)
    }

    fn transfer(
        mut domain: Self::Domain,
        block_id: usize,
        cfg: &mut ControlFlowGraph,
        arguments: Option<&Vec<Argument>>,
    ) -> WorklistResult<Self::Domain> {
        // Handle function arguments in entry block
        if block_id == 0 {
            if let Some(args) = arguments {
                for arg in args {
                    let defs = domain.entry(arg.name.clone()).or_default();
                    defs.clear();
                    defs.insert(0);
                }
            }
        }

        // Process phi nodes first - they define variables at block entry
        for phi in &cfg.basic_blocks[block_id].phi_nodes {
            let defs = domain.entry(phi.dest.clone()).or_default();
            defs.clear();
            for (_var, label) in &phi.phi_args {
                defs.insert(cfg.label_map[label]);
            }
        }

        // Process regular instructions
        for instruction in &cfg.basic_blocks[block_id].instructions {
            if let Some(dest) = instruction.get_destination() {
                let defs = domain.entry(dest.to_string()).or_default();
                defs.clear();
                defs.insert(block_id);
            }
        }

        Ok(domain)
    }
}
