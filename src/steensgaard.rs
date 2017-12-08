use bap::high::bitvector::BitVector;
use bap::high::bil::Statement;
use bap::high::bil;

use std::collections::{BTreeSet, HashMap};

#[derive(Clone)]
pub enum Var {
    StackSlot { func_addr: BitVector, offset: usize },
    Register { site: BitVector, register: String },
}

// Maps a register at a code address to the list of possible definition sites (for a specific
// address)
type DefChain = HashMap<String, BTreeSet<BitVector>>;

fn move_walk<
    A,
    F: Fn(&bil::Variable, &bil::Expression, &mut DefChain, &BitVector, &BitVector) -> Vec<A>,
>(
    stmt: &Statement,
    defs: &mut DefChain,
    cur_addr: &BitVector,
    func_addr: &BitVector,
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
                defs.entry(k).or_insert_with(BTreeSet::new).union(&v);
            }

            let mut out = then_out;
            out.extend(else_out);
            out
        }
        Statement::Move { ref lhs, ref rhs } => f(lhs, rhs, defs, cur_addr, func_addr),
    }
}

enum E {
    AddrOf(Var),
    Base(Var),
    Deref(Var),
}

fn extract_expr(
    e: &bil::Expression,
    defs: &mut DefChain,
    cur_addr: &BitVector,
    func_addr: &BitVector,
) -> Vec<E> {
    use bap::high::bil::Expression::*;
    use num_traits::ToPrimitive;
    match *e {
        // TODO: Forward stack frame information in here so we can detect stack slots off %rbp
        Var(ref bv) => {
            if bv.name == "RSP" {
                vec![
                    E::AddrOf(self::Var::StackSlot {
                        func_addr: func_addr.clone(),
                        offset: 0,
                    }),
                ]
            } else {
                match defs.get(&bv.name) {
                    None => Vec::new(),
                    Some(sites) => sites
                        .iter()
                        .map(|site| {
                            E::Base(self::Var::Register {
                                site: site.clone(),
                                register: bv.name.clone(),
                            })
                        })
                        .collect(),
                }
            }
        }
        // Disabled for speed, enable for global tracking
        Const(_) => Vec::new(),
        Load { ref index, .. } => extract_expr(index, defs, cur_addr, func_addr)
            .into_iter()
            .map(|e| match e {
                E::Base(v) => E::Deref(v),
                E::AddrOf(v) => E::Base(v),
                _ => panic!("doubly nested load"),
            })
            .collect(),
        Store { .. } => panic!("Extracting on memory"),
        // Adjust here for field sensitivity
        BinOp {
            ref lhs,
            ref rhs,
            op,
        } => {
            let mut out = extract_expr(lhs, defs, cur_addr, func_addr);
            out.extend(extract_expr(lhs, defs, cur_addr, func_addr));
            if op == bil::BinOp::Add {
                if let Var(ref lv) = **lhs {
                    if lv.name == "RSP" {
                        match **rhs {
                            Const(ref bv) => vec![
                                E::AddrOf(self::Var::StackSlot {
                                    func_addr: func_addr.clone(),
                                    offset: bv.to_u64().unwrap() as usize,
                                }),
                            ],
                            _ => out,
                        }
                    } else {
                        out
                    }
                } else if let Var(ref rv) = **rhs {
                    if rv.name == "RSP" {
                        match **lhs {
                            Const(ref bv) => vec![
                                E::AddrOf(self::Var::StackSlot {
                                    func_addr: func_addr.clone(),
                                    offset: bv.to_u64().unwrap() as usize,
                                }),
                            ],
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
        IfThenElse {
            true_expr: ref lhs,
            false_expr: ref rhs,
            ..
        } => {
            let mut out = extract_expr(&*lhs, defs, cur_addr, func_addr);
            out.extend(extract_expr(&*rhs, defs, cur_addr, func_addr));
            out
        }
        Let { .. } => panic!("let unimpl"),
        Unknown { .. } | UnOp { .. } | Extract { .. } | Concat { .. } | Cast { .. } => Vec::new(),
    }
}

fn extract_move(
    lhs: &bil::Variable,
    rhs: &bil::Expression,
    defs: &mut DefChain,
    cur_addr: &BitVector,
    func_addr: &BitVector,
) -> Vec<Constraint> {
    match lhs.type_ {
        bil::Type::Memory { .. } => panic!("memlog"),
        bil::Type::Immediate(_) => {
            let lv = Var::Register {
                site: cur_addr.clone(),
                register: lhs.name.clone(),
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
                let mut our_addr = BTreeSet::new();
                our_addr.insert(cur_addr.clone());
                defs.insert(lhs.name.clone(), our_addr);
            }
            out
        }
    }
}

pub enum Constraint {
    // a = &b;
    AddrOf { a: Var, b: Var },
    // a = b
    Asgn { a: Var, b: Var },
    // a = *b
    Deref { a: Var, b: Var },
    // *a = b
    Write { a: Var, b: Var },
}

pub fn extract_constraints(
    sema: &[Statement],
    mut defs: DefChain,
    addr: &BitVector,
    func_addr: &BitVector,
) -> Vec<Constraint> {
    let mut constraints = Vec::new();
    for stmt in sema {
        constraints.extend(move_walk(stmt, &mut defs, addr, func_addr, &extract_move));
    }
    constraints
}
