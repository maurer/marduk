use datalog::*;
use bap::high::bil::{Expression, Statement};
use bap::high::bitvector::BitVector;
use bap::basic::Cast;
use num_traits::ToPrimitive;
use regs::ARGS;

fn const_collapse(e: &Expression) -> Option<BitVector> {
    match *e {
        Expression::Const(ref bv) => Some(bv.clone()),
        Expression::Cast {
            width,
            ref kind,
            ref arg,
        } => {
            if let Some(i) = const_collapse(arg) {
                match *kind {
                    Cast::Low | Cast::Unsigned => {
                        Some(BitVector::from_u64(i.to_u64().unwrap(), width as usize))
                    }
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn const_move(i: &FmtStrConstMoveIn) -> Vec<FmtStrConstMoveOut> {
    for stmt in i.bil {
        if let Statement::Move { ref lhs, ref rhs } = *stmt {
            if !(lhs.name == "RDI") {
                continue;
            }
            if let Some(bv) = const_collapse(rhs) {
                return vec![
                    FmtStrConstMoveOut {
                        addr: bv.to_u64().unwrap(),
                    },
                ];
            }
        }
    }
    Vec::new()
}

pub fn parse_str(i: &FmtStrParseStrIn) -> Vec<FmtStrParseStrOut> {
    let mut skip = false;
    let mut arg = 0;
    let mut out = Vec::new();
    let cs: Vec<_> = i.str.chars().collect();
    for c in (&cs).windows(2) {
        if skip {
            skip = false;
            continue;
        }
        if c[0] == '%' {
            if c[1] == '%' {
                skip = true;
            } else {
                out.push(FmtStrParseStrOut { arg: ARGS[arg] });
                arg += 1;
            }
        }
    }
    out
}

fn ascii_range(c: u8) -> bool {
    if c >= 0x20 && c <= 0x7E {
        return true;
    }
    if c >= 0x07 && c <= 0x0D {
        return true;
    }
    return false;
}

pub fn ascii_nullterm(i: &FmtStrAsciiNulltermIn) -> Vec<FmtStrAsciiNulltermOut> {
    if i.start <= i.addr && i.end > i.addr {
        let addr64 = i.addr.to_u64().unwrap();
        let start64 = i.start.to_u64().unwrap();
        let mut offset = (addr64 - start64) as usize;
        let mut strbuf = Vec::new();
        while offset <= i.contents.len() && i.contents[offset] != 0 {
            let candidate = i.contents[offset];
            offset += 1;
            if ascii_range(candidate) {
                strbuf.push(candidate);
            } else {
                return Vec::new();
            }
        }
        let out = String::from_utf8(strbuf).unwrap();
        return vec![FmtStrAsciiNulltermOut { str: out }];
    }
    Vec::new()
}
