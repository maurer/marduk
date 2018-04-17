//! points_to contains the PointsTo type and relevant implementation details.
//! It is used in flow/context sensitive analysis where we don't have a single solution but many,
//! and need to update and propagate data between them.
use datalog::Loc;
use std::collections::btree_map;
use std::collections::{BTreeMap, BTreeSet};
use steensgaard::Var;

/// PointsTo manages information about what a given variable may point to
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Clone, Default)]
pub struct PointsTo {
    inner: BTreeMap<Var, BTreeSet<Var>>,
}

impl PointsTo {
    /// Makes a new empty PointsTo
    pub fn new() -> Self {
        Self::default()
    }

    /// Iterates over the underlying map.
    ///
    /// Do not depend on this iterator type not changing, it's only like this because rust's type
    /// system makes it extremely cumbersome to return an abstract iterator.
    ///
    /// Generally, you should prefer to add a method to PointsTo rather than using the iterator
    /// unless it is really fundamentally iteration.
    pub fn iter(&self) -> btree_map::Iter<Var, BTreeSet<Var>> {
        self.inner.iter()
    }

    /// Gets the set of what a variable may point to, returning an empty set if unmapped
    // I want it to return the empty set when it finds no element, so it can't return a reference.
    pub fn get(&self, v: &Var) -> BTreeSet<Var> {
        match self.inner.get(v) {
            Some(k) => k.clone(),
            None => BTreeSet::new(),
        }
    }

    /// Updates a points-to set with information from another, assuming both represent valid
    /// possibilities.
    pub fn merge(&mut self, other: &Self) {
        for (k, v) in &other.inner {
            match self.inner.entry(k.clone()) {
                btree_map::Entry::Occupied(mut o) => {
                    o.get_mut().append(&mut v.clone());
                }
                btree_map::Entry::Vacant(e) => {
                    e.insert(v.clone());
                }
            };
        }
    }

    /// Removes all references to a variable by predicate.
    /// If the predicate returns true, the variable will be removed.
    pub fn remove_predicate<F: Fn(&Var) -> bool>(&mut self, f: F) {
        let mut keys = Vec::new();
        self.inner
            .iter_mut()
            .map(|(k, v)| {
                if f(k) {
                    keys.push(k.clone())
                } else {
                    let vs: Vec<Var> = v.iter().cloned().filter(&f).collect();
                    for vi in vs {
                        v.remove(&vi);
                    }
                }
            })
            .count();
        for key in keys {
            self.inner.remove(&key);
        }
    }

    // Helper function which gets us a mutable reference to what's pointed to by the provided
    // variable, creating the entry if needed.
    fn force_mut(&mut self, src: Var) -> &mut BTreeSet<Var> {
        self.inner.entry(src).or_insert_with(BTreeSet::new)
    }

    /// src->tgt is a possibility in addition to whatever may have been before.
    pub fn add_alias(&mut self, src: Var, tgt: Var) {
        self.force_mut(src).insert(tgt);
    }

    /// For each element in the tgts set, src may point there in addition to whatever it could
    /// before.
    pub fn extend_alias(&mut self, src: Var, tgts: BTreeSet<Var>) {
        self.force_mut(src).extend(tgts);
    }

    /// src points only to tgt
    pub fn replace_alias(&mut self, src: Var, tgt: Var) {
        let mut bs = BTreeSet::new();
        bs.insert(tgt);
        self.inner.insert(src, bs);
    }

    /// src can only point to members of tgts
    pub fn set_alias(&mut self, src: Var, tgts: BTreeSet<Var>) {
        if tgts.is_empty() {
            self.inner.remove(&src);
        } else {
            self.inner.insert(src, tgts);
        }
    }

    /// Remove temporary variables from the points-to information.
    ///
    /// Since temporaries do not live across instructions, this should always be called between
    /// them to prevent bloat.
    pub fn remove_temps(&mut self) {
        let tmps: Vec<_> = self.inner.keys().filter(|v| v.is_temp()).cloned().collect();
        for tmp in tmps {
            self.inner.remove(&tmp);
        }
    }

    // Helper function to find all values currently pointed to in the points-to set
    fn pt_to(&self) -> BTreeSet<&Var> {
        let mut pointed_to: BTreeSet<&Var> = BTreeSet::new();
        for v in self.inner.values() {
            pointed_to.extend(v);
        }
        pointed_to
    }

    /// Performs a reachability test for dynamic variables and removes them if they are
    /// unreachable.
    pub fn canonicalize(&mut self) {
        let mut updated = true;
        while updated {
            updated = false;
            // Gather all pointed-to values
            let keys_to_purge = {
                let pointed_to = self.pt_to();

                let mut keys_to_purge = Vec::new();
                for k in self.inner.keys() {
                    if k.is_dyn() && !pointed_to.contains(k) {
                        keys_to_purge.push(*k);
                    }
                }
                if keys_to_purge.is_empty() {
                    return;
                }
                keys_to_purge
            };
            if !keys_to_purge.is_empty() {
                updated = true;
            }

            for k in keys_to_purge {
                self.inner.remove(&k);
            }
        }
    }

    /// Finds all locations where v may have been freed.
    pub fn free_sites(&self, v: &Var) -> Vec<Loc> {
        self.get(v)
            .iter()
            .flat_map(|d| self.get(d))
            .filter_map(|pt| match pt {
                Var::Freed { ref site } => Some(*site),
                _ => None,
            })
            .collect()
    }
}

impl ::std::fmt::Display for PointsTo {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        use printers;
        for (k, v) in &self.inner {
            write!(f, "\t{} -> ", k)?;
            printers::fmt_vec(f, &v.iter().collect::<Vec<_>>())?;
            writeln!(f)?;
        }
        Ok(())
    }
}
