/// Module for local value numbering, Capable of copy propagation, cse, and const expression folding
use std::collections::HashMap;
use std::vec::Vec;
use uuid::Uuid;

use crate::program::{Code, ConstantOp, Literal, MemoryOp, Type, ValueOp};

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
}

impl LocalValueNumberingTable {
    fn new() -> Self {
        Self {
            mangler: Mangling::new(),
            cloud: HashMap::new(),
            table: HashMap::new(),
            canonical_home: vec![],
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

        // println!("expr: {:?}", expr);
        if let Some(abstract_variable) = self.table.get(expr) {
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
                    value_type: match &expr {
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

            // the expression was computed before
            let mangled = self.mangler.mangle(&dest);
            // self.canonical_home.push(canonical_expr);
            // let id = self.canonical_home.len() - 1;
            // self.cloud.insert(mangled.clone(), id);
            self.cloud.insert(mangled.clone(), *abstract_variable);

            return ret;
        } else {
            // the expression is new

            // is the variable new? If the variable is not new, delete old variable mapping.
            if self.mangler.exists(&dest) {
                let mangled = self.mangler.to_mangled(&dest);
                let id = self.cloud.get(mangled).expect("this must exist");
                self.table.retain(|_, v| *v != *id);
            }

            let mangled = self.mangler.mangle(&dest);
            let canonical_expr = match &expr {
                Expr::ConstExpr(t, l) => {
                    // ok to mangle destination because constant has no variable dependencies
                    CanonicalHome::ConstExpr(t.clone(), l.clone(), mangled.clone())
                }
                Expr::Expr(_, _, _) => CanonicalHome::Variable(mangled.clone()),
            };

            self.canonical_home.push(canonical_expr);
            let id = self.canonical_home.len() - 1;
            self.cloud.insert(mangled.clone(), id);
            self.table.insert(expr.clone(), id);

            None
        }
    }
}

fn lvn_on_block(mut basic_block: crate::blocks::BasicBlock) -> crate::blocks::BasicBlock {
    let mut lvn_state = LocalValueNumberingTable::new();

    let mut new_block = vec![];
    // println!("======================= new block ==============");
    for instr in &basic_block.block {
        // println!("{:?}", instr);
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
                new_block.push(instr.clone());
                // let concrete_args = args
                //     .as_ref()
                //     .expect("Effect type operations must have an args list")
                //     .clone();
                // let abstract_args = lvn_state.to_abstract_args_list(&concrete_args);
                // let reprojected_args = lvn_state.from_abstract_args_list(&abstract_args);

                // new_block.push(Code::Effect {
                //     op: op.clone(),
                //     args: Some(reprojected_args),
                //     funcs: funcs.clone(),
                //     labels: labels.clone(),
                // });
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
