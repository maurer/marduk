use crate::constraints::Constraint;
use crate::var::Var;
use bap::basic::Arch;
use bap::high::bil::Statement;
use std::collections::BTreeSet;

use crate::regs::{Reg, ARGS};
type Bytes = Vec<u8>;
type Sema = Vec<Statement>;
type StringSet = BTreeSet<String>;
type Regs = Vec<Reg>;
type Constraints = Vec<Constraint>;
type LocSet = Vec<Loc>;
type Vusize = Vec<usize>;
type Vars = Vec<Var>;
use crate::effect::Effect;
use crate::load::Loc;
use crate::points_to::PointsTo;
use crate::use_def::KillSpec;

use crate::constraints::datalog as constraints;
use crate::context;
use crate::live;

const VAR_ARG_0: Var = Var::Register { register: ARGS[0] };
const VAR_ARG_1: Var = Var::Register { register: ARGS[1] };

fn effect_merge(efs: &[&Effect]) -> Effect {
    let mut out = efs[0].clone();
    for eff in &efs[1..] {
        out = out.merge(eff);
    }
    out
}

fn loc_merge(lss: &[&LocSet]) -> LocSet {
    let mut out = Vec::new();
    out.reserve(lss.iter().map(|ls| ls.len()).sum());
    for ls in lss {
        for l in ls.iter() {
            if !out.contains(l) {
                out.push(l.clone())
            }
        }
    }
    out
}

fn concat<T: Clone>(xss: &[&Vec<T>]) -> Vec<T> {
    let mut out = Vec::new();
    out.reserve(xss.iter().map(|xs| xs.len()).sum());
    for xs in xss {
        out.extend(xs.iter().cloned());
    }
    out
}

fn pts_merge(ptss: &[&PointsTo]) -> PointsTo {
    let mut out: PointsTo = ptss[0].clone();
    for pts in &ptss[1..] {
        out.merge(pts);
    }
    out
}

fn union<T: Clone + Eq + Ord>(bts: &[&BTreeSet<T>]) -> BTreeSet<T> {
    let mut out = BTreeSet::new();
    for bt in bts {
        out.extend(bt.iter().cloned());
    }
    out
}

mycroft_files!(
    "mycroft/schema.my",
    "mycroft/load.my",
    "mycroft/live.my",
    "mycroft/defs.my",
    "mycroft/undef_entry.my",
    "mycroft/fmt_str.my",
    "mycroft/constraints.my",
    "mycroft/flow.my",
    "mycroft/uaf.my",
    "mycroft/fun_effect.my",
    "mycroft/context.my",
    "mycroft/queries.my"
);
pub use self::mycroft_program::*;
