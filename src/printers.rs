use std::fmt::{Display, Formatter, Result};
use steensgaard::{Constraint, Var};
use datalog::*;

pub struct CB<'a>(pub &'a Vec<Constraint>);

fn fmt_vec<T: Display>(f: &mut Formatter, v: &Vec<T>) -> Result {
    write!(f, "[")?;
    for i in 0..v.len() {
        if i != 0 {
            write!(f, ", ")?
        }
        write!(f, "{}", v[i])?
    }
    write!(f, "]")
}

impl<'a> Display for CB<'a> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        fmt_vec(f, self.0)
    }
}

impl Display for Var {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            Var::StackSlot {
                ref func_addr,
                ref offset,
            } => write!(f, "sp+{}@{}", offset, func_addr),
            Var::Register {
                ref site,
                ref register,
                ..
            } => write!(f, "{}@{}", register, site),
            Var::Alloc { ref site } => write!(f, "dyn@{}", site),
        }
    }
}

impl Display for Constraint {
    fn fmt(&self, f: &mut Formatter) -> Result {
        use steensgaard::Constraint::*;
        match *self {
            AddrOf { ref a, ref b } => write!(f, "{} = &{}", a, b),
            Asgn { ref a, ref b } => write!(f, "{} = {}", a, b),
            Deref { ref a, ref b } => write!(f, "{} = *{}", a, b),
            Write { ref a, ref b } => write!(f, "*{} = {}", a, b),
            Xfer { ref a, ref b } => write!(f, "*{} = *{}", a, b),
        }
    }
}

impl Display for CallSiteResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}->{}", self.call_loc, self.target_loc)
    }
}

impl Display for SteensSetsResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        fmt_vec(f, &self.vs)
    }
}

impl Display for SteensResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}->", self.v)?;
        fmt_vec(f, &self.vs)
    }
}

impl Display for Loc {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}@{}", self.file_name, self.addr)
    }
}

impl Display for DefinesResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{} defs {:?}", self.loc, self.registers)
    }
}

impl Display for ReachingResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}:{}->{}", self.defs, self.register, self.reached)
    }
}

impl Display for SuccResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}->{}~call={}", self.src, self.dst, self.is_call)
    }
}

impl Display for LinkPadResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}: {}", self.pad_loc, self.pad_name)
    }
}

impl Display for GetFreeCallResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.loc)
    }
}

impl Display for GetMallocCallResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.loc)
    }
}

impl Display for LiveResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.loc)
    }
}

impl Display for AnyFact {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            AnyFact::Lift(Lift {
                ref loc,
                ref disassembly,
                ..
            }) => write!(f, "lift: {}: {:?}", loc, disassembly),
            AnyFact::Sym(Sym {
                ref name, ref loc, ..
            }) => write!(f, "sym: {}:{}", name, loc),
            AnyFact::File(File { ref name, .. }) => write!(f, "file: {:?}", name),
            AnyFact::Segment(Segment {
                ref start, ref end, ..
            }) => write!(f, "segment: {}->{}", start, end),
            AnyFact::Live(Live { ref loc, .. }) => write!(f, "live: {}", loc),
            AnyFact::Succ(Succ {
                ref dst, ref src, ..
            }) => write!(f, "succ: {} -> {}", src, dst),
            AnyFact::SuccOver(SuccOver {
                ref dst, ref src, ..
            }) => write!(f, "succ_over: {} -> {}", src, dst),
            AnyFact::ProgArch(ProgArch {
                arch,
                ref file_name,
            }) => write!(f, "arch: {}({})", file_name, arch),
            // As a fallback, use debug
            ref x => write!(f, "debug: {:?}", x),
        }
    }
}
