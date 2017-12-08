use std::fmt::{Display, Formatter, Result};
use datalog::*;

impl Display for FuncResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}@{}:{}", self.file, self.entry, self.addr)
    }
}

impl Display for CallSiteResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}@{}->{}@{}",
            self.call_file,
            self.call_addr,
            self.dst_file,
            self.dst_addr
        )
    }
}

impl Display for GetUafFlowFullResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}@{}:{} -> {}@{}:{}",
            self.name,
            self.addr,
            self.alias,
            self.use_name,
            self.use_addr,
            self.use_var
        )
    }
}

impl Display for SuccResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}@{}->{}~call={}",
            self.name,
            self.src,
            self.dst,
            self.call
        )
    }
}

impl Display for LinkPadResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}@{}: {}", self.name, self.addr, self.pad_name)
    }
}

impl Display for GetFreeCallResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}@{}", self.name, self.addr)
    }
}

impl Display for GetMallocCallResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}@{}", self.name, self.addr)
    }
}

impl Display for LiveResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(f, "{}@{}", self.name, self.addr)
    }
}

impl Display for GetAliasResult {
    fn fmt(&self, f: &mut Formatter) -> Result {
        write!(
            f,
            "{}@{}:{} -> {}@{}:{} : {}",
            self.file0,
            self.addr0,
            self.alias_set,
            self.file,
            self.addr,
            self.a_var,
            self.freed
        )
    }
}

impl Display for AnyFact {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match *self {
            AnyFact::Lift(Lift {
                              ref address,
                              ref disassembly,
                              ..
                          }) => write!(f, "lift: {}: {:?}", address, disassembly),
            AnyFact::Sym(Sym {
                             ref name,
                             ref start,
                             ..
                         }) => write!(f, "sym: {}@{}", name, start),
            AnyFact::File(File { ref name, .. }) => write!(f, "file: {:?}", name),
            AnyFact::Segment(Segment { ref start, ref end, .. }) => {
                write!(f, "segment: {}->{}", start, end)
            }
            AnyFact::Live(Live { ref addr, .. }) => write!(f, "live: {}", addr),
            AnyFact::Succ(Succ {
                              ref dst_addr,
                              ref src_addr,
                              ..
                          }) => write!(f, "succ: {} -> {}", src_addr, dst_addr),
            AnyFact::SuccOver(SuccOver { ref dst, ref src, .. }) => {
                write!(f, "succ_over: {} -> {}", src, dst)
            }
            AnyFact::ProgArch(ProgArch {
                                  arch,
                                  ref file_name,
                              }) => write!(f, "arch: {}({})", file_name, arch),
            // As a fallback, use debug
            ref x => write!(f, "debug: {:?}", x),
        }
    }
}
