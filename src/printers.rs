use constraints::Constraint;
use datalog::*;
use interned_string::InternedString;
use load::Loc;
use regs::Reg;
use std::fmt::{Display, Formatter, Result};
use var::Var;
use AliasMode;

pub struct CB<'a, T: Display + 'a>(pub &'a Vec<T>);

pub fn fmt_vec<T: Display>(f: &mut Formatter, v: &[T]) -> Result {
    write!(f, "[")?;
    for i in v.iter().take(1) {
        write!(f, "{}", i)?;
    }
    for i in v.iter().skip(1) {
        write!(f, ", {}", i)?;
    }
    write!(f, "]")
}

impl Display for AliasMode {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}",
            match *self {
                AliasMode::SteensOnly { .. } => "Insensitive",
                AliasMode::FlowOnly { .. } => "Flow sensitive",
                AliasMode::Both { .. } => "Both",
                AliasMode::LoadOnly { .. } => "Load",
            }
        )?;
        if self.uses_ctx() {
            write!(f, "+ctx")?;
        }
        Ok(())
    }
}

impl<'a, T: Display> Display for CB<'a, T> {
    fn fmt(&self, f: &mut Formatter) -> Result {
        fmt_vec(f, self.0)
    }
}

impl Display for UsedVarResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "use@{}: {}", self.loc, self.var)
    }
}

impl Display for LiveVarsResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "live@{}: {}", self.loc, CB(&self.vars))
    }
}

impl Display for Reg {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{:?}", self)
    }
}

impl Display for Var {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            Var::Temp { serial } => write!(f, "v{}", serial),
            Var::StackSlot {
                ref func_addr,
                ref offset,
            } => write!(f, "sp+{}@{}", offset, func_addr),
            Var::Register { ref register, .. } => write!(f, "{}", register),
            Var::Alloc {
                ref site,
                ref stale,
            } => {
                write!(f, "dyn@{}", site)?;
                if *stale {
                    write!(f, "+stale")?;
                }
                Ok(())
            }
            Var::Freed { ref site } => write!(f, "freed@{}", site),
        }
    }
}

impl Display for Constraint {
    fn fmt(&self, f: &mut Formatter) -> Result {
        use constraints::Constraint::*;
        match *self {
            AddrOf { ref a, ref b } => write!(f, "{} = &{}", a, b),
            Asgn { ref a, ref b } => write!(f, "{} = {}", a, b),
            Deref { ref a, ref b } => write!(f, "{} = *{}", a, b),
            Write { ref a, ref b } => write!(f, "*{} = {}", a, b),
            Xfer { ref a, ref b } => write!(f, "*{} = *{}", a, b),
            StackLoad { ref a, ref b } => write!(f, "*{} = &{}", a, b),
            Clobber { ref v } => write!(f, "{} = ?", v),
        }
    }
}

impl Display for DerefVarResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}:*{}", self.loc, self.var)
    }
}

impl Display for UafResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}->{}", self.free, self.use_)
    }
}

impl Display for UafFlowResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}->{}", self.free, self.use_)
    }
}

impl Display for CallSiteResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}->{}", self.call_loc, self.target_loc)
    }
}

impl Display for InternedString {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}", self.to_string())
    }
}

impl Display for Loc {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "0x{:x}", self.addr)?;
        match self.stack {
            ::load::Stack::Return(ref loc) => {
                write!(f, "+{}", loc)?;
            }
            ::load::Stack::EmptyStack => {
                write!(f, "{{}}")?;
            }
            _ => (),
        }
        Ok(())
    }
}

impl Display for DefinesResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{} defs {:?}", self.loc, self.registers)
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

impl Display for FlowResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}:\n{}", self.loc, self.pts)
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
