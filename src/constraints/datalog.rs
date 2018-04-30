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
    i.args
        .iter()
        .cloned()
        .flat_map(|arg_n| {
            i.dc[&ARGS[arg_n]]
                .iter()
                .map(move |src| ConstraintsFreeConstraintOut {
                    c: vec![vec![Constraint::StackLoad {
                        a: Var::Register {
                            site: *src,
                            register: ARGS[arg_n],
                        },
                        b: Var::Freed { site: *i.loc },
                    }]],
                })
        })
        .collect()
}
