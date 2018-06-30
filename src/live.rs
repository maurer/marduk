use bap::high::bil;
use datalog::*;
use load::Loc;
use points_to::{PointsTo, VarRef};
use regs::Reg;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use var::{var_args, Var};

use constraints::generation::move_walk;

pub fn not_defined(i: &LiveNotDefinedIn) -> Vec<LiveNotDefinedOut> {
    if i.vars.contains(i.var) {
        Vec::new()
    } else {
        vec![LiveNotDefinedOut {}]
    }
}

fn defined_walk(
    lhs: &bil::Variable,
    rhs: &bil::Expression,
    cur_addr: &Loc,
    func_addr: &Loc,
    tmp_db: &mut BTreeMap<Var, u64>,
) -> Vec<Var> {
    use constraints::generation::{extract_expr, E};
    let mut out = Vec::new();
    //TODO dedup
    match lhs.type_ {
        bil::Type::Memory { .. } => {
            let index = if let bil::Expression::Store { ref index, .. } = *rhs {
                index
            } else {
                panic!("Writing to memory, but the expression isn't a store");
            };
            for evar in extract_expr(index, cur_addr, func_addr, tmp_db) {
                if let E::VP(v) = evar {
                    if v.derefs() == 1 {
                        out.push(v.base)
                    }
                }
            }
        }
        bil::Type::Immediate(1) => (),
        bil::Type::Immediate(_) => {
            if lhs.tmp || lhs.name == "RSP" {
                return Vec::new();
            }
            if let Ok(reg) = Reg::from_str(lhs.name.as_str()) {
                out.push(Var::Register { register: reg })
            }
        }
    }
    out
}

fn used_walk(
    lhs: &bil::Variable,
    rhs: &bil::Expression,
    cur_addr: &Loc,
    func_addr: &Loc,
    tmp_db: &mut BTreeMap<Var, u64>,
) -> Vec<Var> {
    use constraints::generation::{extract_expr, E};
    let mut out = Vec::new();
    //TODO dedup
    match lhs.type_ {
        bil::Type::Memory { .. } => {
            let (index, value) = if let bil::Expression::Store {
                ref index,
                ref value,
                ..
            } = *rhs
            {
                (index, value)
            } else {
                panic!("Writing to memory, but the expression isn't a store");
            };
            for evar in extract_expr(index, cur_addr, func_addr, tmp_db) {
                if let E::VP(v) = evar {
                    if !v.base.is_temp() && v.derefs() > 1 {
                        out.push(v.base)
                    }
                }
            }
            for evar in extract_expr(value, cur_addr, func_addr, tmp_db) {
                if let E::VP(v) = evar {
                    if !v.base.is_temp() && v.derefs() > 1 {
                        out.push(v.base)
                    }
                }
            }
        }
        bil::Type::Immediate(1) => (),
        bil::Type::Immediate(_) => {
            if lhs.name == "RSP" {
                return Vec::new();
            }
            for evar in extract_expr(rhs, cur_addr, func_addr, tmp_db) {
                if let E::VP(v) = evar {
                    if !v.base.is_temp() && v.derefs() > 1 {
                        out.push(v.base)
                    }
                }
            }
        }
    }
    out
}

pub fn defined(i: &LiveDefinedIn) -> Vec<LiveDefinedOut> {
    let mut defined_vars = Vec::new();
    let mut tmp_db = BTreeMap::new();
    for stmt in i.bil {
        defined_vars.extend(move_walk(stmt, i.loc, i.base, &defined_walk, &mut tmp_db));
    }
    vec![LiveDefinedOut { vars: defined_vars }]
}

pub fn used(i: &LiveUsedIn) -> Vec<LiveUsedOut> {
    let mut used_vars = Vec::new();
    let mut tmp_db = BTreeMap::new();
    for stmt in i.bil {
        used_vars.extend(move_walk(stmt, i.loc, i.base, &used_walk, &mut tmp_db));
    }
    used_vars
        .into_iter()
        .map(|used_var| LiveUsedOut { var: used_var })
        .collect()
}

pub fn promote_var(i: &LivePromoteVarIn) -> Vec<LivePromoteVarOut> {
    vec![LivePromoteVarOut {
        vars: vec![i.var.clone()],
    }]
}

pub fn call_defs(_i: &LiveCallDefsIn) -> Vec<LiveCallDefsOut> {
    vec![LiveCallDefsOut {
        vars: vec![Var::Register { register: Reg::RAX }],
    }]
}

pub fn drop_stack(i: &LiveDropStackIn) -> Vec<LiveDropStackOut> {
    if i.var.is_stack() {
        Vec::new()
    } else {
        vec![LiveDropStackOut {}]
    }
}

pub fn drop_frame(i: &LiveDropFrameIn) -> Vec<LiveDropFrameOut> {
    match *i.var {
        Var::StackSlot { ref func_addr, .. } => {
            if func_addr == i.func {
                Vec::new()
            } else {
                vec![LiveDropFrameOut {}]
            }
        }
        _ => vec![LiveDropFrameOut {}],
    }
}

pub fn promote_reg(i: &LivePromoteRegIn) -> Vec<LivePromoteRegOut> {
    vec![LivePromoteRegOut {
        var: Var::Register { register: *i.reg },
    }]
}

pub fn entry_defined_promote(i: &LiveEntryDefinedPromoteIn) -> Vec<LiveEntryDefinedPromoteOut> {
    vec![LiveEntryDefinedPromoteOut {
        vars: vec![Var::Register {
            register: *i.register,
        }],
    }]
}

fn construct(serial: &mut usize, loc: &Loc) -> Var {
    let out = Var::Constructed {
        serial: *serial,
        site: loc.clone(),
    };
    *serial += 1;
    out
}

fn build_struct(
    pts: &mut PointsTo,
    serial: &mut usize,
    var: Var,
    loc: &Loc,
    width: usize,
    depth: usize,
) {
    const WORD_SIZE: usize = 8;
    let mut bases: Vec<Var> = vec![var];
    for _ in 0..depth {
        let mut new_bases: Vec<Var> = Vec::new();
        for base in bases {
            for w in 0..width {
                let target = construct(serial, loc);
                let mut target_set = BTreeSet::new();
                target_set.insert(VarRef {
                    var: target.clone(),
                    offset: Some(0),
                });
                pts.set_alias(
                    VarRef {
                        var: base.clone(),
                        offset: Some((w * WORD_SIZE) as u64),
                    },
                    target_set,
                );
                new_bases.push(target);
            }
        }
        bases = new_bases
    }
}

pub fn undef_live(i: &LiveUndefLiveIn) -> Vec<LiveUndefLiveOut> {
    let mut undefs = Vec::new();
    trace!("undef_live candidate: {}", i.loc);
    let args = var_args();
    for var in i.live {
        if !i.defined.contains(var) && args.contains(var) {
            undefs.push(var.clone());
        }
    }
    if undefs.is_empty() {
        trace!("All values defined, skipping");
        return Vec::new();
    }
    trace!("Some values undefined:");
    for var in &undefs {
        trace!("{}", var);
    }
    let mut pts = PointsTo::new(i.loc.clone());

    let mut serial = 0;
    for var in undefs {
        build_struct(&mut pts, &mut serial, var, i.loc, 4, 2);
    }
    trace!("Generated self-referential region and assigned.");
    trace!("undef_out: {}", pts);
    vec![LiveUndefLiveOut { pts }]
}
