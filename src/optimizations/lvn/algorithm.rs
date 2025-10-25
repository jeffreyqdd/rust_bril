use crate::{
    dataflow::{run_dataflow_analysis, WorklistProperty, WorklistResult},
    optimizations::lvn::numbering_table::LocalValueNumberingTable,
    representation::{AbstractFunction, ControlFlowGraph},
};

struct Lvn {}

impl WorklistProperty for Lvn {
    type Domain = LocalValueNumberingTable;

    fn init(_: usize, _: &crate::representation::AbstractFunction) -> Self::Domain {
        Self::Domain::default()
    }

    fn is_forward() -> bool {
        true
    }

    fn merge(
        predecessors: Vec<(&crate::representation::BlockId, &Self::Domain)>,
    ) -> crate::dataflow::WorklistResult<Self::Domain> {
        if predecessors.is_empty() {
            return Ok(LocalValueNumberingTable::default());
        }
        let mut merged = predecessors[0].1.clone();
        for (_, pred) in predecessors.iter().skip(1) {
            merged = merged.intersect(pred);
        }
        Ok(merged)
    }

    fn transfer(
        mut domain: Self::Domain,
        block_id: usize,
        cfg: &mut ControlFlowGraph,
        _: Option<&Vec<crate::representation::Argument>>,
    ) -> crate::dataflow::WorklistResult<Self::Domain> {
        let block = &mut cfg.basic_blocks[block_id];
        for instr in block.instructions.iter_mut() {
            *instr = domain.canonicalize(instr.clone());
        }
        Ok(domain)
    }
}

pub fn lvn(mut af: AbstractFunction) -> WorklistResult<AbstractFunction> {
    log::info!("running global value numbering on function '{}'", af.name);
    let start = std::time::Instant::now();
    run_dataflow_analysis::<Lvn>(&mut af)?;
    log::info!(
        "completed global value numbering on function '{}' in {:?}",
        af.name,
        start.elapsed(),
    );
    Ok(af)
}
