use constraints::Constraint;
use datalog::*;
use std::collections::HashMap;
use var::Var;

#[derive(Default, Debug, Eq, PartialOrd, Ord, PartialEq, Clone, Copy)]
struct UFS {
    rank: usize,
    parent: Option<usize>,
}

struct UF {
    backing: Vec<UFS>,
    pays: Vec<Option<Var>>,
    inv: HashMap<Var, usize>,
    points_to: Vec<Option<usize>>,
}

impl UF {
    fn new() -> Self {
        UF {
            backing: Vec::new(),
            pays: Vec::new(),
            inv: HashMap::new(),
            points_to: Vec::new(),
        }
    }
    fn uf_find(&self, k: usize) -> usize {
        match self.backing[k].parent {
            Some(p) => self.uf_find(p),
            None => k,
        }
    }
    // Finds the key that matches the var, or creates the set if it doesn't exist
    fn force_find(&mut self, v: Var) -> usize {
        let k0 = {
            let backing = &mut self.backing;
            let pays = &mut self.pays;
            let points_to = &mut self.points_to;

            *self.inv.entry(v.clone()).or_insert_with(|| {
                backing.push(Default::default());
                pays.push(Some(v.clone()));
                points_to.push(None);
                pays.len() - 1
            })
        };
        self.uf_find(k0)
    }
    // Finds the points to set for this key, or synthesizes one if it does not exist
    fn force_points_to(&mut self, k: usize) -> usize {
        match self.points_to[k] {
            Some(v) => v,
            None => {
                self.backing.push(Default::default());
                self.pays.push(None);
                let v = self.pays.len() - 1;
                self.points_to.push(None);
                self.points_to[k] = Some(v);
                v
            }
        }
    }

    fn uf_union(&mut self, k0: usize, k1: usize) {
        use std::cmp::Ordering;
        let r0 = self.uf_find(k0);
        let r1 = self.uf_find(k1);
        if r0 == r1 {
            return;
        }
        match self.backing[r0].rank.cmp(&self.backing[r1].rank) {
            Ordering::Less => self.backing[r0].parent = Some(r1),
            Ordering::Greater => self.backing[r1].parent = Some(r0),
            Ordering::Equal => {
                self.backing[r0].parent = Some(r1);
                self.backing[r1].rank += 1;
            }
        }
    }

    fn merge(&mut self, ka: usize, kb: usize) {
        if ka == kb {
            return;
        }
        self.uf_union(ka, kb);
        match (
            self.points_to[ka].map(|p| self.uf_find(p)),
            self.points_to[kb].map(|p| self.uf_find(p)),
        ) {
            (Some(pa), Some(pb)) => self.merge(pa, pb),
            (Some(pa), None) => self.points_to[kb] = Some(pa),
            (None, Some(pb)) => self.points_to[ka] = Some(pb),
            (None, None) => (),
        }
    }

    fn dump_sets(&self) -> Vec<Vec<Var>> {
        let mut merger: HashMap<usize, Vec<Var>> = HashMap::new();
        for (key, mvar) in self.pays.iter().enumerate() {
            if let Some(ref var) = *mvar {
                merger
                    .entry(self.uf_find(key))
                    .or_insert_with(Vec::new)
                    .push(var.clone())
            }
        }
        merger.into_iter().map(|x| x.1).collect()
    }

    fn process(&mut self, c: &Constraint) {
        use constraints::Constraint::*;
        match c.clone() {
            // a = &b
            AddrOf { a, b } => self.process(&Write { a, b }),
            // a = b
            Asgn { a, b } => {
                let ka = self.force_find(a);
                let kb = self.force_find(b);
                self.merge(ka, kb);
            }
            // a = *b
            Deref { a, b } => {
                let ka = self.force_find(a);
                let kb = self.force_find(b);
                let pb = self.force_points_to(kb);
                self.merge(ka, pb);
            }
            // *a = b
            Write { a, b } => {
                let ka = self.force_find(a);
                let kb = self.force_find(b);
                let pa = self.force_points_to(ka);
                self.merge(pa, kb);
            }
            // *a = *b
            Xfer { a, b } => {
                let ka = self.force_find(a);
                let kb = self.force_find(b);
                let pa = self.force_points_to(ka);
                let pb = self.force_points_to(kb);
                self.merge(pa, pb)
            }
            // *a = &b
            StackLoad { a, b } => {
                let ka = self.force_find(a);
                let kb = self.force_find(b);
                let pa = self.force_points_to(ka);
                let ppa = self.force_points_to(pa);
                self.merge(ppa, kb);
            }
        }
    }
}

pub fn constraints_to_may_alias(cs: Vec<Constraint>) -> Vec<Vec<Var>> {
    let mut uf = UF::new();
    for c in cs {
        uf.process(&c)
    }
    // We need to track temps during solving, but afterwards we only care about what's at
    // instruction boundaries.
    uf.dump_sets()
        .into_iter()
        .map(|vs| vs.into_iter().filter(|v| !v.is_temp()).collect())
        .collect()
}

pub fn steens_solve(i: &SteensgaardSteensSolveIn) -> Vec<SteensgaardSteensSolveOut> {
    ::steensgaard::constraints_to_may_alias(i.cs.concat())
        .into_iter()
        .map(|vs| SteensgaardSteensSolveOut { vs })
        .collect()
}

pub fn steens_expando(i: &SteensgaardSteensExpandoIn) -> Vec<SteensgaardSteensExpandoOut> {
    i.vs
        .iter()
        .map(|v| SteensgaardSteensExpandoOut { v: v.clone() })
        .collect()
}
