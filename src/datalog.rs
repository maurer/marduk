use mycroft_macros::mycroft_files;
use bap::high::bitvector::BitVector;
use bap::high::bil::Statement;
use bap::basic::Arch;
use steensgaard::{Constraint, DefChain, Var};
use std::collections::{BTreeMap, BTreeSet};
use std::collections::btree_map;
type Bytes = Vec<u8>;
type Sema = Vec<Statement>;
type StringSet = BTreeSet<String>;
type Strings = Vec<String>;
type Constraints = Vec<Constraint>;
type Vars = Vec<Var>;
type LocSet = Vec<Loc>;
pub type PointsTo = BTreeMap<Var, BTreeSet<Var>>;

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

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Hash)]
pub struct Loc {
    pub file_name: String,
    pub addr: BitVector,
}

fn pts_merge(ptss: &[&PointsTo]) -> PointsTo {
    let mut out = ptss[0].clone();
    for pts in &ptss[1..] {
        for (k, v) in pts.iter() {
            match out.entry(k.clone()) {
                btree_map::Entry::Occupied(mut o) => {
                    o.get_mut().append(&mut v.clone());
                }
                btree_map::Entry::Vacant(e) => {
                    e.insert(v.clone());
                }
            };
        }
    }
    out
}

fn chain_merge(dcs: &[&DefChain]) -> DefChain {
    let mut out = dcs[0].clone();
    for dc in &dcs[1..] {
        for (k, v) in dc.iter() {
            match out.entry(k.clone()) {
                btree_map::Entry::Occupied(mut o) => {
                    o.get_mut().append(&mut v.clone());
                }
                btree_map::Entry::Vacant(e) => {
                    e.insert(v.clone());
                }
            };
        }
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

fn concat<T: Clone>(xss: &[&Vec<T>]) -> Vec<T> {
    let mut out = Vec::new();
    out.reserve(xss.iter().map(|xs| xs.len()).sum());
    for xs in xss {
        out.extend(xs.iter().cloned());
    }
    out
}

mycroft_files!(
    "mycroft/schema.my",
    "mycroft/load.my",
    "mycroft/defs.my",
    "mycroft/steensgaard.my",
    "mycroft/flow.my",
    "mycroft/uaf.my",
    "mycroft/queries.my"
);
pub use self::mycroft_program::*;
