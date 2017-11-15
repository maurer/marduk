use datalog::*;
use bap::high::bitvector::BitVector;
use bap::basic::{Image, Bap, BasicDisasm};
use bap::high::bil::{Statement, Expression}; //, Variable, Type, BinOp};

macro_rules! vec_error {
    ($e:expr) => {{
        let name: ::bap::basic::Result<_> = $e;
        match name {
            Ok(i) => vec![i],
            Err(e) => panic!("{}", e) //return Vec::new()
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
        vec![FuncsGetArchOut { arch: image.arch().unwrap() }]
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
