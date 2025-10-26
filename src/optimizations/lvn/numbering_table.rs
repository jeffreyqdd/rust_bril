use std::{
    collections::{HashMap, HashSet},
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
};

use crate::representation::{Code, ConstantOp, EffectOp, Literal, MemoryOp, Type, ValueOp};

static UID_COUNTER: OnceLock<AtomicUsize> = OnceLock::new();

fn next_uid() -> usize {
    let counter = UID_COUNTER.get_or_init(|| AtomicUsize::new(0));
    counter.fetch_add(1, Ordering::SeqCst)
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
/// Wrap operation in a unified enum
///
/// NOTE: EffectOperations should never be constructed since only their args need to be re-projected.
/// They do not create any new variables that we should keep track of.
#[allow(unused)]
enum Operation {
    Value(ValueOp),
    Memory(MemoryOp),
    Effect(EffectOp),
    Constant(ConstantOp),
}

#[derive(Debug, PartialEq, Eq, Hash, Clone)]
enum Expr {
    /// destination type
    ConstExpr(Type, Literal),

    /// destination type
    Expr(Type, Operation, Vec<usize>),
}

#[derive(Debug, Clone, Default)]
pub struct LocalValueNumberingTable {
    /// maps expression to value numbering
    /// answers the question, what is the CH of the expression?
    table: HashMap<Expr, (usize, String)>,

    /// Cloud data structure that maps variables to their LVN
    cloud: HashMap<String, (usize, String)>,
}

impl PartialEq for LocalValueNumberingTable {
    fn eq(&self, other: &Self) -> bool {
        let this_ch = self
            .cloud
            .values()
            .map(|(_, var)| var)
            .collect::<HashSet<&String>>();
        let other_ch = other
            .cloud
            .values()
            .map(|(_, var)| var)
            .collect::<HashSet<&String>>();
        this_ch == other_ch
    }
}

impl Eq for LocalValueNumberingTable {}

impl LocalValueNumberingTable {
    fn get_variable_numbering(&mut self, var: &str) -> (usize, String) {
        if let Some(res) = self.cloud.get(var) {
            return res.clone();
        }

        let vn: usize = next_uid();
        self.cloud.insert(var.to_string(), (vn, var.to_string()));
        self.cloud.get(var).unwrap().clone()
    }

    fn flatten_copy(&self, code: &Code) -> Option<Expr> {
        if matches!(
            code,
            Code::Value {
                op: ValueOp::Id,
                ..
            }
        ) {
            let arg_var = &code.get_arguments().unwrap()[0];
            if let Some((_, expr_var)) = self.cloud.get(arg_var) {
                for (expr, (_, var)) in self.table.iter() {
                    if var == expr_var {
                        return Some(expr.clone());
                    }
                }
            }
        }
        None
    }

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

    fn is_constexpr(&self, operation: &Operation) -> bool {
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
                | ValueOp::Cge
                | ValueOp::Float2bits
                | ValueOp::Bits2float
                | ValueOp::Char2int
                | ValueOp::Int2char => true,
                _ => false,
            },
            _ => false,
        }
    }

    fn eval_constexpr(&self, op: &Operation, _t: &Type, literals: &Vec<Literal>) -> Literal {
        assert!(self.is_constexpr(op));
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
                ValueOp::Char2int => literals[0].cast_to(&Type::Int),
                ValueOp::Int2char => literals[0].cast_to(&Type::Char),
                ValueOp::Float2bits => literals[0].bitcast(&Type::Int),
                ValueOp::Bits2float => literals[0].bitcast(&Type::Float),
                _ => panic!("should not be here"),
            },
            _ => panic!("should not be here"),
        }
    }

    pub fn intersect(&self, other: &Self) -> Self {
        let mut new_table = HashMap::new();
        let mut new_cloud = HashMap::new();
        // println!("Intersecting LVN tables:");
        // // println!("  Self: {:?}", self);
        // println!("  Other: {:?}", other);
        for (expr, (num, var)) in &self.table {
            if let Some((other_num, other_var)) = other.table.get(expr) {
                // If both tables map the same expr to the same variable name, keep it.
                if var == other_var && num == other_num {
                    new_table.insert(expr.clone(), (*num, var.clone()));
                }
            }
        }

        // Cloud intersection: keep only variables that remain valid in new_table.
        for (var, num) in &self.cloud {
            if let Some(other_num) = other.cloud.get(var) {
                if num == other_num {
                    new_cloud.insert(var.clone(), num.clone());
                }
            }
        }

        let ret = Self {
            table: new_table,
            cloud: new_cloud,
        };

        ret
    }

    pub fn fold(&self, expr: Expr) -> Expr {
        if let Expr::Expr(t, op, args) = expr.clone() {
            if self.is_constexpr(&op) {
                let constexpr = args
                    .iter()
                    .filter_map(|uid| {
                        for (expr, (x, y)) in self.table.iter() {
                            if x == uid {
                                if let Expr::ConstExpr(_, lit) = expr {
                                    return Some(lit.clone());
                                }
                            }
                        }
                        return None;
                    })
                    .collect::<Vec<_>>();

                if constexpr.len() == args.len() {
                    let folded_literal = self.eval_constexpr(&op, &t, &constexpr);
                    log::trace!("folding expr {:?} into constant {:?}", expr, folded_literal);
                    return Expr::ConstExpr(t, folded_literal);
                }
            }
        }
        return expr;
    }

    pub fn canonicalize(&mut self, code: Code) -> Code {
        log::trace!("\ncanonicalizing code instruction: {:?}", code);
        let code_copy = code.clone();
        match code {
            Code::Label { .. } => code,
            Code::Effect {
                op,
                args,
                funcs,
                labels,
                pos,
            } => {
                // should at least remap the arguments into effect
                let remapped_args = args.as_ref().map(|v| {
                    v.iter()
                        .map(|a| self.get_variable_numbering(a).0)
                        .collect::<Vec<_>>()
                });

                Code::Effect {
                    op,
                    args: remapped_args.map(|v| {
                        v.iter()
                            .map(|num| {
                                // find variable name from cloud
                                for (var, (n, _)) in self.cloud.iter() {
                                    if n == num {
                                        return var.clone();
                                    }
                                }
                                panic!("variable number {} not found in cloud", num);
                            })
                            .collect()
                    }),
                    funcs,
                    labels,
                    pos,
                }
            }
            Code::Memory { .. } => code,
            Code::Noop { .. } => code,
            Code::Value {
                op: ValueOp::Call, ..
            } => code,
            Code::Value {
                value_type: Type::Ptr(..),
                ..
            } => code,
            Code::Constant {
                op,
                dest,
                constant_type,
                value,
                pos,
            } => {
                // constant types allow us to skip renaming arguments
                let expr = Expr::ConstExpr(constant_type.clone(), value);
                let (num, ch, ret) = if let Some((num, var)) = self.table.get(&expr) {
                    (
                        *num,
                        var.clone(),
                        Code::Value {
                            op: ValueOp::Id,
                            dest: dest.clone(),
                            value_type: constant_type,
                            args: Some(vec![var.clone()]),
                            funcs: None,
                            labels: None,
                            pos: pos,
                        },
                    )
                } else {
                    let fresh_lvn = next_uid();
                    self.table.insert(expr, (fresh_lvn, dest.clone()));
                    (
                        fresh_lvn,
                        dest.clone(),
                        Code::Constant {
                            op,
                            dest: dest.clone(),
                            constant_type,
                            value,
                            pos,
                        },
                    )
                };
                self.cloud.insert(dest, (num, ch));
                ret
            }
            Code::Value {
                op,
                dest,
                value_type,
                args,
                funcs,
                labels,
                pos,
            } => {
                let mut remapped_args: Vec<usize> = args
                    .as_ref()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|a| self.get_variable_numbering(a).0)
                    .collect();

                // do copy propagation if possible
                let mut expr = if let Some(expr) = self.flatten_copy(&code_copy) {
                    expr
                } else {
                    if self.is_commutative(&Operation::Value(op.clone())) {
                        remapped_args.sort();
                    }
                    Expr::Expr(
                        value_type.clone(),
                        Operation::Value(op.clone()),
                        remapped_args.clone(),
                    )
                };

                // if expression can be constant folded, do it
                // if both expression args are constants, we can constant fold
                expr = self.fold(expr);
                if let Expr::ConstExpr(t, l) = expr {
                    assert!(t == value_type);
                    return self.canonicalize(Code::Constant {
                        op: ConstantOp::Const,
                        dest: dest,
                        constant_type: value_type,
                        value: l,
                        pos: pos,
                    });
                }

                let (num, ch, ret) = if let Some((num, var)) = self.table.get(&expr) {
                    (
                        *num,
                        var.clone(),
                        Code::Value {
                            op: ValueOp::Id,
                            dest: dest.clone(),
                            value_type,
                            args: Some(vec![var.clone()]),
                            funcs: funcs,
                            labels: labels,
                            pos,
                        },
                    )
                } else {
                    let fresh_lvn = next_uid();
                    self.table.insert(expr, (fresh_lvn, dest.clone()));

                    let remapped_args: Vec<String> = args
                        .unwrap_or(vec![])
                        .iter()
                        .map(|a| self.get_variable_numbering(a).1)
                        .collect();

                    (
                        fresh_lvn,
                        dest.clone(),
                        Code::Value {
                            op,
                            dest: dest.clone(),
                            value_type,
                            args: Some(remapped_args),
                            funcs: funcs,
                            labels: labels,
                            pos,
                        },
                    )
                };
                self.cloud.insert(dest, (num, ch));
                ret
            }
        }
    }
}
