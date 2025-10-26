use crate::{
    dataflow::{run_dataflow_analysis, DefinitelyInitialized, WorklistResult},
    representation::{
        phi_nodes,
        program::{Code, EffectOp, Position, Type},
        Argument, ControlFlowGraph, DominanceInfo, Function, PhiNode, Program, RichProgram,
        ValueOp,
    },
};
use std::collections::{HashMap, HashSet};
use uuid::Uuid;

// Core types for the IR-friendly representation
pub type BlockId = usize;
pub type Variable = String;
pub type Label = String;

#[derive(Debug, Clone)]
pub struct RichAbstractProgram {
    pub original_text: Vec<String>,
    pub program: AbstractProgram,
}

#[derive(Debug, Clone)]
pub struct AbstractProgram {
    pub functions: HashMap<String, AbstractFunction>,
}

#[derive(Debug, Clone)]
pub struct AbstractFunction {
    pub name: String,
    pub pos: Option<Position>,
    pub cfg: ControlFlowGraph,
    pub dominance_info: DominanceInfo,
    pub args: Option<Vec<Argument>>,
    pub return_type: Option<Type>,
}

#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub label: String,
    pub instructions: Vec<Code>,
    pub terminator: Terminator,
    pub phi_nodes: Vec<PhiNode>,
    pub preheader: Vec<Code>,
    pub natural_loop_return: bool,
}

#[derive(Debug, Clone)]
pub enum Terminator {
    Passthrough,
    Ret(Code),
    Jmp(Label, Code),
    Br(Label, Label, Code),
}

impl Terminator {
    pub fn get_arguments(&self) -> Option<&Vec<String>> {
        match self {
            Terminator::Passthrough => None,
            Terminator::Ret(Code::Effect { args, .. }) => args.as_ref(),
            Terminator::Jmp(_, Code::Effect { args, .. }) => args.as_ref(),
            Terminator::Br(_, _, Code::Effect { args, .. }) => args.as_ref(),
            _ => None,
        }
    }
}

impl From<Function> for AbstractFunction {
    fn from(f: Function) -> Self {
        let now = std::time::Instant::now();
        let basic_blocks = AbstractFunction::into_basic_blocks(f.instrs);
        let cfg = ControlFlowGraph::from(basic_blocks).prune_unreachable_blocks();
        let dominance_info = DominanceInfo::from(&cfg);

        log::debug!("Converted {} into SSA in {:?}", f.name, now.elapsed());

        Self {
            name: f.name,
            pos: f.pos,
            cfg,
            dominance_info,
            args: f.args,
            return_type: f.return_type,
        }
    }
}

// Conversion implementations
impl From<RichProgram> for RichAbstractProgram {
    fn from(rp: RichProgram) -> Self {
        let now = std::time::Instant::now();

        // need to run initialized variable checker first
        let functions = rp
            .program
            .functions
            .into_iter()
            .map(|function| AbstractFunction::from(function))
            .map(
                // this map runs an initialized variable analysis on each function and exits on error
                |mut af| match run_dataflow_analysis::<DefinitelyInitialized>(&mut af) {
                    Ok(_) => af,
                    WorklistResult::Err(e) => e.error_with_context_then_exit(&rp.original_text),
                },
            )
            .map(|function| phi_nodes::insert_phi_nodes(function))
            .map(|result| match result {
                WorklistResult::Ok(func) => (func.name.clone(), func),
                WorklistResult::Err(e) => e.error_with_context_then_exit(&rp.original_text),
            })
            .collect();

        log::info!("converted program to SSA in {:?}", now.elapsed());
        RichAbstractProgram {
            original_text: rp.original_text,
            program: AbstractProgram { functions },
        }
    }
}

impl RichAbstractProgram {
    pub fn into_ssa_program(self) -> RichProgram {
        let functions = self
            .program
            .functions
            .into_values()
            .map(|f| f.remap_phi_nodes())
            .map(|f| f.into_ssa_function())
            .collect();

        RichProgram {
            original_text: self.original_text,
            program: Program { functions },
        }
    }

    pub fn into_program(self) -> RichProgram {
        let functions = self
            .program
            .functions
            .into_values()
            .map(|f| f.remap_phi_nodes())
            .map(|f| f.into_function())
            .collect();

        RichProgram {
            original_text: self.original_text,
            program: Program { functions },
        }
    }
}

impl AbstractFunction {
    fn emit_basic_block(
        block_id: &mut BlockId,
        current_block_instrs: &mut Vec<Code>,
        current_label: &mut Option<String>,
        current_terminator: &mut Terminator,
    ) -> BasicBlock {
        let block = BasicBlock {
            id: *block_id,
            label: current_label.take().unwrap_or_else(|| {
                format!("no_label_{}", Uuid::new_v4().to_string().replace("-", "_"))
            }),
            instructions: std::mem::take(current_block_instrs),
            terminator: std::mem::replace(current_terminator, Terminator::Passthrough),
            phi_nodes: Vec::new(),
            preheader: Vec::new(),
            natural_loop_return: false,
        };

        *block_id += 1;
        block
    }

    fn into_basic_blocks(instrs: Vec<Code>) -> Vec<BasicBlock> {
        let mut blocks = Vec::new();
        let mut current_block_instrs = Vec::new();
        let mut current_label: Option<String> = Some(format!(
            "function_preamble_{}",
            Uuid::new_v4().to_string().replace("-", "_")
        ));
        let mut block_id = 0;
        let mut current_terminator: Terminator = Terminator::Passthrough;

        // insert preamble block in case original first block needs to push values up
        blocks.push(AbstractFunction::emit_basic_block(
            &mut block_id,
            &mut current_block_instrs,
            &mut current_label,
            &mut current_terminator,
        ));

        for code in instrs {
            match &code {
                Code::Label { label, .. } => {
                    if !current_block_instrs.is_empty() || current_label.is_some() {
                        blocks.push(AbstractFunction::emit_basic_block(
                            &mut block_id,
                            &mut current_block_instrs,
                            &mut current_label,
                            &mut current_terminator,
                        ));
                    }

                    current_label = Some(label.clone());
                }
                Code::Effect {
                    op: op @ (EffectOp::Jmp | EffectOp::Br | EffectOp::Ret),
                    labels,
                    ..
                } => {
                    // This is a terminator instruction
                    current_terminator = match op {
                        EffectOp::Jmp => {
                            Terminator::Jmp(labels.clone().unwrap().pop().unwrap(), code)
                        }
                        EffectOp::Br => {
                            let mut v = labels.clone().unwrap();
                            Terminator::Br(v.remove(0), v.remove(0), code)
                        }
                        EffectOp::Ret => Terminator::Ret(code),
                        _ => unreachable!(),
                    };
                    blocks.push(AbstractFunction::emit_basic_block(
                        &mut block_id,
                        &mut current_block_instrs,
                        &mut current_label,
                        &mut current_terminator,
                    ));
                }
                _ => {
                    current_block_instrs.push(code);
                }
            }
        }

        // Handle any remaining instructions
        if !current_block_instrs.is_empty() || current_label.is_some() {
            current_terminator = Terminator::Ret(Code::Effect {
                op: EffectOp::Ret,
                args: None,
                labels: None,
                pos: None,
                funcs: None,
            });
            blocks.push(AbstractFunction::emit_basic_block(
                &mut block_id,
                &mut current_block_instrs,
                &mut current_label,
                &mut current_terminator,
            ));
        }

        blocks
    }

    fn flatten_basic_blocks(blocks: Vec<BasicBlock>) -> Vec<Code> {
        let mut instrs = Vec::new();

        let natural_loop_preheaders = blocks
            .iter()
            .filter_map(|block| {
                if block.preheader.len() > 0 {
                    Some(block.label.clone())
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        for block in blocks {
            // if this block has a natural loop preheader, emit it first
            if natural_loop_preheaders.contains(&block.label) {
                instrs.push(Code::Label {
                    label: format!("pre_header_{}", block.label),
                    pos: None,
                });
                for preheader_instr in block.preheader.iter() {
                    instrs.push(preheader_instr.clone());
                }
            }

            instrs.push(Code::Label {
                label: block.label,
                pos: None,
            });

            // add phi nodes
            for phi in block.phi_nodes.into_iter() {
                log::warn!("emitting phi node: {} [ignore if debugging SSA]", phi);

                // split phi.phi_args (tuple of (var, label)) into two vectors
                let (vars, labels): (Vec<_>, Vec<_>) = phi.phi_args.into_iter().unzip();

                instrs.push(Code::Value {
                    op: ValueOp::Phi,
                    dest: phi.dest,
                    value_type: phi.phi_type,
                    args: Some(vars),
                    funcs: None,
                    labels: Some(labels),
                    pos: None,
                });
            }

            // Add block instructions
            instrs.extend(block.instructions);

            // Helper function to map labels to preheaders when needed
            let map_label_to_preheader = |label: &str| -> String {
                if !block.natural_loop_return && natural_loop_preheaders.contains(label) {
                    format!("pre_header_{}", label)
                } else {
                    label.to_string()
                }
            };

            // Add terminator instruction if present
            match block.terminator {
                Terminator::Passthrough => continue,
                Terminator::Ret(effect_op) => instrs.push(effect_op),
                Terminator::Jmp(_, effect_op) => {
                    // if this is not a natural loop backedge and the target has a preheader, jump to the preheader instead
                    let dest_label = effect_op.get_labels().unwrap()[0].clone();
                    let mapped_label = map_label_to_preheader(&dest_label);
                    if mapped_label != dest_label {
                        instrs.push(Code::Effect {
                            op: EffectOp::Jmp,
                            args: None,
                            labels: Some(vec![mapped_label]),
                            pos: None,
                            funcs: None,
                        });
                    } else {
                        instrs.push(effect_op)
                    }
                }
                Terminator::Br(_, _, effect_op) => {
                    // if this is not a natural loop backedge and any target has a preheader, map to the preheader instead
                    let labels = effect_op.get_labels().unwrap();
                    let true_label = &labels[0];
                    let false_label = &labels[1];
                    let mapped_true_label = map_label_to_preheader(true_label);
                    let mapped_false_label = map_label_to_preheader(false_label);

                    if mapped_true_label != *true_label || mapped_false_label != *false_label {
                        instrs.push(Code::Effect {
                            op: EffectOp::Br,
                            args: effect_op.get_arguments().cloned(),
                            labels: Some(vec![mapped_true_label, mapped_false_label]),
                            pos: None,
                            funcs: None,
                        });
                    } else {
                        instrs.push(effect_op)
                    }
                }
            }
        }

        instrs
    }

    fn into_ssa_function(self) -> Function {
        let instrs = AbstractFunction::flatten_basic_blocks(self.cfg.basic_blocks);
        Function {
            name: self.name,
            pos: self.pos,
            instrs,
            args: self.args,
            return_type: self.return_type,
        }
    }

    fn into_function(mut self) -> Function {
        phi_nodes::remove_phi_nodes(&mut self);
        self.into_ssa_function()
    }

    fn remap_phi_nodes(mut self) -> Self {
        // only remap if not backedge
        let natural_loop_returns = self
            .cfg
            .basic_blocks
            .iter()
            .filter_map(|block| {
                if block.natural_loop_return {
                    Some(block.label.clone())
                } else {
                    None
                }
            })
            .collect::<HashSet<_>>();

        for block in &mut self.cfg.basic_blocks {
            if block.preheader.is_empty() {
                continue;
            }

            for phi_node in &mut block.phi_nodes {
                // Update phi_args to remap labels to preheaders when appropriate
                for (phi_var, phi_label) in &mut phi_node.phi_args {
                    // If this label corresponds to a natural loop with a preheader,
                    // remap it to point to the preheader instead
                    if block
                        .preheader
                        .iter()
                        .find(|instr| instr.get_destination() == Some(phi_var))
                        .is_some()
                        && !natural_loop_returns.contains(phi_label)
                    {
                        *phi_label = format!("pre_header_{}", block.label);
                    }
                }
            }
        }

        self
    }
}
