//! points_to contains the PointsTo type and relevant implementation details.
//! It is used in flow/context sensitive analysis where we don't have a single solution but many,
//! and need to update and propagate data between them.
use load::Loc;
use regs::Reg;
use std::collections::btree_map;
use std::collections::{BTreeMap, BTreeSet};
use var::Var;

/// PointsTo manages information about what a given variable may point to
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Clone, Default)]
pub struct PointsTo {
    inner: BTreeMap<Var, BTreeSet<Var>>,
    super_live: BTreeSet<Var>,
    frames: BTreeSet<Loc>,
}

impl PointsTo {
    /// Makes a new empty PointsTo
    pub fn new(frame: Loc) -> Self {
        let mut base = Self::default();
        base.add_frame(frame);
        base
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

    /// Filters register definition based on a whitelist
    pub fn only_regs(&mut self, whitelist: &[Reg]) {
        let to_kill: Vec<_> = self.inner
            .keys()
            .filter(|v| match *v {
                Var::Register { register, .. } => !whitelist.contains(register),
                _ => false,
            })
            .cloned()
            .collect();
        for key in to_kill {
            self.inner.remove(&key);
        }
    }

    pub fn make_dup(&mut self, alloc_site: &Loc) {
        let stale = Var::Alloc {
            site: *alloc_site,
            stale: true,
        };
        let fresh = Var::Alloc {
            site: *alloc_site,
            stale: false,
        };
        if self.super_live.contains(&fresh) {
            self.super_live.insert(stale);
        }
        if let Some(pt) = self.inner.remove(&fresh) {
            self.inner.insert(stale, pt.clone());
            self.inner.insert(fresh, pt);
        }
        for pt in self.inner.values_mut() {
            *pt = pt.iter()
                .flat_map(|v| {
                    if v == &fresh {
                        vec![stale, fresh]
                    } else {
                        vec![*v]
                    }
                })
                .collect();
        }
    }

    pub fn make_stale(&mut self, alloc_site: &Loc) {
        let stale = Var::Alloc {
            site: *alloc_site,
            stale: true,
        };
        let fresh = Var::Alloc {
            site: *alloc_site,
            stale: false,
        };
        if self.super_live.remove(&fresh) {
            self.super_live.insert(stale);
        }
        if let Some(pt) = self.inner.remove(&fresh) {
            self.inner.insert(stale, pt);
        }
        for pt in self.inner.values_mut() {
            *pt = pt.iter()
                .map(|v| if v == &fresh { stale } else { *v })
                .collect();
        }
    }

    /// Gets the set of what a variable may point to, returning an empty set if unmapped, including
    /// potential free references
    // I want it to return the empty set when it finds no element, so it can't return a reference.
    fn get_all(&self, v: &Var) -> BTreeSet<Var> {
        match self.inner.get(v) {
            Some(k) => k.clone(),
            None => BTreeSet::new(),
        }
    }

    /// Gets the set of what a variable may point to, not including any free references
    pub fn get(&self, v: &Var) -> BTreeSet<Var> {
        match self.inner.get(v) {
            Some(k) => k.iter().filter(|x| !x.is_freed()).cloned().collect(),
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
        self.super_live.extend(other.super_live.iter().cloned());
        self.frames.extend(other.frames.iter().cloned());
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
                    if v.is_empty() {
                        keys.push(k.clone())
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
        if !tgts.is_empty() {
            self.force_mut(src).extend(tgts);
        }
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
    pub fn pt_to(&self) -> BTreeSet<Var> {
        let mut pointed_to: BTreeSet<Var> = self.super_live.clone();
        for v in self.inner.values() {
            pointed_to.extend(v);
        }
        pointed_to
    }

    /// Mark and sweep gc for points-to relationship using roots as a a predicate to identify root
    /// keys
    fn gc<F>(&mut self, roots: F)
    where
        F: Fn(Var) -> bool,
    {
        // mark
        let mut old_size: isize = -1;
        let mut live: BTreeSet<Var> = BTreeSet::new();
        while live.len() as isize != old_size {
            old_size = live.len() as isize;
            for (k, v) in &self.inner {
                if roots(*k) || live.contains(k) {
                    live.insert(*k);
                    live.extend(v.iter().cloned());
                }
            }
        }
        // sweep
        let dead: Vec<_> = self.inner
            .keys()
            .filter(|x| !live.contains(x))
            .cloned()
            .collect();
        for dead_var in dead {
            self.inner.remove(&dead_var);
        }
    }

    pub fn add_live<T>(&mut self, live: T)
    where
        T: IntoIterator<Item = Var>,
    {
        for v in live {
            self.super_live.insert(v);
        }
    }

    pub fn clear_live(&mut self) {
        self.super_live.clear();
    }

    pub fn add_frame(&mut self, frame: Loc) {
        self.frames.insert(frame);
    }
    pub fn clear_frames(&mut self) {
        self.frames.clear()
    }

    pub fn drop_stack(&mut self) {
        self.gc(|v| !v.is_stack() && !v.is_dyn());
    }

    /// Performs a reachability test for dynamic variables and removes them if they are
    /// unreachable.
    pub fn canonicalize(&mut self) {
        let super_live = self.super_live.clone();
        let frames: Vec<_> = self.frames.iter().cloned().collect();
        self.gc(|v| !v.is_dyn() && !v.other_func(&frames) || super_live.contains(&v));
    }

    /// Finds all locations where v may have been freed.
    pub fn free_sites(&self, v: &Var) -> Vec<Loc> {
        self.get(v)
            .iter()
            .flat_map(|d| self.get_all(d))
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
        write!(f, "frames: ")?;
        for frame in &self.frames {
            write!(f, "{}, ", frame)?;
        }
        writeln!(f)?;
        for (k, v) in &self.inner {
            write!(f, "\t{} -> ", k)?;
            printers::fmt_vec(f, &v.iter().collect::<Vec<_>>())?;
            writeln!(f)?;
        }
        Ok(())
    }
}
