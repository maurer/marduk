use super::Constraint;
use bap::high::bil;
use bap::high::bil::Statement;
use load::Loc;
use regs::Reg;
use std::str::FromStr;
use var::Var;

pub fn move_walk<A, F: Fn(&bil::Variable, &bil::Expression, &Loc, &Loc) -> Vec<A>>(
    stmt: &Statement,
    cur_addr: &Loc,
    func_addr: &Loc,
    f: &F,
) -> Vec<A> {
    match *stmt {
        Statement::Jump(_) | Statement::Special | Statement::CPUException(_) => Vec::new(),
        Statement::While { ref body, .. } => {
            // We pass over the body twice to get the flow sensitivity on variables right
            let mut out: Vec<A> = body
                .iter()
                .flat_map(|stmt| move_walk(stmt, cur_addr, func_addr, f))
                .collect();
            out.extend(
                body.iter()
                    .flat_map(|stmt| move_walk(stmt, cur_addr, func_addr, f))
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
                .flat_map(|stmt| move_walk(stmt, cur_addr, func_addr, f))
                .collect();
            let else_out: Vec<_> = else_clause
                .iter()
                .flat_map(|stmt| move_walk(stmt, cur_addr, func_addr, f))
                .collect();

            let mut out = then_out;
            out.extend(else_out);
            out
        }
        Statement::Move { ref lhs, ref rhs } => f(lhs, rhs, cur_addr, func_addr),
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub enum E {
    AddrOf(Var),
    Base(Var),
    Deref(Var),
}

pub fn extract_expr(e: &bil::Expression, cur_addr: &Loc, func_addr: &Loc) -> Vec<E> {
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
                    Ok(reg) => vec![E::Base(Var::Register { register: reg })],
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
        BE::Load { ref index, .. } => extract_expr(index, cur_addr, func_addr)
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
            let mut out = extract_expr(lhs, cur_addr, func_addr);
            out.extend(extract_expr(rhs, cur_addr, func_addr));
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
            let mut out = extract_expr(&*lhs, cur_addr, func_addr);
            out.extend(extract_expr(&*rhs, cur_addr, func_addr));
            out
        }
        BE::Let { .. } => panic!("let unimpl"),
        BE::Cast { ref arg, .. } => extract_expr(arg, cur_addr, func_addr),
        BE::Unknown { .. } | BE::UnOp { .. } | BE::Extract { .. } | BE::Concat { .. } => Vec::new(),
    }
}

fn extract_move_var(
    lhs: &bil::Variable,
    rhs: &bil::Expression,
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
            let lhs_vars = extract_expr(index, cur_addr, func_addr);
            let rhs_vars = extract_expr(rhs, cur_addr, func_addr);
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
            for eval in extract_expr(rhs, cur_addr, func_addr) {
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
            let lhs_vars = extract_expr(index, cur_addr, func_addr);
            let rhs_vars = extract_expr(value, cur_addr, func_addr);
            let mut out = Vec::new();
            for lhs_evar in lhs_vars {
                for rhs_evar in rhs_vars.clone() {
                    out.extend(match (lhs_evar.clone(), rhs_evar) {
                        (AddrOf(l), AddrOf(r)) => vec![Constraint::AddrOf { a: l, b: r }],
                        (AddrOf(l), Base(r)) => vec![Constraint::Asgn { a: l, b: r }],
                        (AddrOf(l), Deref(r)) => vec![Constraint::Deref { a: l, b: r }],
                        (Deref(_), _) => panic!("**a = x ?"),
                        (Base(l), AddrOf(r)) => vec![Constraint::StackLoad { a: l, b: r }],
                        (Base(l), Base(r)) => vec![Constraint::Write { a: l, b: r }],
                        (Base(l), Deref(r)) => vec![Constraint::Xfer { a: l, b: r }],
                    })
                }
                if rhs_vars.is_empty() {
                    match lhs_evar.clone() {
                        AddrOf(l) => out.push(Constraint::Clobber { v: l }),
                        _ => (),
                    }
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
            let out: Vec<_> = extract_expr(rhs, cur_addr, func_addr)
                .into_iter()
                .flat_map(|eval| match eval {
                    E::AddrOf(var) => vec![Constraint::AddrOf {
                        a: lv.clone(),
                        b: var,
                    }],
                    E::Base(var) => vec![Constraint::Asgn {
                        a: lv.clone(),
                        b: var,
                    }],
                    E::Deref(var) => vec![Constraint::Deref {
                        a: lv.clone(),
                        b: var,
                    }],
                })
                .collect();
            if out.is_empty() {
                vec![Constraint::Clobber { v: lv }]
            } else {
                out
            }
        }
    }
}

pub fn extract_constraints(sema: &[Statement], cur: &Loc, func_loc: &Loc) -> Vec<Constraint> {
    let mut constraints = Vec::new();
    for stmt in sema {
        constraints.extend(move_walk(stmt, cur, func_loc, &extract_move));
    }
    constraints
}

pub fn extract_var_use(sema: &[Statement], cur: &Loc, func_loc: &Loc) -> Vec<Var> {
    let mut vars = Vec::new();
    for stmt in sema {
        vars.extend(move_walk(stmt, cur, func_loc, &extract_move_var));
    }
    vars
}
