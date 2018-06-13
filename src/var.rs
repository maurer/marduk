use load::Loc;
use regs::{Reg, ARGS};

#[derive(Clone, Eq, Ord, Hash, PartialOrd, PartialEq, Debug)]
pub enum Var {
    StackSlot { func_addr: Loc, offset: usize },
    Register { register: Reg },
    Temp { serial: u32 },
    Alloc { site: Loc, stale: bool },
    Freed { site: Loc },
}

pub fn var_args() -> Vec<Var> {
    ARGS.iter().map(|reg| Var::Register {register: *reg}).collect()
}

impl Var {
    // Creates a temporary variable by doing string munging to extract the bap unique number from
    // the variable name.
    pub fn temp(name: &str) -> Var {
        let num: String = name.chars().skip_while(|x| !x.is_digit(10)).collect();
        assert!(num.chars().all(|x| x.is_digit(10)));

        Var::Temp {
            serial: num.parse().unwrap(),
        }
    }

    pub fn is_stacked(&self) -> bool {
        match *self {
            Var::StackSlot {
                func_addr: ref site,
                ..
            }
            | Var::Alloc { ref site, .. }
            | Var::Freed { ref site, .. } => site.is_stacked(),
            _ => false,
        }
    }

    pub fn is_temp(&self) -> bool {
        match *self {
            Var::Temp { .. } => true,
            _ => false,
        }
    }

    pub fn is_dyn(&self) -> bool {
        match *self {
            Var::Alloc { .. } => true,
            _ => false,
        }
    }

    pub fn is_freed(&self) -> bool {
        match *self {
            Var::Freed { .. } => true,
            _ => false,
        }
    }

    pub fn is_stack(&self) -> bool {
        match *self {
            Var::StackSlot { .. } => true,
            _ => false,
        }
    }

    pub fn other_func(&self, frames: &[Loc]) -> bool {
        match *self {
            Var::StackSlot { ref func_addr, .. } => !frames.contains(func_addr),
            _ => false,
        }
    }
}
