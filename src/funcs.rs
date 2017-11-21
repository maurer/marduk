use datalog::*;
use bap::high::bitvector::BitVector;
use bap::basic::{Bap, BasicDisasm, Image};
use bap::high::bil::{Expression, Statement};

const MAX_CHOP: usize = 3;
const MAX_STACK: usize = 5;

macro_rules! vec_error {
    ($e:expr) => {{
        let name: ::bap::basic::Result<_> = $e;
        match name {
            Ok(i) => vec![i],
            Err(e) => return Vec::new()
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

pub fn flow_use(i: &FuncsFlowUseIn) -> Vec<FuncsFlowUseOut> {
    if tprop::sema_uses(i.bil, i.a_var) {
        vec![FuncsFlowUseOut {}]
    } else {
        Vec::new()
    }
}

pub fn call_stack_chop(i: &FuncsCallStackChopIn) -> Vec<FuncsCallStackChopOut> {
    let chop2 = if !i.chop.contains(i.addr1) {
        if i.chop.len() >= MAX_CHOP {
            return Vec::new();
        }
        let mut chop2 = i.chop.clone();
        chop2.push(i.addr1.clone());
        chop2
    } else {
        i.chop.clone()
    };
    if i.stack.len() >= MAX_STACK {
        return Vec::new();
    }
    let mut stack2 = i.stack.clone();
    stack2.push((i.file1.clone(), i.ret_addr.clone()));
    vec![
        FuncsCallStackChopOut {
            chop2: chop2,
            stack2: stack2,
        },
    ]
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

pub fn clobbers(i: &FuncsClobbersIn) -> Vec<FuncsClobbersOut> {
    if i.a_var.is_clobbered() {
        Vec::new()
    } else {
        vec![FuncsClobbersOut {}]
    }
}

pub fn dump_segments(i: &FuncsDumpSegmentsIn) -> Vec<FuncsDumpSegmentsOut> {
    Bap::with(|bap| {
        let image = get_image!(bap, i.contents);
        let segs = image.segments();
        let out = {
            segs.iter()
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
                .collect()
        };
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
    out.split("\n")
        .filter(|x| *x != "")
        .map(|line| {
            let mut it = line.split(" ");
            let addr64 = u64::from_str_radix(it.next().unwrap(), 16).unwrap();
            let addr = BitVector::from_u64(addr64, 64);
            let unparsed = it.next().expect(&format!("No name? {}", line));
            let name = unparsed[1..].split("@").next().unwrap();
            FuncsDumpPltOut {
                pad_name: name.to_string(),
                pad_addr: addr,
            }
        })
        .collect()
}

pub fn dump_syms(i: &FuncsDumpSymsIn) -> Vec<FuncsDumpSymsOut> {
    Bap::with(|bap| {
        let image = get_image!(bap, i.contents);
        let out = {
            let syms = image.symbols();
            let out = syms.iter()
                .map(|sym| {
                    FuncsDumpSymsOut {
                        name: sym.name(),
                        start: BitVector::from_basic(&sym.memory().min_addr()),
                        end: BitVector::from_basic(&sym.memory().max_addr()),
                    }
                })
                .collect();
            out
        };
        out
    })
}

pub fn lift(i: &FuncsLiftIn) -> Vec<FuncsLiftOut> {
    use std::fmt::Write;
    use num_traits::cast::ToPrimitive;
    let start = i.seg_start.to_u64().unwrap();
    let addr = i.addr.to_u64().unwrap();
    let end = i.seg_end.to_u64().unwrap();
    if (addr < start) || (addr > end) {
        return vec![];
    }
    vec_error!(Bap::with(|bap| {
        let mut stmts = Vec::new();
        let mut disasm = String::new();
        let mut is_call = false;
        let mut is_ret = false;
        let mut fall: u64 = 0;
        let mut bin: &[u8] = &i.seg_contents[((addr - start) as usize)..];
        let mut may_jump = false;
        let mut first = true;
        let mut addr = addr;
        while !may_jump {
            let disas = BasicDisasm::new(&bap, *i.arch)?;
            let code = disas.disasm(&bin, addr)?;
            let len = code.len() as u64;
            let insn = code.insn();
            let sema = insn.semantics();
            if !first && (insn.is_call() || insn.is_return()) {
                break;
            }
            first = false;
            stmts.extend(sema.iter().map(|bb| Statement::from_basic(&bb)));
            write!(&mut disasm, "{}\n", insn.to_string()).unwrap();
            is_call = insn.is_call();
            is_ret = insn.is_return();
            fall = addr + len;
            may_jump = insn.may_affect_control_flow();
            if !may_jump {
                bin = &bin[(len as usize)..];
                addr = fall;
            }
        }

        disasm.pop();
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
    let (mut targets, fall) = stmt_succ(&i.bil);
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
    if stmts.len() == 0 {
        return (Vec::new(), true);
    }
    match &stmts[0] {
        &Jump(Expression::Const(ref v)) => (vec![v.clone()], false),
        &Jump(_) => (vec![], false),
        &While { cond: _, ref body } => {
            let (mut tgts, fall) = stmt_succ(&body);
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
        &IfThenElse {
            cond: _,
            ref then_clause,
            ref else_clause,
        } => {
            let (mut then_tgts, then_fall) = stmt_succ(&then_clause);
            let (mut else_tgts, else_fall) = stmt_succ(&else_clause);
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

pub fn is_computed_jump(i: &FuncsIsComputedJumpIn) -> Vec<FuncsIsComputedJumpOut> {
    for stmt in i.bil.iter() {
        match *stmt {
            Statement::Jump(Expression::Const(_)) => (),
            Statement::Jump(_) => return vec![FuncsIsComputedJumpOut {}],
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
    fn hv_match(bad: &Vec<AVar>, e: &Expression) -> bool {
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

    fn rem_hvar(bad: Vec<AVar>, hv: AVar) -> Vec<AVar> {
        bad.into_iter().filter(|x| *x != hv).collect()
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
                op: _,
                ref lhs,
                ref rhs,
            } => check_idx(lhs, var) || check_idx(rhs, var),
            _ => false,
        }
    }

    fn deref_var_expr(expr: &Expression, var: &AVar) -> bool {
        match *expr {
            Expression::Load { index: ref idx, .. } => check_idx(idx, var),

            Expression::Store { index: ref idx, .. } => check_idx(idx, var),

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
        return false;
    }

    pub fn proc_stmt(bad: Vec<AVar>, stmt: &Statement) -> Vec<AVar> {
        use bap::high::bil::Statement::*;
        match *stmt {
            // Register update
            Move {
                lhs: ref reg,
                rhs: ref e,
            } if is_reg(&reg) =>
            {
                if hv_match(&bad, &e) {
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
                        AVar {
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
            } if is_mem(&mem) =>
            {
                match *e {
                    Expression::Store {
                        memory: _,
                        index: ref idx,
                        value: ref val,
                        endian: _,
                        size: _,
                    } => if hv_match(&bad, &val) {
                        promote_idx(idx).map_or(bad.clone(), |hidx| add_hvar(bad, hidx))
                    } else {
                        promote_idx(idx).map_or(bad.clone(), |hidx| rem_hvar(bad, hidx))
                    },
                    _ => bad,
                }
            }
            _ => bad,
        }
    }
}

pub fn xfer_taint(i: &FuncsXferTaintIn) -> Vec<FuncsXferTaintOut> {
    i.bil
        .iter()
        .fold(vec![i.a_var.clone()], tprop::proc_stmt)
        .into_iter()
        .filter(|v| v.not_temp())
        .map(|a_var2| FuncsXferTaintOut { a_var2: a_var2 })
        .collect()
}
