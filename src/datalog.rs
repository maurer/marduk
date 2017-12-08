use mycroft_macros::mycroft_files;
use bap::high::bitvector::BitVector;
use bap::high::bil::Statement;
use bap::basic::Arch;
use avar::AVar;
type Bytes = Vec<u8>;
type Sema = Vec<Statement>;
type Stack = Vec<(String, BitVector)>;
type Chop = Vec<BitVector>;

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
    "mycroft/queries.my",
    "mycroft/load.my",
    "mycroft/uaf_flow.my"
);
pub use self::mycroft_program::*;
