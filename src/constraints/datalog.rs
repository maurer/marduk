use super::generation;
use super::{Constraint, VarPath};
use datalog::*;
use regs::{ARGS, RET_REG};
use var::Var;

pub fn gen_constraints(i: &ConstraintsGenConstraintsIn) -> Vec<ConstraintsGenConstraintsOut> {
    vec![ConstraintsGenConstraintsOut {
        c: if i.is_call {
            Vec::new()
        } else {
            generation::extract_constraints(i.bil, i.loc, i.base)
        },
    }]
}

pub fn malloc_constraint(i: &ConstraintsMallocConstraintIn) -> Vec<ConstraintsMallocConstraintOut> {
    vec![ConstraintsMallocConstraintOut {
        c: vec![Constraint {
            lhs: VarPath::reg(&RET_REG),
            rhs: VarPath {
                base: Var::Alloc {
                    site: i.loc.clone(),
                    stale: false,
                },
                offsets: vec![Some(0)],
            }
        }],
    }]
}

pub fn free_constraint(i: &ConstraintsFreeConstraintIn) -> Vec<ConstraintsFreeConstraintOut> {
    vec![ConstraintsFreeConstraintOut {
        c: i.args
            .iter()
            .map(|arg_n| Constraint {
                lhs: VarPath::reg(&ARGS[*arg_n]).unknown().deref(),
                rhs: VarPath {
                    base: Var::Freed {
                        site: i.loc.clone(),
                    },
                    offsets: vec![Some(0)],
                }
            })
            .collect(),
    }]
}
