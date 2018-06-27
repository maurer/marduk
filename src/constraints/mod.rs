pub mod datalog;
pub mod generation;
use var::Var;
use regs::Reg;
use load::Loc;

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct VarPath {
    pub base: Var,
    pub offsets: Vec<Option<u64>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub struct Constraint {
    pub lhs: VarPath,
    pub rhss: Vec<VarPath>,
}

impl Constraint {
    pub fn has_stacked(&self) -> bool {
        self.lhs.base.is_stacked() || self.rhss.iter().any(|vp| vp.base.is_stacked())
    }
}

pub fn off_plus(base: &mut Option<u64>, off: u64) {
    base.as_mut().map(|base_val| *base_val += off);
}

impl VarPath {
    pub fn var(var: Var) -> Self {
        Self {
            base: var,
            // First 0 is to the register "region", second is the 0 offset on the value of the
            // register
            offsets: vec![Some(0), Some(0)]
        }
    }

    pub fn reg(reg: &Reg) -> Self {
        Self::var(Var::Register { register: *reg})
    }

    pub fn temp(s: &str) -> VarPath {
        Self::var(Var::temp(s))
    }

    pub fn addr(var: Var) -> Self {
        Self {
            base: var,
            // First 0 is an offset into the register. We haven't dereferenced yet
            offsets: vec![Some(0)]
        }
    }

    pub fn stack_addr(func_addr: &Loc, offset: usize) -> Self {
        Self::addr(Var::StackSlot {
            func_addr: func_addr.clone(),
            offset: offset
        })
    }
 
    pub fn plus(&self, off: u64) -> Self {
        let mut out = self.clone();
        off_plus(out.offsets.last_mut().unwrap(), off);
        out
    }

    pub fn unknown(&self) -> Self {
        let mut out = self.clone();
        *out.offsets.last_mut().unwrap() = None;
        out
    }

    pub fn derefs(&self) -> usize {
        self.offsets.len()
    }

    pub fn deref(&self) -> Self {
        let mut out = self.clone();
        out.offsets.push(Some(0));
        out
    }
}
