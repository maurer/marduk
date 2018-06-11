use super::generation;
use super::Constraint;
use datalog::*;
use regs::{ARGS, CALLER_SAVED, RET_REG};
use var::Var;

pub fn gen_constraints(i: &ConstraintsGenConstraintsIn) -> Vec<ConstraintsGenConstraintsOut> {
    vec![ConstraintsGenConstraintsOut {
        c: if i.is_call {
            CALLER_SAVED
                .iter()
                .map(|reg| Constraint::Clobber {
                    v: Var::Register { register: *reg },
                })
                .collect()
        } else {
            generation::extract_constraints(i.bil, i.loc, i.base)
        },
    }]
}

pub fn malloc_constraint(i: &ConstraintsMallocConstraintIn) -> Vec<ConstraintsMallocConstraintOut> {
    vec![ConstraintsMallocConstraintOut {
        c: vec![Constraint::AddrOf {
            a: Var::Register { register: RET_REG },
            b: Var::Alloc {
                site: i.loc.clone(),
                stale: false,
            },
        }],
    }]
}

pub fn free_constraint(i: &ConstraintsFreeConstraintIn) -> Vec<ConstraintsFreeConstraintOut> {
    vec![ConstraintsFreeConstraintOut {
        c: i.args
            .iter()
            .map(|arg_n| Constraint::StackLoad {
                a: Var::Register {
                    register: ARGS[*arg_n],
                },
                b: Var::Freed {
                    site: i.loc.clone(),
                },
            })
            .collect(),
    }]
}
