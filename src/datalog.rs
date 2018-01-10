use mycroft_macros::mycroft_files;
use bap::high::bitvector::BitVector;
use bap::high::bil::Statement;
use bap::basic::Arch;
use steensgaard::{Constraint, DefChain, Var};
use std::collections::BTreeSet;
type Bytes = Vec<u8>;
type Sema = Vec<Statement>;
type StringSet = BTreeSet<String>;
type Strings = Vec<String>;
type Constraints = Vec<Constraint>;
type Vars = Vec<Var>;
type LocSet = BTreeSet<Loc>;

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Hash)]
pub struct Loc {
    pub file_name: String,
    pub addr: BitVector,
}

fn chain_merge(dc: &DefChain, dc2: &DefChain) -> DefChain {
    dc.iter()
        .chain(dc2.iter())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn union<T: Clone + Eq + Ord>(x: &BTreeSet<T>, y: &BTreeSet<T>) -> BTreeSet<T> {
    x.union(y).cloned().collect()
}

fn concat<T: Clone>(x: &Vec<T>, y: &Vec<T>) -> Vec<T> {
    x.iter().chain(y.iter()).cloned().collect()
}

mycroft_files!(
    "mycroft/schema.my",
    "mycroft/load.my",
    "mycroft/defs.my",
    "mycroft/steensgaard.my",
    "mycroft/queries.my"
);
pub use self::mycroft_program::*;
