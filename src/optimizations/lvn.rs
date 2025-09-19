/// Module for local value numbering, Capable of copy propagation, cse, and const expression folding
use std::collections::HashMap;
use std::vec::Vec;
use uuid::Uuid;

use crate::program::{Code, ConstantOp, EffectOp, Literal, MemoryOp, Type, ValueOp};

pub fn lvn(mut cfg: crate::blocks::CfgGraph) -> crate::blocks::CfgGraph {
    cfg.function.basic_blocks = cfg
        .function
        .basic_blocks
        .into_iter()
        .map(|block| lvn_on_block(block))
        .collect();

    cfg
}

/// Mangling is a mapping from variables to mangled variables and vice versa
/// The reason for mangling is to avoid collisions when there are multiple declarations of the same variable name
/// This workaround also allows us to preserve the original variable name for later use and to not break other blocks and/or optimizations.
#[derive(Debug)]
struct Mangling {
    variable_to_mangled: HashMap<String, String>,
    mangled_to_variable: HashMap<String, String>,
}

impl Mangling {
    fn new() -> Self {
        Self {
            variable_to_mangled: HashMap::new(),
            mangled_to_variable: HashMap::new(),
        }
    }

    /// Check if a variable exists in the mangling table
    fn exists(&self, variable: &str) -> bool {
        self.variable_to_mangled.contains_key(variable)
    }

    /// Mangle a variable by adding a random UUID to the end of the variable name
    fn mangle(&mut self, variable: &str) -> &String {
        // Remove old mappings if they exist
        if let Some(old_mangled) = self.variable_to_mangled.remove(variable) {
            self.mangled_to_variable.remove(&old_mangled);
        }

        // Create new mangled name
        let mangled = format!("_{}_{}", variable, Uuid::new_v4());

        // Insert both directions
        self.variable_to_mangled
            .insert(variable.to_owned(), mangled.clone());
        self.mangled_to_variable
            .insert(mangled, variable.to_owned());

        self.variable_to_mangled.get(variable).unwrap() // just inserted, so it's fine
    }

    /// Get the mangled variable for a given variable
    fn to_mangled(&mut self, variable: &str) -> &String {
        if !self.exists(variable) {
            self.mangle(variable);
        }
        self.variable_to_mangled.get(variable).unwrap()
    }

    /// Get the original variable for a given mangled variable
    fn from_mangled(&self, mangled: &str) -> Option<&String> {
        self.mangled_to_variable.get(mangled)
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
/// Wrap operation in a unified enum
///
/// NOTE: EffectOperations should never be constructed since only their args need to be reprojected.
/// They do not create any new variables that we should keep track of.
enum Operation {
    Value(ValueOp),
    Memory(MemoryOp),
    #[allow(dead_code)]
    Effect(EffectOp),
    #[allow(dead_code)]
    Constant(ConstantOp),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum Expr {
    /// destination type
    ConstExpr(Type, Literal),

    /// destination type
    Expr(Type, Operation, Vec<usize>),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum CanonicalHome {
    ConstExpr(Type, Literal, String),
    Variable(String),
}

trait SemanticalReasononing: std::fmt::Debug {
    fn is_commutative(&self, operation: &Operation) -> bool;
    fn is_copy(&self, operation: &Operation) -> bool;
    fn can_constexpr(&self, operation: &Operation) -> bool;
    fn eval_constexpr(&self, op: &Operation, t: &Type, literals: &Vec<Literal>) -> Literal;
}

#[derive(Debug)]
struct BrilSemantics;
impl SemanticalReasononing for BrilSemantics {
    fn is_commutative(&self, operation: &Operation) -> bool {
        match operation {
            Operation::Value(value_op) => match value_op {
                ValueOp::And
                | ValueOp::Or
                | ValueOp::Add
                | ValueOp::Mul
                | ValueOp::Eq
                | ValueOp::Fadd
                | ValueOp::Fmul
                | ValueOp::Feq
                | ValueOp::Ceq => true,
                _ => false,
            },
            Operation::Memory(_) => false,
            Operation::Effect(_) => false,
            Operation::Constant(_) => false,
        }
    }

    fn is_copy(&self, operation: &Operation) -> bool {
        match operation {
            Operation::Value(ValueOp::Id) => true,
            _ => false,
        }
    }

    fn can_constexpr(&self, operation: &Operation) -> bool {
        match operation {
            Operation::Value(value_op) => match value_op {
                ValueOp::Add
                | ValueOp::Sub
                | ValueOp::Mul
                | ValueOp::Div
                | ValueOp::Fadd
                | ValueOp::Fsub
                | ValueOp::Fmul
                | ValueOp::Fdiv
                | ValueOp::Or
                | ValueOp::Not
                | ValueOp::And
                | ValueOp::Eq
                | ValueOp::Lt
                | ValueOp::Gt
                | ValueOp::Le
                | ValueOp::Ge
                | ValueOp::Feq
                | ValueOp::Flt
                | ValueOp::Fgt
                | ValueOp::Fle
                | ValueOp::Fge
                | ValueOp::Ceq
                | ValueOp::Clt
                | ValueOp::Cle
                | ValueOp::Cgt
                | ValueOp::Cge => true,
                // | ValueOp::Eq
                // | ValueOp::Feq
                // | ValueOp::Float2bits
                // | ValueOp::Bits2float
                // | ValueOp::Char2int
                // | ValueOp::Int2char
                // | ValueOp::Ceq => true,
                _ => false,
            },
            _ => false,
        }
    }

    fn eval_constexpr(&self, op: &Operation, _t: &Type, literals: &Vec<Literal>) -> Literal {
        assert!(self.can_constexpr(op));
        match op {
            Operation::Value(value_op) => match value_op {
                ValueOp::Add => literals[0].cast_to(&Type::Int) + literals[1].cast_to(&Type::Int),
                ValueOp::Sub => literals[0].cast_to(&Type::Int) - literals[1].cast_to(&Type::Int),
                ValueOp::Mul => literals[0].cast_to(&Type::Int) * literals[1].cast_to(&Type::Int),
                ValueOp::Div => literals[0].cast_to(&Type::Int) / literals[1].cast_to(&Type::Int),
                ValueOp::Fadd => {
                    literals[0].cast_to(&Type::Float) + literals[1].cast_to(&Type::Float)
                }
                ValueOp::Fsub => {
                    literals[0].cast_to(&Type::Float) - literals[1].cast_to(&Type::Float)
                }
                ValueOp::Fmul => {
                    literals[0].cast_to(&Type::Float) * literals[1].cast_to(&Type::Float)
                }
                ValueOp::Fdiv => {
                    literals[0].cast_to(&Type::Float) / literals[1].cast_to(&Type::Float)
                }
                ValueOp::Or => literals[0].cast_to(&Type::Bool) | literals[1].cast_to(&Type::Bool),
                ValueOp::Not => !literals[0].cast_to(&Type::Bool),
                ValueOp::And => literals[0].cast_to(&Type::Bool) & literals[1].cast_to(&Type::Bool),
                ValueOp::Eq => Literal::Bool(literals[0] == literals[1]),
                ValueOp::Lt => Literal::Bool(literals[0] < literals[1]),
                ValueOp::Gt => Literal::Bool(literals[0] > literals[1]),
                ValueOp::Le => Literal::Bool(literals[0] <= literals[1]),
                ValueOp::Ge => Literal::Bool(literals[0] >= literals[1]),
                ValueOp::Feq => Literal::Bool(literals[0] == literals[1]),
                ValueOp::Flt => Literal::Bool(literals[0] < literals[1]),
                ValueOp::Fgt => Literal::Bool(literals[0] > literals[1]),
                ValueOp::Fle => Literal::Bool(literals[0] <= literals[1]),
                ValueOp::Fge => Literal::Bool(literals[0] >= literals[1]),
                ValueOp::Ceq => Literal::Bool(literals[0] == literals[1]),
                ValueOp::Clt => Literal::Bool(literals[0] < literals[1]),
                ValueOp::Cgt => Literal::Bool(literals[0] > literals[1]),
                ValueOp::Cle => Literal::Bool(literals[0] <= literals[1]),
                ValueOp::Cge => Literal::Bool(literals[0] >= literals[1]),
                // ValueOp::Id => todo!(),
                // ValueOp::Char2int => todo!(),
                // ValueOp::Int2char => todo!(),
                // ValueOp::Float2bits => todo!(),
                // ValueOp::Bits2float => todo!(),
                // ValueOp::Call => todo!(),
                _ => panic!("should not be here"),
            },
            _ => panic!("should not be here"),
        }
    }
}

#[derive(Debug)]
struct LocalValueNumberingTable {
    // mangles variable names coming in and out of Canonical Home
    mangler: Mangling,

    /// maps variable name to id
    cloud: HashMap<String, usize>,

    /// maps expr to id
    table: HashMap<Expr, usize>,

    /// maps id to ch
    canonical_home: Vec<CanonicalHome>,

    /// semantical processing of arguments
    semantical_reasoning: Box<dyn SemanticalReasononing>,
}

impl LocalValueNumberingTable {
    fn new() -> Self {
        Self {
            mangler: Mangling::new(),
            cloud: HashMap::new(),
            table: HashMap::new(),
            canonical_home: vec![],
            semantical_reasoning: Box::new(BrilSemantics {}),
        }
    }

    fn to_abstract_args_list(&mut self, arg_list: &Vec<String>) -> Vec<usize> {
        // arg list is an array of concrete args, and we need convert it to an abstract list
        arg_list
            .iter()
            .map(|concrete_variable| {
                let mangled_variable = self.mangler.to_mangled(concrete_variable);
                if let Some(abstract_variable) = self.cloud.get(mangled_variable) {
                    // if an item in arg_list does exist in cloud, we should just return that id
                    abstract_variable.clone()
                } else {
                    // if an item in arg_list doesn't exist in cloud, we should create a direct mapping without
                    // an expression
                    self.canonical_home
                        .push(CanonicalHome::Variable(mangled_variable.clone()));
                    let id = self.canonical_home.len() - 1;
                    self.cloud.insert(mangled_variable.clone(), id);
                    id
                }
            })
            .collect()
    }

    fn from_abstract_args_list(&self, args_list: &Vec<usize>) -> Vec<String> {
        args_list
            .iter()
            .map(
                // we don't expect this part to fail, because it means that
                // we added to cloud or table without the matching entry in canonical home
                |abstract_variable| match &self.canonical_home[*abstract_variable] {
                    CanonicalHome::ConstExpr(_, _, mangled_variable) => self
                        .mangler
                        .from_mangled(&mangled_variable)
                        .expect("something went terribly wrong")
                        .clone(),
                    CanonicalHome::Variable(mangled_variable) => self
                        .mangler
                        .from_mangled(&mangled_variable)
                        .expect(&format!("something went terribly wrong: {:#?}", &self))
                        .clone(),
                },
            )
            .collect()
    }

    /// will return code that you should use instead if there was a matching expression
    /// else, the caller is responsible for generating their own code block
    fn register_expr(&mut self, expr: &Expr, dest: &str) -> Option<Code> {
        // destination should be mangled, but care must be taken to avoid trouble in the following case:
        // n = add n n

        // // take expr and sort args list if commutative
        // let semantic_expr = expr.clone();
        let semantic_expr = match expr {
            Expr::ConstExpr(..) => expr.clone(),
            Expr::Expr(t, op, items) => {
                let mut new_items = items.clone();
                if self.semantical_reasoning.is_commutative(op) {
                    new_items.sort();
                }

                if self.semantical_reasoning.is_copy(op) {
                    // TODO: semantic reasoning
                }

                Expr::Expr(t.clone(), op.clone(), new_items)
            }
        };

        // TODO: This is stupid; fix it for later
        if let Some(abstract_variable) = self.table.clone().get(&semantic_expr) {
            let ret = match &self.canonical_home[*abstract_variable] {
                CanonicalHome::ConstExpr(t, l, _) => Some(Code::Constant {
                    op: ConstantOp::Const,
                    dest: dest.to_owned(),
                    constant_type: t.clone(),
                    value: l.clone(),
                }),
                CanonicalHome::Variable(m) => Some(Code::Value {
                    op: ValueOp::Id,
                    dest: dest.to_owned(),
                    value_type: match &semantic_expr {
                        Expr::ConstExpr(t, _) => t.clone(),
                        Expr::Expr(t, _, _) => t.clone(),
                    },
                    args: Some(vec![self
                        .mangler
                        .from_mangled(m)
                        .expect(&format!("something went terribly wrong: {:#?}", &self))
                        .clone()]),
                    funcs: None,
                    labels: None,
                }),
            };

            // the semantic_expression was computed before
            let exists = self.mangler.exists(&dest);
            let mangled = self.mangler.mangle(&dest);
            // self.canonical_home.push(canonical_semantic_expr);
            // let id = self.canonical_home.len() - 1;
            // self.cloud.insert(mangled.clone(), id);
            self.cloud.insert(mangled.clone(), *abstract_variable);

            // update variable name in ch only if it's the same
            // if dest is the same as its arguments, we should update the ch.
            if exists {
                if let CanonicalHome::ConstExpr(t, l, _) = &self.canonical_home[*abstract_variable]
                {
                    self.canonical_home[*abstract_variable] =
                        CanonicalHome::ConstExpr(t.clone(), l.clone(), mangled.clone());
                } else {
                    self.canonical_home[*abstract_variable] =
                        CanonicalHome::Variable(mangled.clone());
                }
            }
            // println!("{:#?}", self);

            return ret;
        } else {
            // the semantic_expression is new

            // is the variable new? If the variable is not new, delete old variable mapping.
            if self.mangler.exists(&dest) {
                let mangled = self.mangler.to_mangled(&dest);
                let id = self.cloud.get(mangled).expect("this must exist");
                self.table.retain(|_, v| *v != *id);
            }

            let mangled = self.mangler.mangle(&dest);
            let canonical_semantic_expr = match &semantic_expr {
                Expr::ConstExpr(t, l) => {
                    // ok to mangle destination because constant has no variable dependencies
                    CanonicalHome::ConstExpr(t.clone(), l.clone(), mangled.clone())
                }
                Expr::Expr(t, o, args) => {
                    if self.semantical_reasoning.can_constexpr(o) {
                        // see if all args are constexpr
                        let mut constexpr_literals = Vec::new();
                        let can_constexpr_fold = args
                            .iter()
                            .map(|x| match &self.canonical_home[*x] {
                                CanonicalHome::ConstExpr(_, l, _) => {
                                    constexpr_literals.push(l.clone());
                                    true
                                }
                                CanonicalHome::Variable(_) => false,
                            })
                            .fold(true, |acc, elem| acc & elem);

                        if can_constexpr_fold {
                            // evaluate
                            // see if the variables are constexpr, then evaluate it.
                            // println!("expr can be consteval-ed {:?}", semantic_expr);
                            // println!("\t=>ch: {:?}", self.canonical_home);
                            // println!("\t-=> can fold: {:?}", constexpr_literals);

                            let result =
                                self.semantical_reasoning
                                    .eval_constexpr(o, t, &constexpr_literals);
                            let expr = CanonicalHome::ConstExpr(
                                t.clone(),
                                result.clone(),
                                mangled.clone(),
                            );
                            self.canonical_home.push(expr);
                            let id = self.canonical_home.len() - 1;
                            self.cloud.insert(mangled.clone(), id);
                            self.table.insert(semantic_expr.clone(), id);
                            // println!(
                            //     "returning constant: {:?}",
                            //     Code::Constant {
                            //         op: ConstantOp::Const,
                            //         dest: dest.to_owned(),
                            //         constant_type: t.clone(),
                            //         value: result.clone(),
                            //     }
                            // );
                            return Some(Code::Constant {
                                op: ConstantOp::Const,
                                dest: dest.to_owned(),
                                constant_type: t.clone(),
                                value: result,
                            });
                            // println!("\t-=> folded: {:?}", can_const.as_ref().unwrap());
                        }
                    }

                    CanonicalHome::Variable(mangled.clone())
                }
            };

            self.canonical_home.push(canonical_semantic_expr);
            let id = self.canonical_home.len() - 1;
            self.cloud.insert(mangled.clone(), id);
            self.table.insert(semantic_expr.clone(), id);

            None
        }
    }
}

fn lvn_on_block(mut basic_block: crate::blocks::BasicBlock) -> crate::blocks::BasicBlock {
    let mut lvn_state = LocalValueNumberingTable::new();

    let mut new_block = vec![];
    // println!("======================= new block ==============");
    for instr in &basic_block.block {
        // println!("instr: {:?}", instr);
        match &instr {
            Code::Label { .. } | Code::Noop { .. } => new_block.push(instr.clone()),
            Code::Constant {
                dest,
                constant_type,
                value,
                ..
            } => {
                let expr = Expr::ConstExpr(constant_type.clone(), value.clone());
                if let Some(code) = lvn_state.register_expr(&expr, dest) {
                    new_block.push(code);
                } else {
                    new_block.push(instr.clone());
                }
            }
            Code::Value {
                op,
                dest,
                value_type,
                args,
                funcs,
                labels,
            } => {
                match value_type {
                    Type::Ptr(_) => {
                        // do not touch ptrs
                        new_block.push(instr.clone());
                        continue;
                    }
                    _ => (),
                }
                // if it is a call, we just re-project the args;
                let concrete_args = args
                    .as_ref()
                    .expect("Value type operations must have an args list")
                    .clone();
                let abstract_args = lvn_state.to_abstract_args_list(&concrete_args);
                let expr = Expr::Expr(
                    value_type.clone(),
                    Operation::Value(*op),
                    abstract_args.clone(),
                );
                let before = Some(lvn_state.from_abstract_args_list(&abstract_args));
                if let Some(code) = lvn_state.register_expr(&expr, dest) {
                    new_block.push(code);
                } else {
                    new_block.push(Code::Value {
                        op: op.clone(),
                        dest: dest.clone(),
                        value_type: value_type.clone(),
                        args: before,
                        funcs: funcs.clone(),
                        labels: labels.clone(),
                    });
                }
            }
            Code::Effect {
                op,
                args,
                funcs,
                labels,
            } => {
                if args.is_none() {
                    new_block.push(instr.clone());
                    continue;
                }

                // println!("mangler: {:?}", lvn_state.mangler);
                // println!("cloud: {:?}", lvn_state.cloud);
                // println!("ch: {:?}", lvn_state.canonical_home);
                // println!("{:?}", instr);
                if labels.is_some() {
                    new_block.push(instr.clone());
                    continue;
                }

                let concrete_args = args
                    .as_ref()
                    .expect("Should not be here because of args non none check")
                    .clone();
                let abstract_args = lvn_state.to_abstract_args_list(&concrete_args);
                // println!("a {:?}", abstract_args);
                let reprojected_args = lvn_state.from_abstract_args_list(&abstract_args);
                // println!("b {:?}", abstract_args);

                new_block.push(Code::Effect {
                    op: op.clone(),
                    args: Some(reprojected_args),
                    funcs: funcs.clone(),
                    labels: labels.clone(),
                });
            }
            // Store, Alloc, and Free have side effects and must not be optimized
            // We are left with Load and PtrAdd which can be processed
            Code::Memory {
                op,
                args,
                dest,
                ptr_type,
            } => match op {
                MemoryOp::Alloc | MemoryOp::Free | MemoryOp::Store => new_block.push(instr.clone()),
                MemoryOp::Load | MemoryOp::PtrAdd => {
                    let concrete_args = args
                        .as_ref()
                        .expect("MemoryOp::Load type operations must have an args list")
                        .clone();
                    let abstract_args = lvn_state.to_abstract_args_list(&concrete_args);
                    let expr = Expr::Expr(
                        ptr_type.clone().expect("MemoryOp::Load must have type"),
                        Operation::Memory(*op),
                        abstract_args.clone(),
                    );
                    let before = Some(lvn_state.from_abstract_args_list(&abstract_args));
                    if let Some(code) = lvn_state.register_expr(
                        &expr,
                        &dest.clone().expect("MemoryOp::Load must have destination"),
                    ) {
                        new_block.push(code);
                    } else {
                        new_block.push(Code::Memory {
                            op: op.clone(),
                            args: before,
                            dest: dest.clone(),
                            ptr_type: ptr_type.clone(),
                        });
                    }
                }
            },
        }
    }

    basic_block.block = new_block;
    basic_block
}
