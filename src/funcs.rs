use datalog::*;
use bap::high::bitvector::BitVector;
use bap::basic::{Bap, BasicDisasm, Image};
use bap::high::bil::{Expression, Statement, Type, Variable};
use avar::AVar;
use std::collections::{BTreeMap, BTreeSet};
use steensgaard;

const MAX_CHOP: usize = 3;
const MAX_STACK: usize = 5;
const MAX_PATH: usize = 50;

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
pub fn steens_solve(i: &FuncsSteensSolveIn) -> Vec<FuncsSteensSolveOut> {
    steensgaard::constraints_to_may_alias(i.cs.clone())
        .into_iter()
        .map(|vs| FuncsSteensSolveOut { vs: vs })
        .collect()
}

pub fn exclude_names(i: &FuncsExcludeNamesIn) -> Vec<FuncsExcludeNamesOut> {
    if i.names.contains(i.pad_name) {
        Vec::new()
    } else {
        vec![FuncsExcludeNamesOut {}]
    }
}

pub fn singleton_string(i: &FuncsSingletonStringIn) -> Vec<FuncsSingletonStringOut> {
    let mut s = BTreeSet::new();
    s.insert(i.name.clone());
    vec![FuncsSingletonStringOut { names: s }]
}

pub fn inc_path(i: &FuncsIncPathIn) -> Vec<FuncsIncPathOut> {
    if i.steps < MAX_PATH {
        vec![
            FuncsIncPathOut {
                steps_plus_one: i.steps + 1,
            },
        ]
    } else {
        Vec::new()
    }
}

pub fn inc_path2(i: &FuncsIncPath2In) -> Vec<FuncsIncPath2Out> {
    if i.steps < MAX_PATH {
        vec![
            FuncsIncPath2Out {
                steps_plus_one: i.steps + 1,
            },
        ]
    } else {
        Vec::new()
    }
}

pub fn path_start_heap(i: &FuncsPathStartHeapIn) -> Vec<FuncsPathStartHeapOut> {
    // If alias_set is 0, short circuit to do nothing, this was malloc not heap
    if i.alias_set == 0 {
        return Vec::new();
    }

    // TODO factor duplicate code
    // Do exactly as heap_init, but only return the value that matches alias_set
    let mut hs = Vec::new();
    for stmt in i.sema.iter() {
        tprop::heap_prop(stmt, &mut hs)
    }
    hs[i.alias_set - 1]
        .iter()
        .map(|var| FuncsPathStartHeapOut {
            heap_var: var.clone(),
        })
        .collect()
}

pub fn heap_init(i: &FuncsHeapInitIn) -> Vec<FuncsHeapInitOut> {
    let mut hs = Vec::new();
    for stmt in i.sema.iter() {
        tprop::heap_prop(stmt, &mut hs);
    }

    hs.into_iter()
        .enumerate()
        .flat_map(|(idx, h): (usize, _)| {
            // RUSTC-R I shouldn't need to tell it to copy a usize into a closure...
            let q = idx;
            h.into_iter().map(move |var| FuncsHeapInitOut {
                a_s: q + 1,
                heap_var: var,
            })
        })
        .collect()
}

pub fn flow_use(i: &FuncsFlowUseIn) -> Vec<FuncsFlowUseOut> {
    if tprop::sema_uses(i.bil, i.a_var) {
        vec![FuncsFlowUseOut {}]
    } else {
        Vec::new()
    }
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
pub fn trace_use(i: &FuncsTraceUseIn) -> Vec<FuncsTraceUseOut> {
    if tprop::sema_uses(i.bil, i.a_var) {
        vec![FuncsTraceUseOut {}]
    } else {
        Vec::new()
    }
}

pub fn call_stack_chop(i: &FuncsCallStackChopIn) -> Vec<FuncsCallStackChopOut> {
    call_stack_chop_inner(i.stack.as_slice(), i.chop, i.addr1, i.file1, i.ret_addr)
        .into_iter()
        .map(|(chop2, stack2)| FuncsCallStackChopOut {
            chop2: chop2,
            stack2: stack2,
        })
        .collect()
}
pub fn call_stack_chop_trace(i: &FuncsCallStackChopTraceIn) -> Vec<FuncsCallStackChopTraceOut> {
    call_stack_chop_inner(i.stack.as_slice(), i.chop, i.addr1, i.file1, i.ret_addr)
        .into_iter()
        .map(|(chop2, stack2)| FuncsCallStackChopTraceOut {
            chop2: chop2,
            stack2: stack2,
        })
        .collect()
}

pub fn call_stack_chop_inner(
    stack: &[(String, BitVector)],
    chop: &[BitVector],
    addr1: &BitVector,
    file1: &String,
    ret_addr: &BitVector,
) -> Vec<(Vec<BitVector>, Vec<(String, BitVector)>)> {
    let chop2 = if !chop.contains(addr1) {
        if chop.len() >= MAX_CHOP {
            return Vec::new();
        }
        let mut chop2 = chop.to_vec();
        chop2.push(addr1.clone());
        chop2
    } else {
        chop.to_vec()
    };
    if stack.len() >= MAX_STACK {
        return Vec::new();
    }
    let mut stack2 = stack.to_vec();
    stack2.push((file1.clone(), ret_addr.clone()));
    vec![(chop2, stack2)]
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
    steensgaard::extract_constraints(i.bil, i.dc.clone(), i.addr, i.func_base)
        .into_iter()
        .map(|c| FuncsGenConstraintsOut { c: vec![c] })
        .collect()
}

pub fn def_chain(i: &FuncsDefChainIn) -> Vec<FuncsDefChainOut> {
    let mut out = BTreeMap::new();
    out.insert(i.register.clone(), i.def_addr.clone());
    vec![FuncsDefChainOut { dc: out }]
}

pub fn malloc_constraint(i: &FuncsMallocConstraintIn) -> Vec<FuncsMallocConstraintOut> {
    use steensgaard::*;
    vec![
        FuncsMallocConstraintOut {
            c: vec![
                Constraint::AddrOf {
                    a: Var::Register {
                        site: i.addr.clone(),
                        register: "RAX".to_string(),
                    },
                    b: Var::Alloc {
                        site: i.addr.clone(),
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

pub fn expand_registers(i: &FuncsExpandRegistersIn) -> Vec<FuncsExpandRegistersOut> {
    let mut def_addr_singleton = BTreeSet::new();
    def_addr_singleton.insert(i.def_addr.clone());
    i.registers
        .iter()
        .map(|s| FuncsExpandRegistersOut {
            register: s.clone(),
            def_addr_singleton: def_addr_singleton.clone(),
        })
        .collect()
}

pub fn ret_stack(i: &FuncsRetStackIn) -> Vec<FuncsRetStackOut> {
    let mut stack = i.stack.clone();
    match stack.pop() {
        Some((name, addr)) => vec![
            FuncsRetStackOut {
                stack2: stack,
                file2: name,
                addr2: addr,
            },
        ],
        None => Vec::new(),
    }
}
pub fn ret_stack_trace(i: &FuncsRetStackTraceIn) -> Vec<FuncsRetStackTraceOut> {
    let mut stack = i.stack.clone();
    match stack.pop() {
        Some((name, addr)) => vec![
            FuncsRetStackTraceOut {
                stack2: stack,
                file2: name,
                addr2: addr,
            },
        ],
        None => Vec::new(),
    }
}

pub fn ret_no_stack(i: &FuncsRetNoStackIn) -> Vec<FuncsRetNoStackOut> {
    let mut chop2 = i.chop.clone();
    if !i.chop.contains(i.dst_addr) {
        if i.chop.len() >= MAX_CHOP {
            return Vec::new();
        }
        chop2.push(i.dst_addr.clone())
    }
    vec![FuncsRetNoStackOut { chop2: chop2 }]
}

pub fn ret_no_stack_trace(i: &FuncsRetNoStackTraceIn) -> Vec<FuncsRetNoStackTraceOut> {
    let mut chop2 = i.chop.clone();
    if !i.chop.contains(i.dst_addr) {
        if i.chop.len() >= MAX_CHOP {
            return Vec::new();
        }
        chop2.push(i.dst_addr.clone())
    }
    vec![FuncsRetNoStackTraceOut { chop2: chop2 }]
}

pub fn clobbers(i: &FuncsClobbersIn) -> Vec<FuncsClobbersOut> {
    if i.a_var.is_clobbered() {
        Vec::new()
    } else {
        vec![FuncsClobbersOut {}]
    }
}

pub fn exclude_registers(i: &FuncsExcludeRegistersIn) -> Vec<FuncsExcludeRegistersOut> {
    if i.prev_defines.contains(i.register) {
        Vec::new()
    } else {
        vec![FuncsExcludeRegistersOut {}]
    }
}

pub fn clobbers_trace(i: &FuncsClobbersTraceIn) -> Vec<FuncsClobbersTraceOut> {
    if i.a_var.is_clobbered() {
        Vec::new()
    } else {
        vec![FuncsClobbersTraceOut {}]
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
                pad_addr: addr,
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
                start: BitVector::from_basic(&sym.memory().min_addr()),
                end: BitVector::from_basic(&sym.memory().max_addr()),
            })
            .collect();
        out
    })
}

pub fn lift(i: &FuncsLiftIn) -> Vec<FuncsLiftOut> {
    use num_traits::cast::ToPrimitive;
    let start = i.seg_start.to_u64().unwrap();
    let addr = i.addr.to_u64().unwrap();
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
            fall: BitVector::from_u64(fall, 64),
            call: is_call,
            ret: is_ret,
        })
    }))
}

pub fn sema_succ(i: &FuncsSemaSuccIn) -> Vec<FuncsSemaSuccOut> {
    let (mut targets, fall) = stmt_succ(i.bil);
    if fall {
        targets.push(i.fall.clone());
    }
    targets
        .into_iter()
        .map(|x| FuncsSemaSuccOut { dst_addr: x })
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

mod tprop {
    use bap::high::bitvector::BitVector;
    use bap::high::bil::{BinOp, Expression, Statement, Type, Variable};
    use avar::AVar;
    use std::collections::HashSet;
    use std::iter::FromIterator;

    pub fn heap_prop(stmt: &Statement, ks: &mut Vec<HashSet<AVar>>) {
        let mut all_tracked = HashSet::new();
        for ass in ks.iter_mut() {
            all_tracked.extend(ass.iter().cloned());
            *ass = HashSet::from_iter(
                proc_stmt(ass.iter().cloned().collect::<Vec<_>>(), stmt).into_iter(),
            );
        }
        match *stmt {
            Statement::Move { ref lhs, ref rhs } if is_reg(lhs) => {
                match *rhs {
                    Expression::Load { ref index, .. } => {
                        let hvar = AVar {
                            inner: lhs.clone(),
                            offset: None,
                        };
                        if !all_tracked.contains(&hvar) {
                            // This variable doesn't contain a tracked pointer already
                            match promote_idx(index) {
                                Some(ref hv) if !stack_hvar(hv) => {
                                    let mut x = HashSet::new();
                                    x.insert(hvar);
                                    ks.push(x);
                                }
                                _ => (),
                            }
                        }
                    }
                    _ => (),
                }
            }
            _ => (),
        }
    }
    fn stack_hvar(hv: &AVar) -> bool {
        let name = &hv.inner.name;
        (name == "RBP") || (name == "RSP")
    }

    fn hv_match(bad: &[AVar], e: &Expression) -> bool {
        match *e {
            Expression::Var(ref v) => bad.contains(&AVar {
                inner: v.clone(),
                offset: None,
            }),
            Expression::Load { index: ref idx, .. } => match promote_idx(idx) {
                Some(hv) => bad.contains(&hv),
                None => false,
            },
            _ => false,
        }
    }

    fn is_reg(r: &Variable) -> bool {
        match r.type_ {
            Type::Immediate(_) => true,
            _ => false,
        }
    }

    fn is_mem(m: &Variable) -> bool {
        match m.type_ {
            Type::Memory { .. } => true,
            _ => false,
        }
    }

    fn add_hvar(mut bad: Vec<AVar>, hv: AVar) -> Vec<AVar> {
        if !bad.contains(&hv) {
            bad.push(hv)
        }
        bad
    }

    fn rem_hvar(bad: Vec<AVar>, hv: &AVar) -> Vec<AVar> {
        bad.into_iter().filter(|x| x != hv).collect()
    }

    fn promote_idx(idx: &Expression) -> Option<AVar> {
        match *idx {
            Expression::Var(ref v) => Some(AVar {
                inner: v.clone(),
                offset: Some(BitVector::from_u64(0, 64)),
            }),
            Expression::BinOp {
                op: BinOp::Add,
                ref lhs,
                ref rhs,
            } => match **lhs {
                Expression::Var(ref v) => match **rhs {
                    Expression::Const(ref bv) => Some(AVar {
                        inner: v.clone(),
                        offset: Some(bv.clone()),
                    }),
                    _ => None,
                },
                Expression::Const(ref bv) => match **rhs {
                    Expression::Var(ref v) => Some(AVar {
                        inner: v.clone(),
                        offset: Some(bv.clone()),
                    }),
                    _ => None,
                },
                _ => None,
            },
            _ => None,
        }
    }

    fn check_idx(idx: &Expression, var: &AVar) -> bool {
        match *idx {
            Expression::Var(ref v) => (var.offset == None) && (var.inner == *v),
            Expression::BinOp {
                ref lhs, ref rhs, ..
            } => check_idx(lhs, var) || check_idx(rhs, var),
            _ => false,
        }
    }

    fn deref_var_expr(expr: &Expression, var: &AVar) -> bool {
        match *expr {
            Expression::Load { index: ref idx, .. } | Expression::Store { index: ref idx, .. } => {
                check_idx(idx, var)
            }
            Expression::Cast { ref arg, .. } => deref_var_expr(arg, var),
            _ => false,
        }
    }

    fn deref_var_step(stmt: &Statement, var: &AVar) -> bool {
        use bap::high::bil::Statement::Move;
        match *stmt {
            Move { rhs: ref e, .. } => deref_var_expr(e, var),
            _ => false,
        }
    }

    pub fn sema_uses(sema: &[Statement], var: &AVar) -> bool {
        let mut vars = vec![var.clone()];
        for stmt in sema {
            for var in &vars {
                if deref_var_step(stmt, var) {
                    return true;
                }
            }
            vars = proc_stmt(vars, stmt);
        }
        false
    }

    pub fn proc_stmt(bad: Vec<AVar>, stmt: &Statement) -> Vec<AVar> {
        use bap::high::bil::Statement::*;
        match *stmt {
            // Register update
            Move {
                lhs: ref reg,
                rhs: ref e,
            } if is_reg(reg) =>
            {
                if hv_match(&bad, e) {
                    add_hvar(
                        bad,
                        AVar {
                            inner: reg.clone(),
                            offset: None,
                        },
                    )
                } else {
                    rem_hvar(
                        bad,
                        &AVar {
                            inner: reg.clone(),
                            offset: None,
                        },
                    )
                }
            }
            // Memory Write
            Move {
                lhs: ref mem,
                rhs: ref e,
            } if is_mem(mem) =>
            {
                match *e {
                    Expression::Store {
                        index: ref idx,
                        value: ref val,
                        ..
                    } => {
                        if hv_match(&bad, val) {
                            promote_idx(idx).map_or(bad.clone(), |hidx| add_hvar(bad, hidx))
                        } else {
                            promote_idx(idx).map_or(bad.clone(), |hidx| rem_hvar(bad, &hidx))
                        }
                    }
                    _ => bad,
                }
            }
            _ => bad,
        }
    }
}

pub fn xfer_taint(i: &FuncsXferTaintIn) -> Vec<FuncsXferTaintOut> {
    xfer_taint_inner(i.bil, i.a_var)
        .into_iter()
        .map(|a_var2| FuncsXferTaintOut { a_var2: a_var2 })
        .collect()
}

pub fn xfer_taint_trace(i: &FuncsXferTaintTraceIn) -> Vec<FuncsXferTaintTraceOut> {
    xfer_taint_inner(i.bil, i.a_var)
        .into_iter()
        .filter_map(|a_var2| {
            if i.steps < MAX_PATH {
                Some(FuncsXferTaintTraceOut {
                    a_var2: a_var2,
                    steps_plus_one: i.steps + 1,
                })
            } else {
                None
            }
        })
        .collect()
}

pub fn xfer_taint_inner(bil: &[Statement], a_var: &AVar) -> Vec<AVar> {
    bil.iter()
        .fold(vec![a_var.clone()], tprop::proc_stmt)
        .into_iter()
        .filter(|v| v.not_temp())
        .collect()
}
