use std::collections::{HashMap, VecDeque};

use crate::blocks::{BasicBlock, CfgGraph};
use crate::optimizations::dataflow_properties::WorklistProperty;

pub struct WorklistAlgorithm<T>
where
    T: WorklistProperty,
{
    worklist_property: T,
    cfg: CfgGraph,
}

pub struct DataflowResult<T> {
    label_name: String,
    input: T,
    output: T,
}

impl<T: WorklistProperty> WorklistAlgorithm<T> {
    fn new(worklist_property: T, cfg: &CfgGraph) -> Self {
        Self {
            worklist_property,
            cfg: cfg.clone(),
        }
    }

    fn append_successors<'a>(
        &'a self,
        worklist: &mut VecDeque<&'a BasicBlock>,
        current_block: &'a BasicBlock,
    ) {
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

        // println!(
        //     "\t output changed: pushing successors {:?}",
        //     sucessor_blocks
        //         .iter()
        //         .map(|b| &b.label)
        //         .collect::<Vec<&String>>()
        // );

        for i in sucessor_blocks {
            worklist.push_back(i);
        }
    }

    /// returns (Vector of data flow results with input and output mapped to T::Domain where T implements Worklist Property)
    fn run_worklist(&mut self) -> Vec<DataflowResult<T::Domain>> {
        let mut worklist: VecDeque<&BasicBlock> = VecDeque::new();
        let mut result: HashMap<String, (T::Domain, T::Domain)> = HashMap::new();

        for b in self.cfg.function.basic_blocks.iter() {
            worklist.push_back(b);
        }

        loop {
            let current_block = match worklist.pop_front() {
                Some(b) => b,
                None => break,
            };

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
                .filter_map(|block_name| {
                    let data = result.get(&block_name.label);
                    match data {
                        Some((_input, output)) => Some(output.clone()),
                        None => None,
                    }
                })
                .collect();

            let merged_input_domain = self.worklist_property.merge(&input_domain);
            let output_domain = self
                .worklist_property
                .transfer(&merged_input_domain, current_block);

            // if out changed, worklist += successors
            let before = result.insert(
                current_block.label.clone(),
                (merged_input_domain, output_domain.clone()),
            );

            match before {
                None => {
                    self.append_successors(&mut worklist, current_block);
                }
                Some((_, old_output)) if old_output != output_domain => {
                    self.append_successors(&mut worklist, current_block);
                }
                _ => {} // neither none, nor changed, so do nothing
            }
        }

        // consolidate the results into a vector where the labels appear in the same order as the basic block vector
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
