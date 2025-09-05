use rust_bril::program::{EffectOp, Program};
use rust_bril::transform_print;
use std::fs;

/// Helper function to verify that print operations are added before control flow instructions
fn assert_print_before_control_flow(program: Program) {
    for function in program.functions {
        for (i, instruction) in function.instrs.iter().enumerate() {
            match instruction {
                rust_bril::program::Code::Effect {
                    op: EffectOp::Br, ..
                } => {
                    // Assert that the previous instruction is a print operation
                    match &function.instrs[i - 1] {
                        rust_bril::program::Code::Effect {
                            op: EffectOp::Print,
                            ..
                        } => {}
                        _ => panic!(
                            "Expected print operation before Br, found: {:?}",
                            function.instrs[i - 1]
                        ),
                    }
                }
                rust_bril::program::Code::Effect {
                    op: EffectOp::Jmp, ..
                } => {
                    // Assert that the previous instruction is a print operation
                    match &function.instrs[i - 1] {
                        rust_bril::program::Code::Effect {
                            op: EffectOp::Print,
                            ..
                        } => {
                            // This is correct - print operation before jump
                        }
                        _ => panic!(
                            "Expected print operation before Jmp, found: {:?}",
                            function.instrs[i - 1]
                        ),
                    }
                }
                _ => continue,
            }
        }
    }
}

#[test]
fn test_transform_all_fixtures() {
    let fixtures_dir = "tests/fixtures";
    let entries = fs::read_dir(fixtures_dir).expect("Failed to read fixtures directory");

    for entry in entries {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        // Only process JSON files
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            let filename = path.to_str().expect("Failed to convert path to string");

            println!("Testing transform on: {}", filename);

            let program = transform_print(Program::from_file(filename));
            assert_print_before_control_flow(program);
        }
    }
}
