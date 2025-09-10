use clap::Parser;
use rust_bril::{blocks::CfgGraph, program::Program, transform_print};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file (if omitted, read from stdin)
    #[arg(short, long)]
    file: Option<String>,

    /// output file after running optimization passes (if omitted, write to stdout)
    #[arg(short, long)]
    output: Option<String>,

    /// lesson2: transform, which will add print statements before every `jmp` and `br` instruction (will write to stdout if no file is provided)
    #[clap(long, num_args = 0..=1)]
    transform_print: Option<Vec<String>>,

    /// lesson2: construct cfg and write to file (will write to stdout if no file is provided)
    #[clap(long, num_args = 0..=1)]
    construct_cfg: Option<Vec<String>>,

    /// lesson 3: local optimization (DCE)
    #[arg(long, action)]
    local: bool,
}
fn main() {
    let args = Args::parse();

    // parse program
    let mut program = match args.file {
        Some(filename) => Program::from_file(&filename),
        None => Program::from_stdin(),
    };

    if let Some(filepath) = args.transform_print {
        program = transform_print(program);
        if filepath.len() > 0 {
            program.to_file(&filepath[0]);
        } else {
            println!("{}", program.to_string());
        }
    }

    if let Some(filepath) = args.construct_cfg {
        let function_blocks = program.basic_blocks();
        let cfg_graphs: Vec<CfgGraph> =
            function_blocks.iter().map(|x| CfgGraph::from(&x)).collect();
        for graph in &cfg_graphs {
            if filepath.len() > 0 {
                graph.to_file(&filepath[0]);
            } else {
                println!("{:#?}", graph);
            }
        }
    }
}
