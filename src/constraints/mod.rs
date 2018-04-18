pub mod datalog;
pub mod generation;
use var::Var;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Constraint {
    // a = &b;
    AddrOf { a: Var, b: Var },
    // a = b
    Asgn { a: Var, b: Var },
    // a = *b
    Deref { a: Var, b: Var },
    // *a = b
    Write { a: Var, b: Var },
    // *a = *b
    Xfer { a: Var, b: Var },
    // *a = &b (can exist when b is a stack variable)
    StackLoad { a: Var, b: Var },
}
