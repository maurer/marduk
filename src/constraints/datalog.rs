use super::generation;
use super::Constraint;
use datalog::*;
use regs::{ARGS, RET_REG};
use var::Var;

pub fn gen_constraints(i: &ConstraintsGenConstraintsIn) -> Vec<ConstraintsGenConstraintsOut> {
    vec![ConstraintsGenConstraintsOut {
        c: generation::extract_constraints(i.bil, i.dc.clone(), i.loc, i.base),
    }]
}

pub fn malloc_constraint(i: &ConstraintsMallocConstraintIn) -> Vec<ConstraintsMallocConstraintOut> {
    vec![ConstraintsMallocConstraintOut {
        c: vec![vec![Constraint::AddrOf {
            a: Var::Register {
                site: *i.loc,
                register: RET_REG,
            },
            b: Var::Alloc {
                site: *i.loc,
                stale: false,
            },
        }]],
    }]
}

pub fn free_constraint(i: &ConstraintsFreeConstraintIn) -> Vec<ConstraintsFreeConstraintOut> {
    let mut out = Vec::new();
    for arg_n in i.args {
        if let Some(defs) = i.dc.get(&ARGS[*arg_n]) {
            for def in defs {
                out.push(ConstraintsFreeConstraintOut {
                    c: vec![vec![Constraint::StackLoad {
                        a: Var::Register {
                            site: *def,
                            register: ARGS[*arg_n],
                        },
                        b: Var::Freed { site: *i.loc },
                    }]],
                });
            }
        }
    }
    out
}
