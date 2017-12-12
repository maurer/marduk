use mycroft_macros::mycroft_files;
use bap::high::bitvector::BitVector;
use bap::high::bil::Statement;
use bap::basic::Arch;
use avar::AVar;
use std::collections::BTreeSet;
type Bytes = Vec<u8>;
type Sema = Vec<Statement>;
type Stack = Vec<(String, BitVector)>;
type Chop = Vec<BitVector>;
type StringSet = BTreeSet<String>;

fn new_stack() -> Stack {
    Vec::new()
}

fn new_chop() -> Chop {
    Vec::new()
}

fn or(x: bool, y: bool) -> bool {
    x || y
}

fn union<T: Clone + Ord + Eq>(x: &BTreeSet<T>, y: &BTreeSet<T>) -> BTreeSet<T> {
    x.union(y).cloned().collect()
}

const ZERO: usize = 0;
mycroft_files!(
    "mycroft/schema.my",
    "mycroft/queries.my",
    "mycroft/load.my",
    "mycroft/uaf_flow.my",
    "mycroft/uaf_pathlen.my"
);
pub use self::mycroft_program::*;
