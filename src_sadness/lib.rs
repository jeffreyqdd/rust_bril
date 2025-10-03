pub mod blocks;
pub mod dominance;
pub mod optimizations;
pub mod program;
pub mod ssa;

pub fn transform_print(mut program: program::Program) -> program::Program {
    for function in &mut program.functions {
        let mut new_instrs = Vec::new();
        for instr in &mut function.instrs {
            type Eop = program::EffectOp;
            type Type = program::Type;
            type Literal = program::Literal;
            match instr {
                program::Code::Effect {
                    op: Eop::Br | Eop::Jmp,
                    labels,
                    ..
                } => {
                    new_instrs.push(program::Code::Constant {
                        op: crate::program::ConstantOp::Const,
                        dest: String::from("rust_bril_count"),
                        constant_type: Type::Int,
                        value: Literal::Int(match labels {
                            Some(v) => v.len() as i64,
                            None => 0,
                        }),
                    });
                    new_instrs.push(program::Code::Effect {
                        op: Eop::Print,
                        args: Some(vec![String::from("rust_bril_count")]),
                        funcs: None,
                        labels: None,
                    });
                    new_instrs.push(instr.clone())
                }
                _ => new_instrs.push(instr.clone()),
            }
        }

        function.instrs = new_instrs;
    }

    program
}
