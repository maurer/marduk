//! points_to contains the PointsTo type and relevant implementation details.
//! It is used in flow/context sensitive analysis where we don't have a single solution but many,
//! and need to update and propagate data between them.
use load::Loc;
use regs::Reg;
use std::collections::btree_map;
use std::collections::{BTreeMap, BTreeSet};
use var::Var;

#[derive(Eq, PartialEq, Ord, Debug, PartialOrd, Clone, Hash)]
pub struct VarRef {
    pub var: Var,
    pub offset: Option<u64>,
}

impl VarRef {
    fn is_freed(&self) -> bool {
        self.var.is_freed()
    }
}

mod cow_varset {
    const MAX_VAR_REF: usize = 2;
    use super::VarRef;
    use std::collections::BTreeSet;
    use std::ops::{Deref, DerefMut};
    use std::rc::Rc;

    type Inner = BTreeSet<VarRef>;

    #[derive(Default, Eq, PartialEq, Ord, Debug, PartialOrd, Clone, Hash)]
    pub struct VarSet(Rc<Inner>);

    impl Deref for VarSet {
        type Target = Inner;
        fn deref(&self) -> &Inner {
            &self.0
        }
    }

    impl DerefMut for VarSet {
        fn deref_mut(&mut self) -> &mut Inner {
            Rc::make_mut(&mut self.0)
        }
    }

    impl VarSet {
        pub fn new() -> Self {
            VarSet(Rc::new(BTreeSet::new()))
        }
        pub fn insert(&mut self, vr: VarRef) -> bool {
            // 0.) Check if we're here already - we want to early return to avoid widening
            if self.contains(&vr) {
                return false;
            }

            // 1.) Check that we don't already have a v+? in there, if we do, no point in adding it
            let vq = VarRef {
                var: vr.var.clone(),
                offset: None
            };
            if self.contains(&vq) {
                return false;
            }

            // 2.) Count the number of v+x if we're some, or round up if we're none
            let mut vr_alike = Vec::new();
            for vri in self.iter() {
                if vri.var == vr.var {
                    vr_alike.push(vri.clone());
                }
            }

            if vr_alike.len() > MAX_VAR_REF || vr.offset.is_none() {
                for vra in &vr_alike {
                    self.remove(vra);
                }
                let vrn = VarRef {
                    var: vr.var.clone(),
                    offset: None
                };
                return self.deref_mut().insert(vrn)
            }

            // No widening constraints, just insert it
            self.deref_mut().insert(vr)
        }
    }

    impl Extend<VarRef> for VarSet {
        // Potentially, implementing extend like this could be n^2.
        // I'm betting pretty hard on these sets being small...
        fn extend<T: IntoIterator<Item=VarRef>>(&mut self, other: T) {
            for vr in other {
                self.insert(vr);
            }
        }
    }
}

pub use self::cow_varset::VarSet;

#[derive(Default, Eq, PartialEq, Ord, Debug, PartialOrd, Clone, Hash)]
pub struct FieldMap {
    unbounded: VarSet,
    offsets: BTreeMap<u64, VarSet>,
    ub_write: bool,
}

// Currently, we ignore partial reads/writes
// TODO: Document in paper that construction of pointers via means other than arithmetic are
// not dealt with.
impl FieldMap {
    pub fn new() -> Self {
        Self::default()
    }

    fn pt_to(&self) -> BTreeSet<Var> {
        let mut out: BTreeSet<Var> = self.unbounded.iter().map(|v| v.var.clone()).collect();
        for vs in self.offsets.values() {
            out.extend(vs.iter().map(|v| v.var.clone()));
        }
        out
    }

    pub fn merge(&mut self, other: &Self) {
        self.unbounded.extend(other.unbounded.iter().cloned());
        for (k, v) in &other.offsets {
            let mut do_insert = false; // Bool to get around borrowck
            if let Some(our_v) = self.offsets.get_mut(k) {
                our_v.extend(v.iter().cloned());
            } else {
                do_insert = true;
            }
            if do_insert {
                let mut our_v = self.unbounded.clone();
                our_v.extend(v.iter().cloned());
                self.offsets.insert(k.clone(), our_v);
            }
        }
    }

    fn is_empty(&self) -> bool {
        self.unbounded.is_empty() && self.offsets.is_empty()
    }

    fn remove_predicate<F: Fn(&Var) -> bool>(&mut self, f: F) {
        let unbounded_remove: Vec<_> = self.unbounded
            .iter()
            .filter(|vr| f(&vr.var))
            .cloned()
            .collect();
        for vr in unbounded_remove {
            self.unbounded.remove(&vr);
        }
        for vs in self.offsets.values_mut() {
            let to_remove: Vec<_> = vs.iter().filter(|vr| f(&vr.var)).cloned().collect();
            for vr in to_remove {
                vs.remove(&vr);
            }
        }
    }

    fn precise(&self, u_offset: Option<u64>) -> bool {
        let offset = if let Some(offset) = u_offset {
            offset
        } else {
            // If it's an unbounded write, we can't clobber anything
            return false;
        };

        // If an unbounded write has occured, we can never be precise
        if self.ub_write {
            return false;
        }

        // If the offsets table is empty, and the unbounded hasn't been written to,
        // everything is empty already, and we don't care whether we think it's imprecise.
        // We return false because it'll be less work
        if self.offsets.is_empty() {
            return false;
        }

        // If we have more than one address written to, overwriting ub won't be precise.
        if self.offsets.len() > 1 {
            return false;
        }

        // If the address is not in the offsets already, we're adding a second so it's imprecise
        if !self.offsets.contains_key(&offset) {
            return false;
        }

        // We're precise!
        true
    }

    fn write(&mut self, u_offset: Option<u64>, val: VarSet) {
        // If this is register-like (only accessed through one, specific address)
        if self.precise(u_offset) {
            //Reset the unbounded set before extending it, since we know the unbounded data only
            //came from the address we now overwrite.
            self.unbounded.clear();
        }

        if let Some(offset) = u_offset {
            // Destructive update
            self.offsets.insert(offset, val);
        } else {
            self.unbounded.extend(val.iter().cloned());
            self.ub_write = true;
            // We don't understand where the write is, nondestructive updates for everyone
            for vs in self.offsets.values_mut() {
                vs.extend(val.iter().cloned());
            }
        }
    }

    fn read(&self, u_offset: Option<u64>) -> VarSet {
        if let Some(offset) = u_offset {
            match self.offsets.get(&offset) {
                None => self.unbounded.clone(),
                Some(ref vs) => (*vs).clone(),
            }
        } else {
            let mut out = self.unbounded.clone();
            for v in self.offsets.values() {
                out.extend(v.iter().cloned());
            }
            out
        }
    }
}

/// PointsTo manages information about what a given variable may point to
#[derive(Eq, PartialEq, Ord, PartialOrd, Hash, Debug, Clone, Default)]
pub struct PointsTo {
    inner: BTreeMap<Var, FieldMap>,
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
            site: alloc_site.clone(),
            stale: true,
        };
        let fresh = Var::Alloc {
            site: alloc_site.clone(),
            stale: false,
        };
        if self.super_live.contains(&fresh) {
            self.super_live.insert(stale.clone());
        }
        if let Some(pt) = self.inner.remove(&fresh) {
            self.inner.insert(stale.clone(), pt.clone());
            self.inner.insert(fresh.clone(), pt);
        }
        for fm in self.inner.values_mut() {
            let mut u_new = Vec::new();
            for vr in fm.unbounded.iter() {
                if vr.var == fresh {
                    let mut vr_new = vr.clone();
                    vr_new.var = stale.clone();
                    u_new.push(vr_new);
                }
            }
            fm.unbounded.extend(u_new);

            for vs in fm.offsets.values_mut() {
                let mut o_new = Vec::new();
                for vr in vs.iter() {
                    if vr.var == fresh {
                        let mut vr_new = vr.clone();
                        vr_new.var = stale.clone();
                        o_new.push(vr_new);
                    }
                }
                vs.extend(o_new);
            }
        }
    }

    pub fn make_stale(&mut self, alloc_site: &Loc) {
        let stale = Var::Alloc {
            site: alloc_site.clone(),
            stale: true,
        };
        let fresh = Var::Alloc {
            site: alloc_site.clone(),
            stale: false,
        };
        if self.super_live.remove(&fresh) {
            self.super_live.insert(stale.clone());
        }
        if let Some(pt) = self.inner.remove(&fresh) {
            self.inner.insert(stale.clone(), pt);
        }

        for fm in self.inner.values_mut() {
            let mut u_new = Vec::new();
            let mut u_old = Vec::new();
            for vr in fm.unbounded.iter() {
                if vr.var == fresh {
                    let mut vr_new = vr.clone();
                    vr_new.var = stale.clone();
                    u_new.push(vr_new);
                    u_old.push(vr.clone());
                }
            }
            for vr in u_old {
                fm.unbounded.remove(&vr);
            }
            fm.unbounded.extend(u_new);

            for vs in fm.offsets.values_mut() {
                let mut o_new = Vec::new();
                let mut o_old = Vec::new();
                for vr in vs.iter() {
                    if vr.var == fresh {
                        let mut vr_new = vr.clone();
                        vr_new.var = stale.clone();
                        o_new.push(vr_new);
                        o_old.push(vr.clone());
                    }
                }
                for vr in o_old {
                    vs.remove(&vr);
                }
                vs.extend(o_new);
            }
        }
    }

    /// Gets the set of what a variable may point to, returning an empty set if unmapped, including
    /// potential free references
    // I want it to return the empty set when it finds no element, so it can't return a reference.
    fn get_all(&self, v: &VarRef) -> VarSet {
        match self.inner.get(&v.var) {
            Some(k) => k.read(v.offset).clone(),
            None => VarSet::new(),
        }
    }

    /// Gets the set of what a variable may point to, not including any free references
    pub fn get(&self, v: &VarRef) -> VarSet {
        let mut out = VarSet::new();
        out.extend(self.get_all(v).iter().filter(|x| !x.is_freed()).cloned());
        out
    }

    pub fn get_var(&self, v: &Var) -> FieldMap {
        self.inner.get(v).unwrap_or(&FieldMap::new()).clone()
    }

    /// Updates a points-to set with information from another, assuming both represent valid
    /// possibilities.
    pub fn merge(&mut self, other: &Self) {
        for (k, v) in &other.inner {
            match self.inner.entry(k.clone()) {
                btree_map::Entry::Occupied(mut o) => {
                    o.get_mut().merge(&v);
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
                    v.remove_predicate(&f);
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
    fn force_mut(&mut self, src: Var) -> &mut FieldMap {
        self.inner.entry(src).or_insert_with(FieldMap::new)
    }

    /// src->tgts only
    pub fn set_alias(&mut self, src: VarRef, tgts: VarSet) {
        // If we aren't updating anything, and the new field map would be empty, just leave it
        // empty
        if tgts.is_empty() && !self.inner.contains_key(&src.var) {
            return;
        }

        self.force_mut(src.var).write(src.offset, tgts);
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
            pointed_to.extend(v.pt_to());
        }
        pointed_to
    }

    pub fn clobber(&mut self, v: &Var) {
        self.inner.remove(v);
    }

    /// Mark and sweep gc for points-to relationship using roots as a a predicate to identify root
    /// keys
    fn gc<F>(&mut self, roots: F)
    where
        F: Fn(&Var) -> bool,
    {
        // mark
        let mut old_size: isize = -1;
        let mut live: BTreeSet<Var> = BTreeSet::new();
        while live.len() as isize != old_size {
            old_size = live.len() as isize;
            for (k, v) in &self.inner {
                if roots(k) || live.contains(k) {
                    live.insert(k.clone());
                    live.extend(v.pt_to());
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

    pub fn purge_dead(&mut self, live: &[Var]) {
        let mut to_purge = Vec::new();
        for key in self.inner.keys() {
            if !live.contains(key) && !key.is_dyn() && !self.super_live.contains(key) {
                to_purge.push(key.clone());
            }
        }
        for key in to_purge {
            self.inner.remove(&key);
        }
        self.canonicalize();
    }

    /// Finds all locations where v may have been freed.
    pub fn free_sites(&self, v: &Var) -> Vec<Loc> {
        self.get(&VarRef {
            var: v.clone(),
            offset: Some(0),
        }).iter()
            .flat_map(|d| self.get_var(&d.var).pt_to())
            .filter_map(|pt| match pt {
                Var::Freed { ref site } => Some(site.clone()),
                _ => None,
            })
            .collect()
    }
}

impl ::std::fmt::Display for PointsTo {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        write!(f, "frames: ")?;
        for frame in &self.frames {
            write!(f, "{}, ", frame)?;
        }
        writeln!(f)?;
        write!(f, "super_live: ")?;
        for live in &self.super_live {
            write!(f, "{}, ", live)?;
        }
        writeln!(f)?;
        for (k, v) in &self.inner {
            write!(f, "\t{} -> {}", k, v)?;
            writeln!(f)?;
        }
        Ok(())
    }
}

impl ::std::fmt::Display for FieldMap {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::fmt::Result {
        use printers;
        write!(f, "u: ")?;
        printers::fmt_vec(f, &self.unbounded.iter().collect::<Vec<_>>())?;
        for (k, v) in &self.offsets {
            write!(f, "\n{}: ", k)?;
            printers::fmt_vec(f, &v.iter().collect::<Vec<_>>())?;
        }
        Ok(())
    }
}
