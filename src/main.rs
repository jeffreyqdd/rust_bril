mod blocks;
mod program;

use clap::Parser;

use crate::blocks::CfgGraph;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file (if omitted, read from stdin)
    #[arg(short, long)]
    file: Option<String>,

    /// Output file (if omitted, will write to `main.out.json`)
    #[arg(short, long, default_value = "main.out.json")]
    out: String,

    /// lesson2: transform, which will add print statements before every `jmp` and `br` instruction
    #[clap(long, action)]
    transform_print: bool,

    /// lesson2: construct cfg and write to file (if omitted, will write to stdout)
    #[clap(long, num_args = 0..=1)]
    construct_cfg: Option<Option<String>>,
}

fn transform_print(mut program: program::Program) -> program::Program {
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
                        op: String::from("const"),
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

fn main() {
    let args = Args::parse();

    // parse program
    let mut program = match args.file {
        Some(filename) => program::Program::from_file(&filename),
        None => program::Program::from_stdin(),
    };

    if args.transform_print {
        program = transform_print(program);
    }

    program.to_file(&args.out);

    if args.construct_cfg.as_ref().is_some() {
        let function_blocks = program.basic_blocks();
        let cfg_graphs: Vec<CfgGraph> =
            function_blocks.iter().map(|x| CfgGraph::from(&x)).collect();
        if let Some(Some(filename)) = &args.construct_cfg {
            for graph in &cfg_graphs {
                graph.to_file(filename);
            }
        } else {
            for graph in &cfg_graphs {
                println!("{:#?}", graph);
            }
        }
    }
}
