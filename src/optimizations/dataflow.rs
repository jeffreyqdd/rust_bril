use std::collections::{HashMap, VecDeque};

use crate::blocks::{BasicBlock, CfgGraph};
use crate::optimizations::dataflow_properties::WorklistProperty;

pub struct WorklistAlgorithm<T>
where
    T: WorklistProperty,
{
    worklist_property: T,

    /// mapping from block UID to its output domain
    output: HashMap<String, T::Domain>,

    cfg: CfgGraph,
}

pub struct DataflowResult<T> {
    label_name: String,
    input: T,
    output: T,
}

impl<T: WorklistProperty> WorklistAlgorithm<T> {
    fn new(worklist_property: T, cfg: &CfgGraph) -> Self {
        let mut output = HashMap::new();
        for block in &cfg.function.basic_blocks {
            output.insert(block.label.clone(), T::init());
        }

        Self {
            worklist_property,
            output,
            cfg: cfg.clone(),
        }
    }

    /// returns (Vector of data flow results with input and output mapped to T::Domain where T implements Worklist Property)
    fn run_worklist(&mut self) -> Vec<DataflowResult<T::Domain>> {
        let mut worklist: VecDeque<&BasicBlock> = VecDeque::new();
        let mut result: HashMap<String, (T::Domain, T::Domain)> = HashMap::new();
        worklist.push_back(&self.cfg.function.basic_blocks[0]);

        while let Some(current_block) = worklist.pop_front() {
            // get every predecessor of top
            let input_blocks = if self.worklist_property.is_forward() {
                // For the forward direction, the predecessor are the ancestors
                self.cfg
                    .predecessors(&current_block.label)
                    .expect("this means cfg was not constructed properly")
            } else {
                // For the backward direction, the predecessor are the children
                self.cfg
                    .successor(&current_block.label)
                    .expect("this means cfg was not constructed properly")
            };

            let input_domain = input_blocks
                .into_iter()
                .map(|block_name| {
                    self.output
                        .get(&block_name.label)
                        .expect("should exist, even if it is bottom variant")
                        .clone()
                })
                .collect();

            let merged_input_domain = self.worklist_property.merge(&input_domain);
            let output_domain = self
                .worklist_property
                .transfer(merged_input_domain.clone(), current_block);

            result.insert(
                current_block.label.clone(),
                (merged_input_domain, output_domain.clone()),
            );

            // if out changed, worklist += successors
            let changed = self.output.get(&current_block.label) != Some(&output_domain);
            if changed {
                // update
                self.output
                    .insert(current_block.label.clone(), output_domain);

                // search successors
                let sucessor_blocks = if self.worklist_property.is_forward() {
                    // For the forward direction, the successor are the children
                    self.cfg
                        .successor(&current_block.label)
                        .expect("this means cfg was not constructed properly")
                } else {
                    // For the backward direction, the successor are the ancestors
                    self.cfg
                        .predecessors(&current_block.label)
                        .expect("this means cfg was not constructed properly")
                };

                for i in sucessor_blocks {
                    worklist.push_back(i);
                }
            }
        }

        let mut ret = Vec::new();
        for code in &self.cfg.function.basic_blocks {
            let (i, o) = result.get(&code.label).expect("should never happen");
            ret.push(DataflowResult {
                label_name: code.label.clone(),
                input: i.clone(),
                output: o.clone(),
            });
        }

        ret
    }
}

pub fn run_dataflow_analysis<T: WorklistProperty>(cfg: CfgGraph, worklist_variant: T) {
    let mut algorithm = WorklistAlgorithm::new(worklist_variant, &cfg);
    let result = algorithm.run_worklist();
    for i in result {
        println!("{}:", i.label_name);
        println!("\tin: {:?}", T::deterministic_printable_medium(&i.input));
        println!("\tout: {:?}", T::deterministic_printable_medium(&i.output));
    }
}
