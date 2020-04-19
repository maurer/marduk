use bap::basic::{Bap, BasicDisasm, Image};
use bap::high::bil::{Expression, Statement};
use bap::high::bitvector::BitVector;
use crate::datalog::*;
use crate::interned_string::InternedString;
use std::collections::BTreeSet;

const STACK_MAX_DEPTH: usize = 1;

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Hash)]
pub enum Stack {
    /// Stack tracking not in use
    NoStack,
    /// Stack tracking in use, but nowhere to go
    EmptyStack,
    /// Stack tracking in use, should point to a Loc which is either EmptyStack or Return. If it
    /// points to NoStack, that is a bug.
    // TODO: maybe use a smart constructor here instead?
    Return(Box<Loc>),
}

impl Stack {
    // TODO this is pretty slowly written with unnecessary clones
    fn find(&self, addr: u64) -> Option<Self> {
        match *self {
            Stack::Return(ref tgt) => {
                if addr == tgt.addr {
                    Some(self.clone())
                } else {
                    tgt.stack.find(addr)
                }
            }
            _ => None,
        }
    }
    fn deloop(self) -> Self {
        if let Stack::Return(ref tgt) = self {
            if let Some(new) = tgt.stack.find(tgt.addr) {
                return new;
            }
        }
        self
    }
    pub fn relimit(&mut self, limit: usize) {
        if limit == 0 {
            *self = Stack::EmptyStack;
        }
        if let Stack::Return(ref mut tgt) = *self {
            tgt.stack.relimit(limit - 1);
        }
    }
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Hash)]
pub struct Loc {
    pub file_name: InternedString,
    pub addr: u64,
    pub stack: Stack,
}

impl Loc {
    pub fn is_stacked(&self) -> bool {
        self.stack != Stack::NoStack
    }
}

macro_rules! vec_error {
    ($e:expr) => {{
        let name: ::bap::basic::Result<_> = $e;
        match name {
            Ok(i) => vec![i],
            Err(_) => return Vec::new(),
        }
    }};
}

macro_rules! get_image {
    ($bap:expr, $contents:expr) => {{
        match Image::from_data(&$bap, &$contents) {
            Ok(i) => i,
            Err(_) => return Vec::new(),
        }
    }};
}

pub fn singleton_string(i: &LoadSingletonStringIn) -> Vec<LoadSingletonStringOut> {
    let mut s = BTreeSet::new();
    s.insert(i.name.clone());
    vec![LoadSingletonStringOut { names: s }]
}

// RUSTC-R see whether the let binding can be removed and this warning avoided
#[cfg_attr(feature = "cargo-clippy", allow(let_and_return))]
pub fn dump_segments(i: &LoadDumpSegmentsIn) -> Vec<LoadDumpSegmentsOut> {
    use num_traits::ToPrimitive;
    Bap::with(|bap| {
        let image = get_image!(bap, i.contents);
        let segs = image.segments();
        let out = segs.iter()
            .map(|seg| {
                let mem = seg.memory();
                LoadDumpSegmentsOut {
                    seg_contents: mem.data().to_vec(),
                    start: BitVector::from_basic(&mem.min_addr()).to_u64().unwrap(),
                    end: BitVector::from_basic(&mem.max_addr()).to_u64().unwrap(),
                    read: seg.is_readable(),
                    write: seg.is_writable(),
                    execute: seg.is_executable(),
                }
            })
            .collect();
        out
    })
}

pub fn dump_plt(i: &LoadDumpPltIn) -> Vec<LoadDumpPltOut> {
    use mktemp::Temp;
    use std::fs::File;
    use std::io::Write;
    use std::process::Command;
    let elf_temp = Temp::new_file().unwrap();
    let elf_path_buf = elf_temp.to_path_buf();
    let elf_path = elf_path_buf.to_str().unwrap();
    {
        let mut elf_file = File::create(elf_path).unwrap();
        elf_file.write_all(i.contents).unwrap();
    }
    let out: String = String::from_utf8(
        Command::new("bash")
            .arg("-c")
            .arg(format!("objdump -d {} | grep plt\\>:", elf_path))
            .output()
            .expect("objdump grep pipeline failure")
            .stdout,
    ).unwrap();
    out.split('\n')
        .filter(|x| *x != "")
        .map(|line| {
            let mut it = line.split(' ');
            let addr64 = u64::from_str_radix(it.next().unwrap(), 16).unwrap();
            let unparsed = it.next().unwrap_or_else(|| panic!("No name? {}", line));
            let name = unparsed[1..].split('@').next().unwrap();
            LoadDumpPltOut {
                pad_name: name.to_string(),
                pad_loc: Loc {
                    file_name: InternedString::from_string(i.file_name),
                    addr: addr64,
                    stack: Stack::NoStack,
                },
            }
        })
        .collect()
}

// RUSTC-R see whether the let binding can be removed and this warning avoided
#[cfg_attr(feature = "cargo-clippy", allow(let_and_return))]
pub fn dump_syms(i: &LoadDumpSymsIn) -> Vec<LoadDumpSymsOut> {
    use num_traits::cast::ToPrimitive;
    Bap::with(|bap| {
        let image = get_image!(bap, i.contents);
        let syms = image.symbols();
        let out = syms.iter()
            .map(|sym| LoadDumpSymsOut {
                name: sym.name(),
                loc: Loc {
                    addr: BitVector::from_basic(&sym.memory().min_addr())
                        .to_u64()
                        .unwrap(),
                    file_name: InternedString::from_string(i.file_name),
                    stack: Stack::NoStack,
                },
                end: BitVector::from_basic(&sym.memory().max_addr())
                    .to_u64()
                    .unwrap(),
            })
            .collect();
        out
    })
}

pub fn lift(i: &LoadLiftIn) -> Vec<LoadLiftOut> {
    use num_traits::cast::ToPrimitive;
    // This is super inefficient if we load tons of files in
    if i.loc.file_name != InternedString::from_string(i.file_name) {
        return Vec::new();
    }
    let start = i.seg_start.to_u64().unwrap();
    let addr = i.loc.addr.to_u64().unwrap();
    let end = i.seg_end.to_u64().unwrap();
    if (addr < start) || (addr > end) {
        return vec![];
    }
    if i.loc.is_stacked() {
        return Vec::new();
    }
    vec_error!(Bap::with(|bap| {
        let bin: &[u8] = &i.seg_contents[((addr - start) as usize)..];
        let disas = BasicDisasm::new(bap, *i.arch)?;
        let code = disas.disasm(bin, addr)?;
        let len = code.len() as u64;
        let insn = code.insn();
        let sema = insn.semantics();
        let stmts = sema.iter().map(|bb| Statement::from_basic(&bb)).collect();
        let disasm = insn.to_string();
        let is_call = insn.is_call();
        let is_ret = insn.is_return();
        let fall = addr + len;

        Ok(LoadLiftOut {
            bil: stmts,
            disasm,
            fall: Loc {
                file_name: i.loc.file_name,
                addr: fall,
                stack: i.loc.stack.clone(),
            },
            call: is_call,
            ret: is_ret,
        })
    }))
}

pub fn sema_succ(i: &LoadSemaSuccIn) -> Vec<LoadSemaSuccOut> {
    let (mut targets, fall) = stmt_succ(i.bil);
    if fall {
        targets.push(i.fall.addr);
    }
    let stack = match &i.src.stack {
        &Stack::NoStack => Stack::NoStack,
        s => if i.is_call {
            let mut s = Stack::Return(Box::new(i.fall.clone())).deloop();
            s.relimit(STACK_MAX_DEPTH);
            s
        } else {
            s.clone()
        },
    };
    targets
        .into_iter()
        .map(|x| LoadSemaSuccOut {
            dst: Loc {
                file_name: i.fall.file_name,
                addr: x,
                stack: stack.clone(),
            },
        })
        .collect()
}

fn stmt_succ(stmts: &[Statement]) -> (Vec<u64>, bool) {
    use bap::high::bil::Statement::*;
    use num_traits::ToPrimitive;
    if stmts.is_empty() {
        return (Vec::new(), true);
    }
    match stmts[0] {
        Jump(Expression::Const(ref v)) => (vec![v.to_u64().unwrap()], false),
        Jump(_) => (vec![], false),
        While { ref body, .. } => {
            let (mut tgts, fall) = stmt_succ(body);
            if fall {
                let (mut tgts2, fall2) = stmt_succ(&stmts[1..]);
                let mut tgt_res = Vec::new();
                tgt_res.append(&mut tgts);
                tgt_res.append(&mut tgts2);
                (tgt_res, fall2)
            } else {
                (tgts, fall)
            }
        }
        IfThenElse {
            ref then_clause,
            ref else_clause,
            ..
        } => {
            let (mut then_tgts, then_fall) = stmt_succ(then_clause);
            let (mut else_tgts, else_fall) = stmt_succ(else_clause);
            let fall = then_fall || else_fall;
            let mut tgt_res = Vec::new();
            tgt_res.append(&mut then_tgts);
            tgt_res.append(&mut else_tgts);
            if fall {
                let (mut tgts2, fall2) = stmt_succ(&stmts[1..]);
                tgt_res.append(&mut tgts2);
                (tgt_res, fall2)
            } else {
                (tgt_res, fall)
            }
        }
        _ => stmt_succ(&stmts[1..]),
    }
}

fn is_const(e: &Expression) -> bool {
    match *e {
        Expression::Const(_) => true,
        _ => false,
    }
}

pub fn is_computed_jump(i: &LoadIsComputedJumpIn) -> Vec<LoadIsComputedJumpOut> {
    for stmt in i.bil.iter() {
        match *stmt {
            Statement::Jump(ref e) if !is_const(e) => return vec![LoadIsComputedJumpOut {}],
            _ => (),
        }
    }
    vec![]
}

pub fn get_arch(i: &LoadGetArchIn) -> Vec<LoadGetArchOut> {
    Bap::with(|bap| {
        let image = get_image!(bap, i.contents);
        vec![LoadGetArchOut {
            arch: image.arch().unwrap(),
        }]
    })
}

pub fn malloc_name(func_name: &str) -> bool {
    func_name.contains("malloc")
        || func_name.contains("calloc")
        || func_name == "_Znam"
        || func_name == "_Znwm"
}

pub fn is_malloc_name(i: &LoadIsMallocNameIn) -> Vec<LoadIsMallocNameOut> {
    if malloc_name(i.func_name) {
        vec![LoadIsMallocNameOut {}]
    } else {
        Vec::new()
    }
}

pub fn is_free_name(i: &LoadIsFreeNameIn) -> Vec<LoadIsFreeNameOut> {
    let s = i.func_name;
    if (s == "free") || (s == "g_free") || (s == "_ZdaPv") || (s == "_ZdlPvm") {
        vec![LoadIsFreeNameOut { args: vec![0] }]
    } else if s == "qfree" {
        vec![LoadIsFreeNameOut { args: vec![1] }]
    } else {
        Vec::new()
    }
}

pub fn is_returning_name(i: &LoadIsReturningNameIn) -> Vec<LoadIsReturningNameOut> {
    let s = i.func_name;
    if s == "abort" || s == "__stack_chk_fail" || s == "exit" {
        Vec::new()
    } else {
        vec![LoadIsReturningNameOut {}]
    }
}

pub fn call_site_stack(i: &LoadCallSiteStackIn) -> Vec<LoadCallSiteStackOut> {
    let mut target_loc_adjusted = i.target_loc.clone();
    if !i.call_loc.is_stacked() {
        target_loc_adjusted.stack = Stack::NoStack;
    } else {
        target_loc_adjusted.stack = i.pad_loc.stack.clone();
    }
    vec![LoadCallSiteStackOut {
        target_loc_adjusted,
    }]
}

pub fn called_unstacked(i: &LoadCalledUnstackedIn) -> Vec<LoadCalledUnstackedOut> {
    if !i.loc.is_stacked() {
        vec![LoadCalledUnstackedOut {
            locs: vec![i.loc.clone()],
        }]
    } else {
        Vec::new()
    }
}

pub fn called_filter(i: &LoadCalledFilterIn) -> Vec<LoadCalledFilterOut> {
    trace!("locs={:?}, loc={}", i.locs, i.loc);
    if i.locs.contains(i.loc) || i.loc.is_stacked() {
        Vec::new()
    } else {
        vec![LoadCalledFilterOut {}]
    }
}
