use super::{Constraint, VarPath};
use bap::high::bil;
use bap::high::bil::Statement;
use load::Loc;
use regs::Reg;
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use var::Var;

pub fn move_walk<
    A,
    F: Fn(&bil::Variable, &bil::Expression, &Loc, &Loc, &mut BTreeMap<Var, u64>) -> Vec<A>,
>(
    stmt: &Statement,
    cur_addr: &Loc,
    func_addr: &Loc,
    f: &F,
    tmp_db: &mut BTreeMap<Var, u64>,
) -> Vec<A> {
    match *stmt {
        Statement::Jump(_) | Statement::Special | Statement::CPUException(_) => Vec::new(),
        Statement::While { ref body, .. } => {
            // We pass over the body twice to get the flow sensitivity on variables right
            let mut out: Vec<A> = body.iter()
                .flat_map(|stmt| move_walk(stmt, cur_addr, func_addr, f, tmp_db))
                .collect();
            out.extend(
                body.iter()
                    .flat_map(|stmt| move_walk(stmt, cur_addr, func_addr, f, tmp_db))
                    .collect::<Vec<_>>(),
            );
            out
        }
        Statement::IfThenElse {
            ref then_clause,
            ref else_clause,
            ..
        } => {
            let then_out: Vec<_> = then_clause
                .iter()
                .flat_map(|stmt| move_walk(stmt, cur_addr, func_addr, f, tmp_db))
                .collect();
            let else_out: Vec<_> = else_clause
                .iter()
                .flat_map(|stmt| move_walk(stmt, cur_addr, func_addr, f, tmp_db))
                .collect();

            let mut out = then_out;
            out.extend(else_out);
            out
        }
        Statement::Move { ref lhs, ref rhs } => f(lhs, rhs, cur_addr, func_addr, tmp_db),
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum E {
    VP(VarPath),
    Const(u64),
}

pub fn extract_expr(
    e: &bil::Expression,
    cur_addr: &Loc,
    func_addr: &Loc,
    tmp_db: &BTreeMap<Var, u64>,
) -> Vec<E> {
    use bap::high::bil::Expression as BE;
    use num_traits::ToPrimitive;
    match *e {
        // TODO: Forward stack frame information in here so we can detect stack slots off %rbp
        BE::Var(ref bv) => {
            if bv.type_ == bil::Type::Immediate(1) {
                return Vec::new();
            }
            let vp = if bv.name == "RSP" {
                VarPath::stack_addr(func_addr, 0)
            } else {
                match Reg::from_str(bv.name.as_str()) {
                    Ok(reg) => VarPath::reg(reg),
                    Err(_) => if bv.tmp {
                        VarPath::temp(bv.name.as_str())
                    } else {
                        error!("Unrecognized variable name: {:?}", bv.name);
                        return Vec::new();
                    },
                }
            };
            if let Some(k) = tmp_db.get(&vp.base) {
                vec![E::Const(*k)]
            } else {
                vec![E::VP(vp)]
            }
        }
        BE::Const(ref e) => vec![E::Const(e.to_u64().unwrap())],
        BE::Load { ref index, .. } => extract_expr(index, cur_addr, func_addr, tmp_db)
            .into_iter()
            .flat_map(|e| match e {
                E::VP(v) => vec![E::VP(v.deref())],
                E::Const(_) => {
                    trace!("constant dereference");
                    Vec::new()
                }
            })
            .collect(),
        BE::Store { .. } => panic!("Extracting on memory"),
        // Adjust here for field sensitivity
        BE::BinOp {
            ref lhs,
            ref rhs,
            op,
        } => {
            if op == bil::BinOp::Add {
                // Check for stack-relative addressing
                if let BE::Var(ref lv) = **lhs {
                    if lv.name == "RSP" {
                        if let BE::Const(ref bv) = **rhs {
                            return vec![E::VP(VarPath::stack_addr(
                                func_addr,
                                bv.to_usize().unwrap(),
                            ))];
                        }
                    }
                } else if let BE::Var(ref rv) = **rhs {
                    if rv.name == "RSP" {
                        if let BE::Const(ref bv) = **lhs {
                            return vec![E::VP(VarPath::stack_addr(
                                func_addr,
                                bv.to_usize().unwrap(),
                            ))];
                        }
                    }
                }
                // Since we don't have stack relative addressing, it's time to do field math
                let lhe = extract_expr(lhs, cur_addr, func_addr, tmp_db);
                let rhe = extract_expr(rhs, cur_addr, func_addr, tmp_db);
                let mut out = Vec::new();
                for e0 in &lhe {
                    for e1 in &rhe {
                        match (e0, e1) {
                            (&E::Const(ref k), &E::VP(ref v))
                            | (&E::VP(ref v), &E::Const(ref k)) => out.push(E::VP(v.plus(*k))),
                            // TODO this is a little iffy, doesn't do bitwidth right
                            // Unlikely to be a problem with pointers though
                            (&E::Const(ref k), &E::Const(ref k2)) => out.push(E::Const(*k + *k2)),
                            (&E::VP(ref v), &E::VP(ref v2)) => {
                                out.push(E::VP(v.unknown()));
                                out.push(E::VP(v2.unknown()));
                            }
                        }
                    }
                }
                out
            } else {
                // Some kind of unknown computation is happening on the pointers. It might be
                // subtraction, it might be some weird oring/anding, in any case, we no longer know
                // the offset.
                // Just enumerate everything on the left, everything on the right, set their offset
                // to None for "who knows", and return. This is equivalent to the old field
                // insensitive code
                let mut used: Vec<E> = extract_expr(lhs, cur_addr, func_addr, tmp_db);
                used.extend(extract_expr(rhs, cur_addr, func_addr, tmp_db));
                let mut out = BTreeSet::new();
                for e in used {
                    match e {
                        E::VP(v) => {
                            out.insert(E::VP(v.unknown()));
                            ()
                        }
                        E::Const(_) => (),
                    }
                }
                out.into_iter().collect()
            }
        }
        BE::IfThenElse {
            true_expr: ref lhs,
            false_expr: ref rhs,
            ..
        } => {
            let mut out = extract_expr(&*lhs, cur_addr, func_addr, tmp_db);
            out.extend(extract_expr(&*rhs, cur_addr, func_addr, tmp_db));
            out
        }
        BE::Let { .. } => panic!("let unimpl"),
        BE::Cast { ref arg, .. } => extract_expr(arg, cur_addr, func_addr, tmp_db),
        BE::Unknown { .. } | BE::UnOp { .. } | BE::Extract { .. } | BE::Concat { .. } => Vec::new(),
    }
}

fn extract_move_var(
    lhs: &bil::Variable,
    rhs: &bil::Expression,
    cur_addr: &Loc,
    func_addr: &Loc,
    tmp_db: &mut BTreeMap<Var, u64>,
) -> Vec<Var> {
    match lhs.type_ {
        bil::Type::Memory { .. } => {
            use self::E::*;
            let (index, rhs) = if let bil::Expression::Store {
                ref index,
                ref value,
                ..
            } = *rhs
            {
                (index, value)
            } else {
                panic!("Writing to memory, but the expression isn't a store")
            };
            let lhs_vars = extract_expr(index, cur_addr, func_addr, &tmp_db);
            let rhs_vars = extract_expr(rhs, cur_addr, func_addr, &tmp_db);
            let mut out = Vec::new();
            for lhs_evar in lhs_vars {
                if let VP(l) = lhs_evar {
                    out.push(l.base)
                }
            }
            for rhs_evar in rhs_vars {
                if let VP(v) = rhs_evar {
                    if v.derefs() > 2 {
                        out.push(v.base)
                    }
                }
            }
            out
        }
        bil::Type::Immediate(_) => {
            let mut out = Vec::new();
            for eval in extract_expr(rhs, cur_addr, func_addr, &tmp_db) {
                match eval {
                    E::VP(v) => if v.derefs() > 2 {
                        out.push(v.base)
                    },
                    E::Const(_) => (),
                }
            }
            out
        }
    }
}

fn extract_move(
    lhs: &bil::Variable,
    rhs: &bil::Expression,
    cur_addr: &Loc,
    func_addr: &Loc,
    tmp_db: &mut BTreeMap<Var, u64>,
) -> Vec<Constraint> {
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
                panic!("Writing to memory, but the expression isn't a store")
            };
            let lhs_vars = extract_expr(index, cur_addr, func_addr, &tmp_db);
            let rhs_vars = extract_expr(value, cur_addr, func_addr, &tmp_db);
            let mut out = Vec::new();
            for lhs_evar in lhs_vars {
                let lhs_expr = match &lhs_evar {
                    E::Const(_) => {
                        warn!("Ignoring write to constant address");
                        continue;
                    }
                    E::VP(ref lhs) => {
                        if lhs.derefs() > 1 {
                            warn!("attempting to do a nested store");
                        }
                        lhs.deref()
                    }
                };
                let mut out_exprs = Vec::new();
                for rhs_evar in rhs_vars.clone() {
                    match rhs_evar {
                        // We're not dealing with clobbers at the moment
                        E::Const(_) => continue,
                        E::VP(rhs) => {
                            if rhs.derefs() > 2 {
                                warn!("Attempting to do nested load");
                            }
                            out_exprs.push(rhs);
                        }
                    }
                }
                if !out_exprs.is_empty() {
                    out.push(Constraint {
                        lhs: lhs_expr,
                        rhss: out_exprs,
                    })
                }
            }
            out
        }
        // Ignore flags
        bil::Type::Immediate(1) => Vec::new(),
        bil::Type::Immediate(_) => {
            let lv = if lhs.tmp {
                Var::temp(lhs.name.as_str())
            } else if lhs.name == "RSP" {
                // Suppress generation of RSP constraints - we're handling stack discipline
                // separately
                return Vec::new();
            } else if let Ok(reg) = Reg::from_str(lhs.name.as_str()) {
                Var::Register { register: reg }
            } else {
                warn!("Unrecognized variable name: {:?}", lhs.name);
                return Vec::new();
            };
            let mut ks = Vec::new();
            let rhs_exprs: Vec<_> = extract_expr(rhs, cur_addr, func_addr, &tmp_db)
                .into_iter()
                .filter_map(|eval| match eval {
                    E::VP(vp) => Some(vp),
                    E::Const(k) => {
                        ks.push(k);
                        None
                    }
                })
                .collect();
            // World's dumbest constant folding
            // If this resolved to a constant, only one constant, and
            // there are no pointer expressions, then put it in the temp db.
            // Otherwise, purge it
            if ks.len() == 1 && rhs_exprs.is_empty() {
                tmp_db.insert(lv.clone(), ks[0]);
            } else {
                tmp_db.remove(&lv);
            }
            if rhs_exprs.is_empty() {
                Vec::new()
            } else {
                vec![Constraint {
                    lhs: VarPath::var(lv),
                    rhss: rhs_exprs,
                }]
            }
        }
    }
}

pub fn extract_constraints(sema: &[Statement], cur: &Loc, func_loc: &Loc) -> Vec<Constraint> {
    let mut constraints = Vec::new();
    let mut tmp_db: BTreeMap<Var, u64> = BTreeMap::new();
    for stmt in sema {
        constraints.extend(move_walk(stmt, cur, func_loc, &extract_move, &mut tmp_db));
    }
    constraints
}

pub fn extract_var_use(sema: &[Statement], cur: &Loc, func_loc: &Loc) -> Vec<Var> {
    let mut vars = Vec::new();
    let mut tmp_db = BTreeMap::new();
    for stmt in sema {
        vars.extend(move_walk(
            stmt,
            cur,
            func_loc,
            &extract_move_var,
            &mut tmp_db,
        ));
    }
    vars
}
