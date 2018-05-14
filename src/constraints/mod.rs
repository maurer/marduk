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

impl Constraint {
    pub fn has_stacked(&self) -> bool {
        use self::Constraint::*;
        let (a, b) = match *self {
            AddrOf { ref a, ref b }
            | Asgn { ref a, ref b }
            | Deref { ref a, ref b }
            | Write { ref a, ref b }
            | Xfer { ref a, ref b }
            | StackLoad { ref a, ref b } => (a, b),
        };
        a.is_stacked() || b.is_stacked()
    }
}
