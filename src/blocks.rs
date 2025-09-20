use std::{collections::HashMap, fs::File};

use crate::program::{Argument, Code, EffectOp, Function, Program, Type};
use serde;
use serde_json;

/// Chunk program into basic block

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum Terminator {
    Passthrough,
    Ret,
    Jmp(String),
    Br(String, String),
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct BasicBlock {
    pub label: String,
    pub block: Vec<Code>,
    pub terminator: Terminator,
    pub external_references: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionBlock {
    pub name: String,
    pub args: Option<Vec<Argument>>,
    pub return_type: Option<Type>,
    pub basic_blocks: Vec<BasicBlock>,
}

impl BasicBlock {
    fn new(label: String, block: Vec<Code>, terminator: Terminator) -> Self {
        // since we know all the code that's going in the basic block, we can find
        // variables that do not have an assignment and put that in the list of
        // external_references
        let mut declared_variables = std::collections::HashSet::new();
        let mut external_references = Vec::new();
        for code in &block {
            match code {
                Code::Noop { .. } => continue,
                Code::Label { .. } => continue,
                Code::Constant { dest, .. } => {
                    declared_variables.insert(dest.clone());
                }
                Code::Value { dest, args, .. } => {
                    args.iter()
                        .flatten()
                        .filter(|v| !declared_variables.contains(*v))
                        .for_each(|v| external_references.push(v.clone()));
                    declared_variables.insert(dest.clone());
                }
                Code::Effect { args, .. } => args
                    .iter()
                    .flatten()
                    .filter(|v| !declared_variables.contains(*v))
                    .for_each(|v| external_references.push(v.clone())),
                Code::Memory { args, dest, .. } => {
                    args.iter()
                        .flatten()
                        .filter(|v| !declared_variables.contains(*v))
                        .for_each(|v| external_references.push(v.clone()));
                    if let Some(d) = dest.as_ref() {
                        declared_variables.insert(d.clone());
                    }
                }
            }
        }

        Self {
            label,
            block,
            terminator,
            external_references: external_references,
        }
    }
}

impl Program {
    /// for each function, chunk it into basic blocks
    pub fn basic_blocks(&self) -> Vec<FunctionBlock> {
        let mut ret = Vec::new();

        for function in &self.functions {
            let mut basic_block = Vec::new();
            let mut curr_section = String::new();
            let mut curr_block = Vec::new();

            for code in &function.instrs {
                match code {
                    Code::Label { label, position: _ } => {
                        if curr_block.len() == 0 {
                            curr_section = label.clone();
                            curr_block.push(code.clone());
                            continue;
                        }
                        // if section does not have a label, it gets "no_label"
                        // TODO: probably replace with with some less jank name-mangling
                        let l = if curr_section.is_empty() {
                            format!("no_label_{}", uuid::Uuid::new_v4())
                        } else {
                            curr_section
                        };

                        let b = BasicBlock::new(l, curr_block, Terminator::Passthrough);

                        basic_block.push(b);

                        curr_block = Vec::new();
                        curr_block.push(code.clone());
                        curr_section = label.clone();
                    }
                    Code::Effect {
                        op: op @ (EffectOp::Jmp | EffectOp::Br | EffectOp::Ret),
                        labels,
                        ..
                    } => {
                        let l = if curr_section.is_empty() {
                            format!("no_label_{}", uuid::Uuid::new_v4())
                        } else {
                            curr_section
                        };
                        // TODO: this is arguable very bad
                        // rc<Vec<String>> might be the better pattern
                        let v = labels.clone().unwrap_or_else(|| Vec::new());

                        let t = match op {
                            EffectOp::Jmp => Terminator::Jmp(v[0].clone()),
                            EffectOp::Br => Terminator::Br(v[0].clone(), v[1].clone()),
                            EffectOp::Ret => Terminator::Ret,
                            _ => panic!("should never be here because op is constrained"),
                        };

                        curr_block.push(code.clone());
                        basic_block.push(BasicBlock::new(l, curr_block, t));

                        curr_block = Vec::new();
                        curr_section = String::new();
                    }
                    _ => {
                        curr_block.push(code.clone());
                    }
                }
            }

            if curr_block.len() > 0 {
                // if section does not have a
                let l = if curr_section.is_empty() {
                    format!("no_label_{}", uuid::Uuid::new_v4())
                } else {
                    curr_section
                };

                let b = BasicBlock::new(l, curr_block, Terminator::Passthrough);

                basic_block.push(b);
            }

            ret.push(FunctionBlock {
                name: function.name.clone(),
                args: function.args.clone(),
                return_type: function.return_type.clone(),
                basic_blocks: basic_block,
            });
        }

        ret
    }

    pub fn from(cfg: Vec<CfgGraph>) -> Self {
        Program {
            functions: cfg.into_iter().map(|x| x.into_function()).collect(),
        }
    }
}

#[allow(dead_code)]
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CfgGraph {
    pub function: FunctionBlock,
    pub edges: Vec<Vec<usize>>, // edges[i] = successors of block i
    pub predecessors: Vec<Vec<usize>>,
    pub label_map: HashMap<String, usize>, // map label -> block index
}

impl CfgGraph {
    pub fn from(function_block: &FunctionBlock) -> Self {
        let mut label_map = HashMap::new();
        for (i, block) in function_block.basic_blocks.iter().enumerate() {
            label_map.insert(block.label.clone(), i);
        }

        let mut edges = vec![Vec::new(); function_block.basic_blocks.len()];
        for (i, block) in function_block.basic_blocks.iter().enumerate() {
            match &block.terminator {
                Terminator::Passthrough => {
                    // if not the end block, connect to next block
                    if i < function_block.basic_blocks.len() - 1 {
                        edges[i].push(i + 1);
                    }
                }
                Terminator::Jmp(dest) => {
                    if let Some(&j) = label_map.get(dest) {
                        edges[i].push(j);
                    }
                }
                Terminator::Br(dest1, dest2) => {
                    if let Some(&j) = label_map.get(dest1) {
                        edges[i].push(j);
                    }
                    if let Some(&j) = label_map.get(dest2) {
                        edges[i].push(j);
                    }
                }
                // noop
                Terminator::Ret => {}
            }
        }

        let mut predecessors = vec![Vec::new(); edges.len()];

        for (from, successors) in edges.iter().enumerate() {
            for &to in successors {
                // Format the predecessor reference as "b{from}"
                predecessors[to].push(from);
            }
        }

        CfgGraph {
            function: function_block.clone(),
            edges,
            predecessors,
            label_map,
        }
    }

    pub fn into_function(self) -> Function {
        Function {
            name: self.function.name,
            args: self.function.args,
            return_type: self.function.return_type,
            instrs: self
                .function
                .basic_blocks
                .into_iter()
                .map(|x| x.block)
                .flatten()
                .collect(),
        }
    }

    /// None variable if node dne
    pub fn predecessors(&self, node: &str) -> Option<Vec<&BasicBlock>> {
        let id = self.label_map.get(node);

        if let Some(id) = id {
            let ret = self.predecessors[*id]
                .iter()
                .map(|u| &self.function.basic_blocks[*u])
                .collect::<Vec<&BasicBlock>>();
            Some(ret)
        } else {
            None
        }
    }

    /// None variable if node dne
    pub fn successor(&self, node: &str) -> Option<Vec<&BasicBlock>> {
        let id = self.label_map.get(node);

        if let Some(id) = id {
            let ret = self.edges[*id]
                .iter()
                .map(|u| &self.function.basic_blocks[*u])
                .collect::<Vec<&BasicBlock>>();
            Some(ret)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    pub fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }

    #[allow(dead_code)]
    pub fn to_file(&self, file_path: &str) {
        let file = File::create(file_path).unwrap();
        serde_json::to_writer_pretty(file, self).unwrap();
    }
}
