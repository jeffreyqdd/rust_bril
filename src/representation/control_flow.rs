use std::collections::{HashMap, HashSet};

use crate::representation::{BasicBlock, BlockId, Terminator};

/// module that represents control flow across basic blocks

#[derive(Debug, Clone)]
pub struct ControlFlowGraph {
    pub label_map: HashMap<String, BlockId>,
    pub successors: Vec<HashSet<usize>>,
    pub predecessors: Vec<HashSet<usize>>,
    pub basic_blocks: Vec<BasicBlock>,
}

impl From<Vec<BasicBlock>> for ControlFlowGraph {
    fn from(basic_blocks: Vec<BasicBlock>) -> Self {
        log::debug!("converting into cfg from {} blocks", basic_blocks.len());

        // construct label map
        let label_map: HashMap<String, BlockId> = basic_blocks
            .iter()
            .map(|block| (block.label.clone(), block.id))
            .collect();

        let mut successors: Vec<HashSet<usize>> = vec![HashSet::new(); basic_blocks.len()];
        let mut predecessors: Vec<HashSet<usize>> = vec![HashSet::new(); basic_blocks.len()];

        for block in &basic_blocks {
            let parent = block.id;
            let children = match &block.terminator {
                Terminator::Passthrough => vec![parent + 1],
                Terminator::Ret(_) => vec![],
                Terminator::Jmp(label, _) => vec![*label_map
                    .get(label)
                    .expect(&format!("label {} not found", label))],
                Terminator::Br(label1, label2, _) => vec![
                    *label_map
                        .get(label1)
                        .expect(&format!("label {} not found", label1)),
                    *label_map
                        .get(label2)
                        .expect(&format!("label {} not found", label2)),
                ],
            };

            for &child in &children {
                predecessors[child].insert(parent);
            }
            successors[parent].extend(children);
        }

        ControlFlowGraph {
            label_map,
            successors,
            predecessors,
            basic_blocks,
        }
    }
}

impl ControlFlowGraph {
    pub fn prune_unreachable_blocks(self) -> Self {
        let mut bb = self.basic_blocks;

        if bb.is_empty() {
            return ControlFlowGraph::from(bb);
        }

        let mut reachable = HashSet::new();
        let mut stack = vec![bb.first().unwrap().id];

        while let Some(block_id) = stack.pop() {
            if !reachable.insert(block_id) {
                continue;
            }

            for &succ in &self.successors[block_id] {
                stack.push(succ);
            }
        }
        let count_before = bb.len();
        bb.retain(|b| reachable.contains(&b.id));
        log::info!(
            "pruned {} unreachable blocks, {} remaining",
            count_before - bb.len(),
            bb.len()
        );

        // reassign basic block ids
        for (i, block) in bb.iter_mut().enumerate() {
            block.id = i;
        }

        ControlFlowGraph::from(bb)
    }
}
