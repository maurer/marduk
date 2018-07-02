use datalog::*;
use load::Loc;
use std::collections::BTreeSet;

#[derive(Debug, Clone, Eq, Ord, Hash, PartialOrd, PartialEq, Default)]
pub struct Effect {
    does_malloc: BTreeSet<Loc>,
    maybe_malloc: BTreeSet<Loc>,
}

impl Effect {
    pub fn nop() -> Self {
        Self::default()
    }
    pub fn merge(&self, other: &Self) -> Self {
        let does_malloc = self.does_malloc
            .intersection(&other.does_malloc)
            .cloned()
            .collect();
        let mut maybe_malloc: BTreeSet<Loc> = self.maybe_malloc
            .union(&other.maybe_malloc)
            .cloned()
            .collect();
        maybe_malloc.extend(
            self.does_malloc
                .symmetric_difference(&other.does_malloc)
                .cloned(),
        );

        Self {
            does_malloc,
            maybe_malloc,
        }
    }
    fn apply(&mut self, other: &Self) {
        for site in &other.does_malloc {
            self.malloc(site)
        }
        for site in &other.maybe_malloc {
            self.maybe_malloc(site);
        }
    }

    fn malloc(&mut self, site: &Loc) {
        self.does_malloc.insert(site.clone());
        self.maybe_malloc.remove(site);
    }

    fn maybe_malloc(&mut self, site: &Loc) {
        self.does_malloc.remove(site);
        self.maybe_malloc.insert(site.clone());
    }
}

pub fn apply_effect(i: &EffectApplyEffectIn) -> Vec<EffectApplyEffectOut> {
    let mut out = i.effect.clone();
    out.apply(i.effect_call);
    vec![EffectApplyEffectOut { effect2: out }]
}

pub fn remote_apply_effect(i: &EffectRemoteApplyEffectIn) -> Vec<EffectRemoteApplyEffectOut> {
    if !::load::malloc_name(i.name) {
        let mut out = i.effect.clone();
        out.apply(i.effect_call);
        vec![EffectRemoteApplyEffectOut { effect2: out }]
    } else {
        Vec::new()
    }
}

pub fn malloc(i: &EffectMallocIn) -> Vec<EffectMallocOut> {
    let mut effect2 = i.effect.clone();
    effect2.malloc(i.local);
    vec![EffectMallocOut { effect2 }]
}

pub fn update_pts(i: &EffectUpdatePtsIn) -> Vec<EffectUpdatePtsOut> {
    trace!("Updating pts with effect: {:?}", i.effect);
    trace!("Pre: {}", i.pts);
    let mut pts2 = i.pts.clone();
    for site in &i.effect.does_malloc {
        pts2.make_stale(site);
    }
    for site in &i.effect.maybe_malloc {
        pts2.make_dup(site);
    }
    trace!("Post: {}", pts2);
    vec![EffectUpdatePtsOut { pts2 }]
}
