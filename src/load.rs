use bap::basic::{Bap, BasicDisasm, Image};
use bap::high::bil::{Expression, Statement};
use bap::high::bitvector::BitVector;
use datalog::*;
use interned_string::InternedString;
use std::collections::BTreeSet;

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Hash, Copy)]
pub struct Loc {
    pub file_name: InternedString,
    pub addr: u64,
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
            let unparsed = it.next().expect(&format!("No name? {}", line));
            let name = unparsed[1..].split('@').next().unwrap();
            LoadDumpPltOut {
                pad_name: name.to_string(),
                pad_loc: Loc {
                    file_name: InternedString::from_string(i.file_name),
                    addr: addr64,
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
    targets
        .into_iter()
        .map(|x| LoadSemaSuccOut {
            dst: Loc {
                file_name: i.fall.file_name,
                addr: x,
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

pub fn is_malloc_name(i: &LoadIsMallocNameIn) -> Vec<LoadIsMallocNameOut> {
    if i.func_name.contains("malloc") || i.func_name.contains("calloc") {
        vec![LoadIsMallocNameOut {}]
    } else {
        Vec::new()
    }
}

pub fn is_free_name(i: &LoadIsFreeNameIn) -> Vec<LoadIsFreeNameOut> {
    let s = i.func_name;
    if (s == "free") || (s == "g_free") {
        vec![LoadIsFreeNameOut { args: vec![0] }]
    } else if s == "qfree" {
        vec![LoadIsFreeNameOut { args: vec![1] }]
    } else {
        Vec::new()
    }
}
