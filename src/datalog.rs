use bap::high::bil::Statement;
use bap::basic::Arch;
use steensgaard::{Constraint, DefChain, Var};
use std::collections::{BTreeMap, BTreeSet};
use std::collections::btree_map;
use std::sync::Mutex;
use std::collections::HashMap;

use regs::{Reg, ARGS, CALLER_SAVED};
type Bytes = Vec<u8>;
type Sema = Vec<Statement>;
type StringSet = BTreeSet<String>;
type Regs = Vec<Reg>;
type Constraints = Vec<Constraint>;
type Vars = Vec<Var>;
type LocSet = Vec<Loc>;
type Vusize = Vec<usize>;
pub type PointsTo = BTreeMap<Var, BTreeSet<Var>>;

lazy_static! {
    static ref STRING_INTERN: Mutex<(Vec<String>, HashMap<String, Interned>)> = Mutex::new((Vec::new(), HashMap::new()));
}

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Copy)]
pub struct Interned {
    index: u8,
}

impl Interned {
    fn max() -> usize {
        ::std::u8::MAX as usize
    }
    pub fn from_string(s: &String) -> Self {
        let mut intern = STRING_INTERN.lock().unwrap();
        if let Some(idx) = intern.1.get(s) {
            return *idx;
        }
        // This would be an if let/else, but rust considers the borrow from the then clause still
        // active
        assert!(intern.0.len() < Interned::max());
        intern.0.push(s.clone());
        let out = Interned {
            index: (intern.0.len() - 1) as u8,
        };
        intern.1.insert(s.clone(), out);
        out
    }
    pub fn to_string(&self) -> String {
        STRING_INTERN.lock().unwrap().0[self.index as usize].clone()
    }
}

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum KillSpec {
    Registers(Vec<Reg>),
    StackFrame(Loc),
}

impl KillSpec {
    pub fn empty() -> Self {
        KillSpec::Registers(Vec::new())
    }
    // As an optimization, realize that registers can never be pointed to, and so we don't need to
    // purge them from the right hand side of points to tables.
    fn kills_vals(&self) -> bool {
        use self::KillSpec::*;
        match *self {
            StackFrame(_) => true,
            Registers(_) => false,
        }
    }
    fn kill(&self, v: &Var) -> bool {
        use self::KillSpec::*;
        use steensgaard::Var::*;
        match (self, v) {
            (&Registers(ref regs), &Register { ref register, .. }) => regs.contains(register),
            (&StackFrame(ref l), &StackSlot { ref func_addr, .. }) => func_addr == l,
            _ => false,
        }
    }
    pub fn purge_pts(&self, pts: &mut PointsTo) {
        let mut keys = Vec::new();
        pts.iter_mut()
            .map(|(k, v)| {
                if self.kill(k) {
                    keys.push(k.clone())
                } else if self.kills_vals() {
                    let vs: Vec<Var> = v.iter().filter(|v| self.kill(v)).cloned().collect();
                    for vi in vs {
                        v.remove(&vi);
                    }
                }
            })
            .count();
        for key in keys {
            pts.remove(&key);
        }
    }
}

fn caller_saved() -> Regs {
    CALLER_SAVED.to_vec()
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

#[derive(Debug, Eq, Clone, PartialEq, PartialOrd, Ord, Hash, Copy)]
pub struct Loc {
    pub file_name: Interned,
    pub addr: u64,
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
    "mycroft/fmt_str.my",
    "mycroft/steensgaard.my",
    "mycroft/flow.my",
    "mycroft/uaf.my",
    "mycroft/queries.my"
);
pub use self::mycroft_program::*;
