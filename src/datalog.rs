use mycroft_macros::mycroft_files;
use bap::high::bitvector::BitVector;
use bap::high::bil::Statement;
use bap::basic::Arch;
use avar::AVar;
use steensgaard::{Constraint, DefChain, Var};
use std::collections::BTreeSet;
type Bytes = Vec<u8>;
type Sema = Vec<Statement>;
type Stack = Vec<(String, BitVector)>;
type Chop = Vec<BitVector>;
type StringSet = BTreeSet<String>;
type Strings = Vec<String>;
type Constraints = Vec<Constraint>;
type BitVectorSet = BTreeSet<BitVector>;
type Vars = Vec<Var>;

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

fn new_stack() -> Stack {
    Vec::new()
}

fn new_chop() -> Chop {
    Vec::new()
}

fn or(x: bool, y: bool) -> bool {
    x || y
}

const ZERO: usize = 0;
mycroft_files!(
    "mycroft/schema.my",
    "mycroft/load.my",
    "mycroft/uaf_flow.my",
    "mycroft/uaf_pathlen.my",
    "mycroft/defs.my",
    "mycroft/steensgaard.my",
    "mycroft/queries.my"
);
pub use self::mycroft_program::*;
