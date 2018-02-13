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

fn loc_merge(l1: &LocSet, l2: &LocSet) -> LocSet {
    let mut out = Vec::new();
    out.reserve(l1.len() + l2.len());
    for l in l1.iter().chain(l2.iter()) {
        if !out.contains(l) {
            out.push(l.clone())
        }
    }
    out
}

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Hash)]
pub struct Loc {
    pub file_name: String,
    pub addr: BitVector,
}

fn pts_merge(pts: &PointsTo, pts2: &PointsTo) -> PointsTo {
    let mut out = pts.clone();
    for (k, v) in pts2.iter() {
        match out.entry(k.clone()) {
            btree_map::Entry::Occupied(mut o) => {
                o.get_mut().append(&mut v.clone());
            }
            btree_map::Entry::Vacant(e) => {
                e.insert(v.clone());
            }
        };
    }
    out
}

//TODO chain_merge is buggy, it will drop duplicate entries rather than merging them
fn chain_merge(dc: &DefChain, dc2: &DefChain) -> DefChain {
    dc.iter()
        .chain(dc2.iter())
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

fn union<T: Clone + Eq + Ord>(x: &BTreeSet<T>, y: &BTreeSet<T>) -> BTreeSet<T> {
    x.union(y).cloned().collect()
}

fn concat<T: Clone>(x: &Vec<T>, y: &Vec<T>) -> Vec<T> {
    x.iter().chain(y.iter()).cloned().collect()
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
