use datalog::*;
use bap::high::bitvector::BitVector;
use bap::basic::{Bap, BasicDisasm, Image};
use bap::high::bil::{Expression, Statement, Type, Variable};
use std::collections::{BTreeMap, BTreeSet};
use steensgaard;
use datalog::Loc;

macro_rules! vec_error {
    ($e:expr) => {{
        let name: ::bap::basic::Result<_> = $e;
        match name {
            Ok(i) => vec![i],
            Err(_) => return Vec::new()
        }
    }}
}

macro_rules! get_image {
    ($bap:expr, $contents:expr) => {{
        match Image::from_data(&$bap, &$contents) {
            Ok(i) => i,
            Err(_) => return Vec::new()
        }
    }}
}

pub fn free_rdi(i: &FuncsFreeRdiIn) -> Vec<FuncsFreeRdiOut> {
    i.dc["RDI"]
        .iter()
        .map(|loc| FuncsFreeRdiOut {
            rdi: steensgaard::Var::Register {
                site: loc.clone(),
                tmp: false,
                register: "RDI".to_string(),
            },
        })
        .collect()
}

pub fn expand_vars(i: &FuncsExpandVarsIn) -> Vec<FuncsExpandVarsOut> {
    i.vs
        .iter()
        .map(|v| FuncsExpandVarsOut { v2: v.clone() })
        .collect()
}

pub fn reads_vars(i: &FuncsReadsVarsIn) -> Vec<FuncsReadsVarsOut> {
    steensgaard::extract_var_use(i.bil, i.dc.clone(), i.loc, i.base)
        .into_iter()
        .map(|v| FuncsReadsVarsOut { v: v })
        .collect()
}

pub fn expand_registers(i: &FuncsExpandRegistersIn) -> Vec<FuncsExpandRegistersOut> {
    let mut def_loc_singleton = BTreeSet::new();
    def_loc_singleton.insert(i.def_loc.clone());
    i.registers
        .iter()
        .map(|s| FuncsExpandRegistersOut {
            register: s.clone(),
            def_loc_singleton: def_loc_singleton.clone(),
        })
        .collect()
}

pub fn steens_solve(i: &FuncsSteensSolveIn) -> Vec<FuncsSteensSolveOut> {
    println!("Problem state");
    for c in i.cs {
        println!("{}", c);
    }
    steensgaard::constraints_to_may_alias(i.cs.clone())
        .into_iter()
        .map(|vs| FuncsSteensSolveOut { vs: vs })
        .collect()
}

pub fn singleton_string(i: &FuncsSingletonStringIn) -> Vec<FuncsSingletonStringOut> {
    let mut s = BTreeSet::new();
    s.insert(i.name.clone());
    vec![FuncsSingletonStringOut { names: s }]
}

pub fn steens_expando(i: &FuncsSteensExpandoIn) -> Vec<FuncsSteensExpandoOut> {
    i.vs
        .iter()
        .map(|v| FuncsSteensExpandoOut { v: v.clone() })
        .collect()
}

fn is_normal_reg(r: &Variable) -> bool {
    match r.type_ {
        Type::Immediate(x) => x > 1,
        _ => false,
    }
}

fn defines_stmt(stmt: &Statement, defs: &mut BTreeSet<String>) {
    match *stmt {
        Statement::Move { ref lhs, .. } => {
            if !lhs.tmp && is_normal_reg(lhs) {
                defs.insert(lhs.name.clone());
            }
        }
        Statement::While { ref body, .. } => for stmt in body {
            defines_stmt(stmt, defs);
        },
        Statement::IfThenElse {
            ref then_clause,
            ref else_clause,
            ..
        } => {
            for stmt in then_clause {
                defines_stmt(stmt, defs);
            }
            for stmt in else_clause {
                defines_stmt(stmt, defs);
            }
        }
        _ => (),
    }
}

pub fn gen_constraints(i: &FuncsGenConstraintsIn) -> Vec<FuncsGenConstraintsOut> {
    steensgaard::extract_constraints(i.bil, i.dc.clone(), i.loc, i.base)
        .into_iter()
        .map(|c| FuncsGenConstraintsOut { c: vec![c] })
        .collect()
}

pub fn def_chain(i: &FuncsDefChainIn) -> Vec<FuncsDefChainOut> {
    let mut out = BTreeMap::new();
    out.insert(i.register.clone(), i.defs.clone());
    vec![FuncsDefChainOut { dc: out }]
}

pub fn malloc_constraint(i: &FuncsMallocConstraintIn) -> Vec<FuncsMallocConstraintOut> {
    use steensgaard::*;
    vec![
        FuncsMallocConstraintOut {
            c: vec![
                Constraint::AddrOf {
                    a: Var::Register {
                        site: i.loc.clone(),
                        register: "RAX".to_string(),
                        tmp: false,
                    },
                    b: Var::Alloc {
                        site: i.loc.clone(),
                    },
                },
            ],
        },
    ]
}

pub fn defines(i: &FuncsDefinesIn) -> Vec<FuncsDefinesOut> {
    let mut defs = BTreeSet::new();
    for stmt in i.bil {
        defines_stmt(stmt, &mut defs);
    }
    vec![
        FuncsDefinesOut {
            registers: defs.into_iter().collect(),
        },
    ]
}

pub fn exclude_registers(i: &FuncsExcludeRegistersIn) -> Vec<FuncsExcludeRegistersOut> {
    if i.prev_defines.contains(i.register) {
        Vec::new()
    } else {
        vec![FuncsExcludeRegistersOut {}]
    }
}

// RUSTC-R see whether the let binding can be removed and this warning avoided
#[cfg_attr(feature = "cargo-clippy", allow(let_and_return))]
pub fn dump_segments(i: &FuncsDumpSegmentsIn) -> Vec<FuncsDumpSegmentsOut> {
    Bap::with(|bap| {
        let image = get_image!(bap, i.contents);
        let segs = image.segments();
        let out = segs.iter()
            .map(|seg| {
                let mem = seg.memory();
                FuncsDumpSegmentsOut {
                    seg_contents: mem.data().to_vec(),
                    start: BitVector::from_basic(&mem.min_addr()),
                    end: BitVector::from_basic(&mem.max_addr()),
                    read: seg.is_readable(),
                    write: seg.is_writable(),
                    execute: seg.is_executable(),
                }
            })
            .collect();
        out
    })
}

pub fn dump_plt(i: &FuncsDumpPltIn) -> Vec<FuncsDumpPltOut> {
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
            let addr = BitVector::from_u64(addr64, 64);
            let unparsed = it.next().expect(&format!("No name? {}", line));
            let name = unparsed[1..].split('@').next().unwrap();
            FuncsDumpPltOut {
                pad_name: name.to_string(),
                pad_loc: Loc {
                    file_name: i.file_name.clone(),
                    addr: addr,
                },
            }
        })
        .collect()
}

// RUSTC-R see whether the let binding can be removed and this warning avoided
#[cfg_attr(feature = "cargo-clippy", allow(let_and_return))]
pub fn dump_syms(i: &FuncsDumpSymsIn) -> Vec<FuncsDumpSymsOut> {
    Bap::with(|bap| {
        let image = get_image!(bap, i.contents);
        let syms = image.symbols();
        let out = syms.iter()
            .map(|sym| FuncsDumpSymsOut {
                name: sym.name(),
                loc: Loc {
                    addr: BitVector::from_basic(&sym.memory().min_addr()),
                    file_name: i.file_name.clone(),
                },
                end: BitVector::from_basic(&sym.memory().max_addr()),
            })
            .collect();
        out
    })
}

pub fn lift(i: &FuncsLiftIn) -> Vec<FuncsLiftOut> {
    use num_traits::cast::ToPrimitive;
    // This is super inefficient if we load tons of files in
    if i.loc.file_name != *i.file_name {
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

        Ok(FuncsLiftOut {
            bil: stmts,
            disasm: disasm,
            fall: Loc {
                file_name: i.file_name.clone(),
                addr: BitVector::from_u64(fall, 64),
            },
            call: is_call,
            ret: is_ret,
        })
    }))
}

pub fn sema_succ(i: &FuncsSemaSuccIn) -> Vec<FuncsSemaSuccOut> {
    let (mut targets, fall) = stmt_succ(i.bil);
    if fall {
        targets.push(i.fall.addr.clone());
    }
    targets
        .into_iter()
        .map(|x| FuncsSemaSuccOut {
            dst: Loc {
                file_name: i.fall.file_name.clone(),
                addr: x,
            },
        })
        .collect()
}

fn stmt_succ(stmts: &[Statement]) -> (Vec<BitVector>, bool) {
    use bap::high::bil::Statement::*;
    if stmts.is_empty() {
        return (Vec::new(), true);
    }
    match stmts[0] {
        Jump(Expression::Const(ref v)) => (vec![v.clone()], false),
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

pub fn is_computed_jump(i: &FuncsIsComputedJumpIn) -> Vec<FuncsIsComputedJumpOut> {
    for stmt in i.bil.iter() {
        match *stmt {
            Statement::Jump(ref e) if !is_const(e) => return vec![FuncsIsComputedJumpOut {}],
            _ => (),
        }
    }
    vec![]
}

pub fn get_arch(i: &FuncsGetArchIn) -> Vec<FuncsGetArchOut> {
    Bap::with(|bap| {
        let image = get_image!(bap, i.contents);
        vec![
            FuncsGetArchOut {
                arch: image.arch().unwrap(),
            },
        ]
    })
}

pub fn is_malloc_name(i: &FuncsIsMallocNameIn) -> Vec<FuncsIsMallocNameOut> {
    if i.func_name.contains("malloc") || i.func_name.contains("calloc") {
        vec![FuncsIsMallocNameOut {}]
    } else {
        Vec::new()
    }
}

pub fn is_free_name(i: &FuncsIsFreeNameIn) -> Vec<FuncsIsFreeNameOut> {
    let s = i.func_name;
    if (s == "free") || (s == "qfree") || (s == "g_free") {
        vec![FuncsIsFreeNameOut {}]
    } else {
        Vec::new()
    }
}
