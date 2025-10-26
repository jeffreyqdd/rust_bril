use std::{
    any::type_name,
    collections::{HashMap, HashSet, VecDeque},
};
use thiserror::Error;

use crate::representation::{
    AbstractFunction, Argument, BasicBlock, BlockId, ControlFlowGraph, Position,
};

/// Errors that can occur during worklist algorithm execution
#[derive(Error, Debug, Clone)]
pub enum WorklistError {
    #[error("Block not found: {reason}")]
    BlockNotFound { block_id: BlockId, reason: String },

    #[error("Transfer error: Block {block_id} ({block_label}) {reason}")]
    TransferFunctionError {
        block_id: BlockId,
        block_label: String,
        reason: String,
        position: Option<Position>,
        code_snippet: Option<String>,
    },

    #[error("Merge error: {reason}")]
    MergeFunctionError {
        inputs: Vec<BlockId>,
        reason: String,
        position: Option<Position>,
        code_snippet: Option<String>,
    },

    #[error("Analysis convergence failed: reached maximum iterations ({max_iterations}) at function {function_name}")]
    ConvergenceError {
        function_name: String,
        max_iterations: usize,
    },
}

impl WorklistError {
    /// Create a new BlockNotFound error with position info
    pub fn block_not_found(block_id: BlockId, reason: impl Into<String>) -> Self {
        Self::BlockNotFound {
            block_id,
            reason: reason.into(),
        }
    }

    /// Create a new TransferFunctionError with position info
    pub fn transfer_error(
        block: &BasicBlock,
        reason: impl Into<String>,
        position: &Option<Position>,
    ) -> Self {
        Self::TransferFunctionError {
            block_id: block.id,
            block_label: block.label.clone(),
            reason: reason.into(),
            position: *position,
            code_snippet: None,
        }
    }

    /// Create a new MergeFunctionError with position info
    pub fn merge_error(
        inputs: Vec<BlockId>,
        reason: impl Into<String>,
        position: Option<Position>,
    ) -> Self {
        Self::MergeFunctionError {
            inputs,
            reason: reason.into(),
            position,
            code_snippet: None,
        }
    }

    /// Get the position information if available
    pub fn position(&self) -> Option<&Position> {
        match self {
            Self::TransferFunctionError { position, .. }
            | Self::MergeFunctionError { position, .. } => position.as_ref(),
            Self::BlockNotFound { .. } | Self::ConvergenceError { .. } => None,
        }
    }

    /// Get the block ID associated with this error if available
    pub fn block_id(&self) -> Option<Vec<BlockId>> {
        match self {
            Self::BlockNotFound { block_id, .. } | Self::TransferFunctionError { block_id, .. } => {
                Some(vec![*block_id])
            }
            Self::MergeFunctionError { inputs, .. } => Some(inputs.clone()),
            Self::ConvergenceError { .. } => None,
        }
    }

    pub fn error_with_context_then_exit(&self, text: &Vec<String>) -> ! {
        eprintln!("{}", self);
        if let Some(pos) = self.position() {
            let line = pos.row as usize;
            let column = pos.col as usize;

            let lines: &Vec<String> = text;
            let context_lines = 10; // Show 10 lines before and after the error

            let start_line = line.saturating_sub(context_lines + 1); // -1 because line numbers are 1-based
            let end_line = (line + context_lines).min(lines.len());

            let mut snippet = String::new();
            for (i, line_content) in lines[start_line..end_line].iter().enumerate() {
                let line_num = start_line + i + 1;
                let marker = if line_num == line { ">>> " } else { "    " };
                // row pointer
                snippet.push_str(&format!("{}{:3}: {}\n", marker, line_num, line_content));
                // col pointer
                if line_num == line && column > 0 && line <= lines.len() {
                    let pointer = format!(">>>      {}^\n", " ".repeat(column));
                    snippet.push_str(&pointer);
                }
            }
            eprintln!("Error context:\n{}", snippet);
        }
        std::process::exit(1);
    }
}

pub type WorklistResult<T> = Result<T, WorklistError>;

struct WorklistAlgorithm<'a> {
    abstract_function: &'a mut AbstractFunction,
    max_iterations: usize,
}

pub trait WorklistProperty {
    type Domain: Clone + PartialEq + Eq + std::fmt::Debug;
    fn init(block_id: usize, abstract_function: &AbstractFunction) -> Self::Domain;
    fn is_forward() -> bool;
    fn merge(predecessors: Vec<(&BlockId, &Self::Domain)>) -> WorklistResult<Self::Domain>;
    fn transfer(
        domain: Self::Domain,
        block_id: usize,
        cfg: &mut ControlFlowGraph,
        args: Option<&Vec<Argument>>,
    ) -> WorklistResult<Self::Domain>;

    /// run final pass after analysis converges to assert some property
    fn should_run_final_check() -> bool {
        false
    }

    fn final_check(
        _domain: &Self::Domain,
        _block: &BasicBlock,
        _args: Option<&Vec<Argument>>,
    ) -> WorklistResult<()> {
        Ok(())
    }
}

impl<'a> WorklistAlgorithm<'a> {
    fn from(abstract_function: &'a mut AbstractFunction) -> Self {
        Self {
            abstract_function,
            max_iterations: 10_000,
        }
    }

    #[inline]
    /// Get the inputs into the basic block from the specified direction (predecessors if forward, successors if backward)
    fn edges(&self, block_label: &BlockId, forward: bool) -> WorklistResult<&HashSet<usize>> {
        let cfg = &self.abstract_function.cfg;
        if forward {
            cfg.predecessors
                .get(*block_label)
                .ok_or_else(|| WorklistError::BlockNotFound {
                    reason: format!(
                        "block id {} not in function {}",
                        block_label, self.abstract_function.name
                    ),
                    block_id: *block_label,
                })
        } else {
            cfg.successors
                .get(*block_label)
                .ok_or_else(|| WorklistError::BlockNotFound {
                    reason: format!(
                        "block id {} not in function {}",
                        block_label, self.abstract_function.name
                    ),
                    block_id: *block_label,
                })
        }
    }
    fn run_worklist<T: WorklistProperty>(
        &mut self,
    ) -> WorklistResult<HashMap<BlockId, (T::Domain, T::Domain)>> {
        let mut worklist: VecDeque<usize> = self
            .abstract_function
            .cfg
            .basic_blocks
            .iter()
            .map(|b| b.id)
            .collect();

        let forward = T::is_forward();
        let mut num_it = 0;
        let mut result: HashMap<BlockId, (T::Domain, T::Domain)> =
            (0..self.abstract_function.cfg.basic_blocks.len())
                .map(|i| {
                    let init = T::init(i, self.abstract_function);
                    (i, (init.clone(), init))
                })
                .collect();
        log::trace!("{}: worklist={:?}", type_name::<T>(), worklist);
        while let Some(cur) = { worklist.pop_front() } {
            if num_it >= self.max_iterations {
                return Err(WorklistError::ConvergenceError {
                    function_name: self.abstract_function.name.clone(),
                    max_iterations: self.max_iterations,
                });
            }

            let block_name = &self.abstract_function.cfg.basic_blocks[cur].label;
            log::trace!("it {:<4}: visiting block {}: {}", num_it, cur, block_name);

            let inputs: Vec<(&BlockId, &T::Domain)> = self
                .edges(&cur, forward)?
                .into_iter()
                .filter_map(|b| result.get(b).map(|(_, o)| (b, o)))
                .collect();
            let in_ = T::merge(inputs)?;
            let out = T::transfer(
                in_.clone(),
                cur,
                &mut self.abstract_function.cfg,
                self.abstract_function.args.as_ref(),
            )?;
            let is_same = result.get(&cur).is_some_and(|(_, o)| *o == out);
            result.insert(cur, (in_, out));

            if !is_same {
                // push successor blocks if first time or output changed
                // negate to get "children" instead of "parents"
                worklist.extend(self.edges(&cur, !forward)?.into_iter().map(|x| *x));
            }

            num_it += 1;
        }

        if T::should_run_final_check() {
            for block in &self.abstract_function.cfg.basic_blocks {
                if let Some((in_, _)) = result.get(&block.id) {
                    T::final_check(in_, block, self.abstract_function.args.as_ref())?;
                }
            }
        }

        Ok(result)
    }
}

pub fn run_dataflow_analysis<T>(
    abstract_function: &mut AbstractFunction,
) -> WorklistResult<HashMap<BlockId, (T::Domain, T::Domain)>>
where
    T: WorklistProperty,
{
    let result = {
        let mut algorithm: WorklistAlgorithm = WorklistAlgorithm::from(abstract_function);
        algorithm.run_worklist::<T>()?
    };

    Ok(result)
}
