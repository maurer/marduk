use bap::basic::Arch;
use bap::high::bil::Statement;
use constraints::Constraint;
use std::collections::BTreeSet;
use var::Var;

use regs::{Reg, ARGS};
type Bytes = Vec<u8>;
type Sema = Vec<Statement>;
type StringSet = BTreeSet<String>;
type Regs = Vec<Reg>;
type Constraints = Vec<Constraint>;
type LocSet = Vec<Loc>;
type Vusize = Vec<usize>;
use effect::Effect;
use load::Loc;
use points_to::PointsTo;
use use_def::KillSpec;

use constraints::datalog as constraints;
use context;

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
    "mycroft/defs.my",
    "mycroft/fmt_str.my",
    "mycroft/constraints.my",
    "mycroft/flow.my",
    "mycroft/uaf.my",
    "mycroft/fun_effect.my",
    "mycroft/context.my",
    "mycroft/queries.my"
);
pub use self::mycroft_program::*;
