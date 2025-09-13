use clap::Parser;
use rust_bril::{blocks::CfgGraph, optimizations, program::Program, transform_print};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file (if omitted, read from stdin). If the file extension is .bril, will run bril2json to convert to json
    #[arg(short, long)]
    file: Option<String>,

    /// output file after running optimization passes (if omitted, write to stdout)
    /// If the file extension is .bril, will run bril2txt to convert to text
    #[arg(short, long)]
    output: Option<String>,
    /// parse and print to stdout
    #[arg(long, action)]
    parse: bool,

    /// lesson2: transform, which will add print statements before every `jmp` and `br` instruction
    #[clap(long, action)]
    transform_print: bool,

    /// lesson2: construct cfg and write to file (will write to stdout if no file is provided)
    #[clap(long, num_args = 0..=1)]
    construct_cfg: Option<Vec<String>>,

    /// lesson 3: local optimization (DCE) (will write to stdout if no file is provided)
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

    if args.parse {
        println!("{:#?}", program);
    }

    if args.transform_print {
        program = transform_print(program);
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

    if args.local {
        let function_blocks = program.basic_blocks();
        let cfg_graphs: Vec<CfgGraph> = function_blocks
            .iter()
            .map(|x| CfgGraph::from(&x))
            .map(|x| optimizations::lvn::lvn(x))
            .map(|x| optimizations::dce::dce(x))
            .collect();

        program = Program::from(cfg_graphs);
    }

    if let Some(filepath) = args.output {
        program.to_file(&filepath);
    } else {
        println!("{}", program.to_string());
    }
}
