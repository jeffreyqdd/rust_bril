use clap::Parser;
use rust_bril::{blocks::CfgGraph, program::Program, transform_print};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file (if omitted, read from stdin)
    #[arg(short, long)]
    file: Option<String>,

    /// Output file (if omitted, will write to `main.out.json`, `-` will print to stdout)
    #[arg(short, long, default_value = "main.out.json")]
    out: String,

    /// lesson2: transform, which will add print statements before every `jmp` and `br` instruction
    #[clap(long, action)]
    transform_print: bool,

    /// lesson2: construct cfg and write to file (if omitted, will write to stdout)
    #[clap(long, num_args = 0..=1)]
    construct_cfg: Option<Vec<String>>,
}
fn main() {
    let args = Args::parse();

    // parse program
    let mut program = match args.file {
        Some(filename) => Program::from_file(&filename),
        None => Program::from_stdin(),
    };

    if args.transform_print {
        program = transform_print(program);
    }

    if args.out == "-" {
        println!("{}", program.to_string());
    } else {
        program.to_file(&args.out);
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
