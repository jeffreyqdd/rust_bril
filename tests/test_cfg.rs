use rust_bril::blocks::{CfgGraph, Terminator};
use rust_bril::program::Program;
use tempfile::NamedTempFile;

#[test]
fn test_add_cfg_construction() {
    let program = Program::from_file("tests/fixtures/add.json");
    let function_blocks = program.basic_blocks();

    assert_eq!(function_blocks.len(), 1);
    let function_block = &function_blocks[0];
    assert_eq!(function_block.name, "main");

    // Should have 1 basic block (no control flow)
    assert_eq!(function_block.basic_blocks.len(), 1);

    let cfg = CfgGraph::from(function_block);
    assert_eq!(cfg.function.name, "main");
    assert_eq!(cfg.function.basic_blocks.len(), 1);
    assert_eq!(cfg.edges.len(), 1);
    assert_eq!(cfg.edges[0].len(), 0); // No outgoing edges (Passthrough at end)
}

#[test]
fn test_positions_cfg_construction() {
    let program = Program::from_file("tests/fixtures/positions.json");
    let function_blocks = program.basic_blocks();

    assert_eq!(function_blocks.len(), 1);
    let function_block = &function_blocks[0];
    assert_eq!(function_block.name, "main");

    // Should have 2 basic blocks (jmp creates a split)
    assert_eq!(function_block.basic_blocks.len(), 2);

    let cfg = CfgGraph::from(function_block);
    assert_eq!(cfg.function.name, "main");
    assert_eq!(cfg.function.basic_blocks.len(), 2);
    assert_eq!(cfg.edges.len(), 2);

    // First block should have jmp to second block
    assert_eq!(cfg.edges[0].len(), 1);
    assert_eq!(cfg.edges[0][0], 1);

    // Second block should have no outgoing edges
    assert_eq!(cfg.edges[1].len(), 0);

    // Verify labels are mapped correctly
    assert!(cfg.label_map.contains_key("label"));
    assert_eq!(cfg.label_map["label"], 1);
}

#[test]
fn test_cfg_serialization() {
    let program = Program::from_file("tests/fixtures/add.json");
    let function_blocks = program.basic_blocks();
    let cfg = CfgGraph::from(&function_blocks[0]);

    // Test that CFG can be serialized to JSON
    let json = cfg.to_string();
    assert!(json.contains("name"));
    assert!(json.contains("blocks"));
    assert!(json.contains("edges"));
    assert!(json.contains("label_map"));

    // Test that CFG can be written to file using temporary file
    let temp_file = NamedTempFile::new().expect("Failed to create temp file");
    let output_file = temp_file
        .path()
        .to_str()
        .expect("Failed to get temp file path");
    cfg.to_file(output_file);

    // Clean up is automatic when temp_file goes out of scope
}

#[test]
fn test_cfg_graph_properties() {
    let program = Program::from_file("tests/fixtures/positions.json");
    let function_blocks = program.basic_blocks();
    let cfg = CfgGraph::from(&function_blocks[0]);

    // Validate basic properties
    assert_eq!(cfg.function.name, "main");
    assert_eq!(cfg.function.basic_blocks.len(), 2);
    assert_eq!(cfg.edges.len(), 2);
    assert_eq!(cfg.label_map.len(), 2); // Both blocks have labels

    // Validate block structure
    let first_block = &cfg.function.basic_blocks[0];
    assert!(first_block.label.starts_with("no_label_"));
    assert!(matches!(first_block.terminator, Terminator::Jmp(_)));

    let second_block = &cfg.function.basic_blocks[1];
    assert_eq!(second_block.label, "label");
    assert!(matches!(second_block.terminator, Terminator::Passthrough));

    // Validate edges
    assert_eq!(cfg.edges[0].len(), 1);
    assert_eq!(cfg.edges[0][0], 1);
    assert_eq!(cfg.edges[1].len(), 0);

    // Validate label mapping
    assert_eq!(cfg.label_map["label"], 1);
}

#[test]
fn test_cfg_connectivity() {
    let program = Program::from_file("tests/fixtures/positions.json");
    let function_blocks = program.basic_blocks();
    let cfg = CfgGraph::from(&function_blocks[0]);

    // Test that all edges point to valid blocks
    for (i, edges) in cfg.edges.iter().enumerate() {
        for &target in edges {
            assert!(
                target < cfg.function.basic_blocks.len(),
                "Edge from block {} points to invalid block {}",
                i,
                target
            );
        }
    }

    // Test that all labels in terminators exist in label_map
    for block in &cfg.function.basic_blocks {
        match &block.terminator {
            Terminator::Jmp(label) => {
                assert!(
                    cfg.label_map.contains_key(label),
                    "Jmp target '{}' not found in label_map",
                    label
                );
            }
            Terminator::Br(label1, label2) => {
                assert!(
                    cfg.label_map.contains_key(label1),
                    "Br target '{}' not found in label_map",
                    label1
                );
                assert!(
                    cfg.label_map.contains_key(label2),
                    "Br target '{}' not found in label_map",
                    label2
                );
            }
            _ => {} // Passthrough and Ret don't have labels
        }
    }
}

#[test]
fn test_simple_cfg_no_control_flow() {
    let program = Program::from_file("tests/fixtures/add.json");
    let function_blocks = program.basic_blocks();
    let cfg = CfgGraph::from(&function_blocks[0]);

    // Simple program should have one block with no outgoing edges
    assert_eq!(cfg.function.basic_blocks.len(), 1);
    assert_eq!(cfg.edges.len(), 1);
    assert_eq!(cfg.edges[0].len(), 0);
    assert_eq!(cfg.label_map.len(), 1); // The block has a generated label

    // The single block should have Passthrough terminator
    assert!(matches!(
        cfg.function.basic_blocks[0].terminator,
        Terminator::Passthrough
    ));
}

#[test]
fn test_jumps_cfg_construction() {
    let program = Program::from_file("tests/fixtures/jumps.json");
    let function_blocks = program.basic_blocks();

    assert_eq!(function_blocks.len(), 1);
    let function_block = &function_blocks[0];
    assert_eq!(function_block.name, "main");

    // Should have 6 basic blocks (3 jumps + 3 label blocks)
    assert_eq!(function_block.basic_blocks.len(), 6);

    let cfg = CfgGraph::from(function_block);
    assert_eq!(cfg.function.name, "main");
    assert_eq!(cfg.function.basic_blocks.len(), 6);
    assert_eq!(cfg.edges.len(), 6);

    // First block should have jmp to label1 (block 3)
    assert_eq!(cfg.edges[0].len(), 1);
    assert_eq!(cfg.edges[0][0], 3);

    // Second block should have jmp to label2 (block 4)
    assert_eq!(cfg.edges[1].len(), 1);
    assert_eq!(cfg.edges[1][0], 4);

    // Third block should have jmp to label3 (block 5)
    assert_eq!(cfg.edges[2].len(), 1);
    assert_eq!(cfg.edges[2][0], 5);

    // Fourth block (label1) should have edge to label2 (block 4)
    assert_eq!(cfg.edges[3].len(), 1);
    assert_eq!(cfg.edges[3][0], 4);

    // Fifth block (label2) should have edge to label3 (block 5)
    assert_eq!(cfg.edges[4].len(), 1);
    assert_eq!(cfg.edges[4][0], 5);

    // Sixth block (label3) should have no outgoing edges
    assert_eq!(cfg.edges[5].len(), 0);

    // Verify labels are mapped correctly
    assert!(cfg.label_map.contains_key("label1"));
    assert!(cfg.label_map.contains_key("label2"));
    assert!(cfg.label_map.contains_key("label3"));
    assert_eq!(cfg.label_map["label1"], 3);
    assert_eq!(cfg.label_map["label2"], 4);
    assert_eq!(cfg.label_map["label3"], 5);

    // Verify terminators
    assert!(matches!(
        cfg.function.basic_blocks[0].terminator,
        Terminator::Jmp(_)
    ));
    assert!(matches!(
        cfg.function.basic_blocks[1].terminator,
        Terminator::Jmp(_)
    ));
    assert!(matches!(
        cfg.function.basic_blocks[2].terminator,
        Terminator::Jmp(_)
    ));
    assert!(matches!(
        cfg.function.basic_blocks[3].terminator,
        Terminator::Passthrough
    ));
    assert!(matches!(
        cfg.function.basic_blocks[4].terminator,
        Terminator::Passthrough
    ));
    assert!(matches!(
        cfg.function.basic_blocks[5].terminator,
        Terminator::Passthrough
    ));
}
