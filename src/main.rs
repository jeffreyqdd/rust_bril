use clap::{Parser, ValueEnum};
use log::LevelFilter;
use rust_bril::{bril_logger, optimizations::dce, optimizations::lvn};
use std::path::Path;

// use rust_bril::{
//     blocks::CfgGraph,
//     dominance,
//     optimizations::{
//         self,
//         dataflow::run_dataflow_analysis,
//         dataflow_properties::{InitializedVariables, LiveVariables},
//     },
//     program::Program,
//     ssa, transform_print,
// };

#[derive(ValueEnum, Clone, Copy, Debug, PartialEq, Eq)]
enum LogLevel {
    /// Trace level logging (most verbose)
    Trace,
    /// Debug level logging
    Debug,
    /// Info level logging (default)
    Info,
    /// Warning level logging
    Warn,
    /// Error level logging
    Error,
    /// No logging
    Off,
}

// #[derive(ValueEnum, Clone, Debug, PartialEq, Eq)]
// enum DataflowAnalysis {
//     /// set of variables that are initialized by the end of each basic block
//     InitializedVariables,

//     /// set of variables that are referenced at some point in the future
//     LiveVariables,
// }

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Input file (if omitted, read from stdin). If the file extension is .bril, will run bril2json to convert to json
    // make this positional
    file: String,

    #[arg(short, long)]
    output: Option<String>,

    /// Set the log level (trace, debug, info, warn, error, off)
    #[arg(long, value_enum, default_value = "info")]
    log_level: LogLevel,

    /// Don't push out of SSA form
    #[arg(short = 'S', action)]
    show_ssa: bool,

    /// Run dead code elimination
    #[arg(long, action)]
    dce: bool,

    /// Run local value numbering
    #[arg(long, action)]
    lvn: bool,

    /// Run loop optimizations
    #[arg(long, action)]
    loops: bool,
}

impl From<LogLevel> for LevelFilter {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Trace => LevelFilter::Trace,
            LogLevel::Debug => LevelFilter::Debug,
            LogLevel::Info => LevelFilter::Info,
            LogLevel::Warn => LevelFilter::Warn,
            LogLevel::Error => LevelFilter::Error,
            LogLevel::Off => LevelFilter::Off,
        }
    }
}

fn main() {
    let args = Args::parse();

    if let Err(e) = bril_logger::init_logger(args.log_level.into()) {
        eprintln!("Failed to initialize logger: {}", e);
        std::process::exit(1);
    }

    // parse into program
    let time_start = std::time::Instant::now();
    let file_paths = Path::new(&args.file);
    let rich_program = match rust_bril::representation::RichProgram::from_file(file_paths) {
        Ok(p) => p,
        Err(e) => {
            log::error!("Failed to load program from file '{}': {}", args.file, e);
            std::process::exit(1);
        }
    };
    log::info!(
        "loaded program from '{}' in {:?}",
        args.file,
        time_start.elapsed()
    );

    // convert into SSA form
    let mut abstract_program = rust_bril::representation::RichAbstractProgram::from(rich_program);

    // run optimizations
    if args.loops {
        abstract_program.program.functions = abstract_program
            .program
            .functions
            .into_iter()
            .map(|(n, af)| {
                match rust_bril::optimizations::loops::loop_invariant_code_motion_pass(af) {
                    Ok(af_new) => (n, af_new),
                    Err(e) => e.error_with_context_then_exit(&abstract_program.original_text),
                }
            })
            .collect();
    }
    if args.lvn {
        abstract_program.program.functions = abstract_program
            .program
            .functions
            .into_iter()
            .map(|(n, af)| match lvn(af) {
                Ok(af_new) => (n, af_new),
                Err(e) => e.error_with_context_then_exit(&abstract_program.original_text),
            })
            .collect();
    }

    if args.dce {
        abstract_program.program.functions = abstract_program
            .program
            .functions
            .into_iter()
            .map(|(n, af)| match dce(af) {
                Ok(af_new) => (n, af_new),
                Err(e) => e.error_with_context_then_exit(&abstract_program.original_text),
            })
            .collect();
    }

    // convert out of SSA form
    let final_program = if args.show_ssa {
        abstract_program.into_ssa_program()
    } else {
        abstract_program.into_program()
    };

    if let Some(filepath) = args.output {
        log::info!("writing program to file '{}'", filepath);
        match final_program.to_file(Path::new(&filepath)) {
            Ok(_) => (),
            Err(e) => {
                log::error!("Failed to write program to file '{}': {}", filepath, e);
                std::process::exit(1);
            }
        };
    } else {
        println!("{}", final_program.to_string());
    }
}
