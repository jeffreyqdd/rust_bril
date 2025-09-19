use std::collections::HashSet;

use crate::{blocks::BasicBlock, program::Code};

pub trait WorklistProperty {
    type Domain: Clone + PartialEq + Eq + std::fmt::Debug;

    fn init() -> Self::Domain;
    fn deterministic_printable_medium(domain: &Self::Domain) -> Vec<String>;

    fn is_forward(&self) -> bool;
    fn merge(&self, predecessors: &Vec<Self::Domain>) -> Self::Domain;
    fn transfer(&self, domain: Self::Domain, block: &BasicBlock) -> Self::Domain;
}

pub struct InitializedVariables {}
impl WorklistProperty for InitializedVariables {
    type Domain = HashSet<String>;

    fn init() -> Self::Domain {
        HashSet::new()
    }

    fn is_forward(&self) -> bool {
        true
    }

    fn merge(&self, predecessors: &Vec<Self::Domain>) -> Self::Domain {
        predecessors
            .iter()
            .cloned()
            .reduce(|a, b| &a & &b)
            .unwrap_or_default()
    }

    fn transfer(&self, mut domain: Self::Domain, block: &BasicBlock) -> Self::Domain {
        domain.extend(block.block.iter().filter_map(|c| match c {
            Code::Constant { dest, .. } | Code::Value { dest, .. } => Some(dest.clone()),
            Code::Memory { dest: Some(x), .. } => Some(x.clone()),
            _ => None,
        }));
        domain
    }

    fn deterministic_printable_medium(domain: &Self::Domain) -> Vec<String> {
        let mut x: Vec<String> = domain.clone().into_iter().collect();
        x.sort();
        x
    }
}
