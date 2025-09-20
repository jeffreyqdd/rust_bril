use std::collections::{HashMap, VecDeque};

use crate::blocks::{BasicBlock, CfgGraph};
use crate::optimizations::dataflow_properties::WorklistProperty;

pub struct WorklistAlgorithm<T: WorklistProperty> {
    worklist_property: T,
    cfg: CfgGraph,
}

pub struct DataflowResult<T> {
    pub label_name: String,
    pub input: T,
    pub output: T,
}

impl<T: WorklistProperty> WorklistAlgorithm<T> {
    fn new(worklist_property: T, cfg: &CfgGraph) -> Self {
        Self {
            worklist_property,
            cfg: cfg.clone(),
        }
    }
    #[inline]
    /// Get the inputs into the basic block from the specified direction (predecessors if forward, successors if backward)
    fn edges<'a>(&'a self, b: &'a BasicBlock, forward: bool) -> Vec<&'a BasicBlock> {
        if forward {
            self.cfg.predecessors(&b.label)
        } else {
            self.cfg.successor(&b.label)
        }
        .expect("cfg was not constructed properly (missing block; empty list is expected for no successors or predecessors)")
    }

    /// returns (Vector of data flow results with input and output mapped to T::Domain where T implements Worklist Property)
    fn run_worklist(&mut self) -> Vec<DataflowResult<Vec<String>>> {
        let mut worklist: VecDeque<&BasicBlock> = self.cfg.function.basic_blocks.iter().collect();
        let mut result: HashMap<String, (T::Domain, T::Domain)> = HashMap::new();
        let forward = self.worklist_property.is_forward();

        while let Some(cur) = { worklist.pop_front() } {
            let inputs: Vec<T::Domain> = self
                .edges(cur, forward)
                .into_iter()
                .filter_map(|b| result.get(&b.label).map(|(_, o)| o.clone()))
                .collect();

            let in_ = self.worklist_property.merge(&inputs);
            let out = self.worklist_property.transfer(&in_, cur);
            let before = result.insert(cur.label.clone(), (in_, out.clone()));

            // push successor blocks if first time or output changed
            if before.as_ref().map(|(_, o)| o) != Some(&out) {
                // negate to get "children" instead of "parents"
                worklist.extend(self.edges(cur, !forward));
            }
        }

        // consolidate the results into a vector where the labels appear in the same order as the basic block vector
        self.cfg
            .function
            .basic_blocks
            .iter()
            .map(|b| {
                let (i, o) = result.get(&b.label).expect("missing dataflow result");
                DataflowResult {
                    label_name: b.label.clone(),
                    input: T::deterministic_array(i),
                    output: T::deterministic_array(o),
                }
            })
            .collect()
    }
}

pub fn run_dataflow_analysis<T: WorklistProperty>(
    cfg: CfgGraph,
    worklist_variant: T,
) -> Vec<DataflowResult<Vec<String>>> {
    let mut algorithm = WorklistAlgorithm::new(worklist_variant, &cfg);
    let result = algorithm.run_worklist();
    return result;
}
