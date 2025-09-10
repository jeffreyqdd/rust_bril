use std::vec;

use crate::{blocks, program};

pub fn dce(mut cfg: blocks::CfgGraph) -> blocks::CfgGraph {
    // for each function iterate backwards and delete code that is
    // not referenced anywhere else

    let mut referenced_variables = std::collections::HashSet::new();
    for basic_block in &mut cfg.blocks {
        let mut new_basic_block = vec![];
        for (_idx, instruction) in basic_block.block.iter().rev().enumerate() {
            // println!("checking instruction {:?}", instruction);
            match instruction {
                program::Code::Label { .. } => new_basic_block.push(instruction.clone()),
                program::Code::Constant { dest, .. } => {
                    if referenced_variables.contains(dest) {
                        // only push if referenced
                        new_basic_block.push(instruction.clone());
                        // println!("pushing {:?}", instruction);
                    }
                }
                program::Code::Value { dest, args, .. } => {
                    if referenced_variables.contains(dest) {
                        for i in args.iter().flatten() {
                            referenced_variables.insert(i.clone());
                            // println!("referencing {:?}", i);
                        }

                        // only push if referenced
                        new_basic_block.push(instruction.clone());
                        // println!("pushing {:?}", instruction);
                    }
                }
                program::Code::Effect { args, .. } => {
                    for i in args.iter().flatten() {
                        referenced_variables.insert(i.clone());
                        // println!("referencing {:?}", i);
                    }

                    // push because effect operations have "side effects"
                    // println!("pushing {:?}", instruction);
                    new_basic_block.push(instruction.clone());
                }
            }
        }
        new_basic_block.reverse();
        basic_block.block = new_basic_block;
    }

    cfg
}
