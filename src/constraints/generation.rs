use super::Constraint;
use bap::high::bil;
use bap::high::bil::Statement;
use load::Loc;
use regs::Reg;
use std::str::FromStr;
use use_def::DefChain;
use var::Var;

fn move_walk<A, F: Fn(&bil::Variable, &bil::Expression, &mut DefChain, &Loc, &Loc) -> Vec<A>>(
    stmt: &Statement,
    defs: &mut DefChain,
    cur_addr: &Loc,
    func_addr: &Loc,
    f: &F,
) -> Vec<A> {
    match *stmt {
        Statement::Jump(_) | Statement::Special | Statement::CPUException(_) => Vec::new(),
        Statement::While { ref body, .. } => {
            // We pass over the body twice to get the flow sensitivity on variables right
            let mut out: Vec<A> = body.iter()
                .flat_map(|stmt| move_walk(stmt, defs, cur_addr, func_addr, f))
                .collect();
            out.extend(
                body.iter()
                    .flat_map(|stmt| move_walk(stmt, defs, cur_addr, func_addr, f))
                    .collect::<Vec<_>>(),
            );
            out
        }
        Statement::IfThenElse {
            ref then_clause,
            ref else_clause,
            ..
        } => {
            // Process left, then right, then merge defs
            let mut else_defs = defs.clone();
            let then_out: Vec<_> = then_clause
                .iter()
                .flat_map(|stmt| move_walk(stmt, defs, cur_addr, func_addr, f))
                .collect();
            let else_out: Vec<_> = else_clause
                .iter()
                .flat_map(|stmt| move_walk(stmt, &mut else_defs, cur_addr, func_addr, f))
                .collect();

            // Merge back else defs info
            for (k, v) in else_defs {
                let e = defs.entry(k).or_insert_with(Vec::new);
                for ve in v {
                    if !e.contains(&ve) {
                        e.push(ve)
                    }
                }
            }

            let mut out = then_out;
            out.extend(else_out);
            out
        }
        Statement::Move { ref lhs, ref rhs } => f(lhs, rhs, defs, cur_addr, func_addr),
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum E {
    AddrOf(Var),
    Base(Var),
    Deref(Var),
}

fn extract_expr(
    e: &bil::Expression,
    defs: &mut DefChain,
    cur_addr: &Loc,
    func_addr: &Loc,
) -> Vec<E> {
    use bap::high::bil::Expression as BE;
    use num_traits::ToPrimitive;
    match *e {
        // TODO: Forward stack frame information in here so we can detect stack slots off %rbp
        BE::Var(ref bv) => {
            if bv.type_ == bil::Type::Immediate(1) {
                Vec::new()
            } else if bv.name == "RSP" {
                vec![E::AddrOf(Var::StackSlot {
                    func_addr: func_addr.clone(),
                    offset: 0,
                })]
            } else {
                match Reg::from_str(bv.name.as_str()) {
                    Ok(reg) => match defs.get(&reg) {
                        None => Vec::new(),
                        Some(sites) => sites
                            .iter()
                            .map(|site| {
                                E::Base(Var::Register {
                                    site: site.clone(),
                                    register: reg,
                                })
                            })
                            .collect(),
                    },
                    Err(_) => if bv.tmp {
                        vec![E::Base(Var::temp(bv.name.as_str()))]
                    } else {
                        error!("Unrecognized variable name: {:?}", bv.name);
                        Vec::new()
                    },
                }
            }
        }
        // Disabled for speed, enable for global tracking
        BE::Const(_) => Vec::new(),
        BE::Load { ref index, .. } => extract_expr(index, defs, cur_addr, func_addr)
            .into_iter()
            .map(|e| match e {
                E::Base(v) => E::Deref(v),
                E::AddrOf(v) => E::Base(v),
                _ => panic!("doubly nested load"),
            })
            .collect(),
        BE::Store { .. } => panic!("Extracting on memory"),
        // Adjust here for field sensitivity
        BE::BinOp {
            ref lhs,
            ref rhs,
            op,
        } => {
            let mut out = extract_expr(lhs, defs, cur_addr, func_addr);
            out.extend(extract_expr(rhs, defs, cur_addr, func_addr));
            if op == bil::BinOp::Add {
                if let BE::Var(ref lv) = **lhs {
                    if lv.name == "RSP" {
                        match **rhs {
                            BE::Const(ref bv) => vec![E::AddrOf(Var::StackSlot {
                                func_addr: func_addr.clone(),
                                offset: bv.to_u64().unwrap() as usize,
                            })],
                            _ => out,
                        }
                    } else {
                        out
                    }
                } else if let BE::Var(ref rv) = **rhs {
                    if rv.name == "RSP" {
                        match **lhs {
                            BE::Const(ref bv) => vec![E::AddrOf(Var::StackSlot {
                                func_addr: func_addr.clone(),
                                offset: bv.to_u64().unwrap() as usize,
                            })],
                            _ => out,
                        }
                    } else {
                        out
                    }
                } else {
                    out
                }
            } else {
                out
            }
        }
        BE::IfThenElse {
            true_expr: ref lhs,
            false_expr: ref rhs,
            ..
        } => {
            let mut out = extract_expr(&*lhs, defs, cur_addr, func_addr);
            out.extend(extract_expr(&*rhs, defs, cur_addr, func_addr));
            out
        }
        BE::Let { .. } => panic!("let unimpl"),
        BE::Cast { ref arg, .. } => extract_expr(arg, defs, cur_addr, func_addr),
        BE::Unknown { .. } | BE::UnOp { .. } | BE::Extract { .. } | BE::Concat { .. } => Vec::new(),
    }
}

fn extract_move_var(
    lhs: &bil::Variable,
    rhs: &bil::Expression,
    defs: &mut DefChain,
    cur_addr: &Loc,
    func_addr: &Loc,
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
            let lhs_vars = extract_expr(index, defs, cur_addr, func_addr);
            let rhs_vars = extract_expr(rhs, defs, cur_addr, func_addr);
            let mut out = Vec::new();
            for lhs_evar in lhs_vars {
                if let Base(l) = lhs_evar {
                    out.push(l)
                }
            }
            for rhs_evar in rhs_vars {
                if let Deref(v) = rhs_evar {
                    out.push(v)
                }
            }
            out
        }
        bil::Type::Immediate(_) => {
            let mut out = Vec::new();
            for eval in extract_expr(rhs, defs, cur_addr, func_addr) {
                if let E::Deref(v) = eval {
                    out.push(v)
                }
            }
            out
        }
    }
}

fn extract_move(
    lhs: &bil::Variable,
    rhs: &bil::Expression,
    defs: &mut DefChain,
    cur_addr: &Loc,
    func_addr: &Loc,
) -> Vec<Constraint> {
    match lhs.type_ {
        bil::Type::Memory { .. } => {
            use self::E::*;
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
            let lhs_vars = extract_expr(index, defs, cur_addr, func_addr);
            let rhs_vars = extract_expr(value, defs, cur_addr, func_addr);
            let mut out = Vec::new();
            for lhs_evar in lhs_vars {
                for rhs_evar in rhs_vars.clone() {
                    out.push(match (lhs_evar.clone(), rhs_evar) {
                        (AddrOf(l), AddrOf(r)) => Constraint::AddrOf { a: l, b: r },
                        (AddrOf(l), Base(r)) => Constraint::Asgn { a: l, b: r },
                        (AddrOf(l), Deref(r)) => Constraint::Deref { a: l, b: r },
                        (Deref(_), _) => panic!("**a = x ?"),
                        (Base(l), AddrOf(r)) => Constraint::StackLoad { a: l, b: r },
                        (Base(l), Base(r)) => Constraint::Write { a: l, b: r },
                        (Base(l), Deref(r)) => Constraint::Xfer { a: l, b: r },
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
                Var::Register {
                    site: cur_addr.clone(),
                    register: reg,
                }
            } else {
                warn!("Unrecognized variable name: {:?}", lhs.name);
                return Vec::new();
            };
            let out = extract_expr(rhs, defs, cur_addr, func_addr)
                .into_iter()
                .map(|eval| match eval {
                    E::AddrOf(var) => Constraint::AddrOf {
                        a: lv.clone(),
                        b: var,
                    },
                    E::Base(var) => Constraint::Asgn {
                        a: lv.clone(),
                        b: var,
                    },
                    E::Deref(var) => Constraint::Deref {
                        a: lv.clone(),
                        b: var,
                    },
                })
                .collect();
            if !lhs.tmp {
                // We've just overwritten a non-temporary, update the def chain
                let our_addr = vec![cur_addr.clone()];
                defs.insert(Reg::from_str(lhs.name.as_str()).unwrap(), our_addr);
            }
            out
        }
    }
}

pub fn extract_constraints(
    sema: &[Statement],
    mut defs: DefChain,
    cur: &Loc,
    func_loc: &Loc,
) -> Vec<Vec<Constraint>> {
    let mut constraints = Vec::new();
    for stmt in sema {
        let stmt_constrs = move_walk(stmt, &mut defs, cur, func_loc, &extract_move);
        if !stmt_constrs.is_empty() {
            constraints.push(stmt_constrs)
        }
    }
    constraints
}

pub fn extract_var_use(
    sema: &[Statement],
    mut defs: DefChain,
    cur: &Loc,
    func_loc: &Loc,
) -> Vec<Var> {
    let mut vars = Vec::new();
    for stmt in sema {
        vars.extend(move_walk(stmt, &mut defs, cur, func_loc, &extract_move_var));
    }
    vars
}
