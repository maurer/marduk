use datalog::*;
use steensgaard::{Constraint, Var};
use datalog::PointsTo;
use std::collections::{BTreeSet, HashMap};
use std::sync::Mutex;

lazy_static! {
    static ref GAS: Mutex<HashMap<Loc, u32>> = Mutex::new(HashMap::new());
}

fn pt_get(pts: &PointsTo, v: &Var) -> BTreeSet<Var> {
    match pts.get(v) {
        Some(k) => k.clone(),
        None => BTreeSet::new(),
    }
}

fn apply(pts: &PointsTo, out_pts: &mut PointsTo, updated: &mut Vec<Var>, c: &Constraint) {
    match *c {
        // *a = &b
        Constraint::StackLoad { ref a, ref b } => {
            // TODO: does this need 'updated' logic?
            let mut bs = BTreeSet::new();
            bs.insert(b.clone());
            // TODO DEDUP
            let pta = pt_get(pts, a);
            if pta.len() == 1 {
                let mut bs = BTreeSet::new();
                bs.insert(b.clone());
                out_pts.insert(pta.iter().next().unwrap().clone(), bs);
            } else {
                for pt in pta {
                    out_pts.get_mut(&pt).map(|ptr| ptr.insert(b.clone()));
                }
            }
        }
        // a = &b;
        Constraint::AddrOf { ref a, ref b } => {
            if updated.contains(a) {
                out_pts.get_mut(a).unwrap().insert(b.clone());
            } else {
                let mut bs = BTreeSet::new();
                bs.insert(b.clone());
                out_pts.insert(a.clone(), bs);
                updated.push(a.clone());
            }
        }
        // a = b;
        Constraint::Asgn { ref a, ref b } => {
            let ptb = pt_get(pts, b);
            if updated.contains(a) {
                out_pts.get_mut(a).unwrap().extend(ptb)
            } else if !ptb.is_empty() {
                out_pts.insert(a.clone(), ptb);
                updated.push(a.clone());
            } else {
                out_pts.remove(a);
            }
        }
        // a = *b;
        Constraint::Deref { ref a, ref b } => {
            let ptb = pt_get(pts, b)
                .iter()
                .fold(BTreeSet::new(), |bs, ptb| &bs | &pt_get(pts, ptb));
            if updated.contains(a) {
                out_pts.get_mut(a).unwrap().extend(ptb);
            } else if !ptb.is_empty() {
                out_pts.insert(a.clone(), ptb);
                updated.push(a.clone());
            } else {
                out_pts.remove(a);
            }
        }
        // *a = b;
        Constraint::Write { ref a, ref b } => {
            // TODO: does this need 'updated' logic?
            let pta = pt_get(pts, a);
            let ptb = pt_get(pts, b);
            if pta.len() == 1 {
                let pt = pta.iter().next().unwrap();
                if ptb.is_empty() {
                    out_pts.remove(pt);
                } else {
                    out_pts.insert(pt.clone(), ptb);
                }
            } else {
                for pt in pta {
                    out_pts.get_mut(&pt).map(|ptr| ptr.extend(ptb.clone()));
                }
            }
        }
        // *a = *b;
        Constraint::Xfer { ref a, ref b } => {
            // TODO: does this need 'updated' logic?
            let pta = pt_get(pts, a);
            let ptb = pt_get(pts, b)
                .iter()
                .fold(BTreeSet::new(), |bs, ptb| &bs | &pt_get(pts, ptb));
            if pta.len() == 1 {
                let pt = pta.iter().next().unwrap();
                if ptb.is_empty() {
                    out_pts.remove(pt);
                } else {
                    out_pts.insert(pt.clone(), ptb);
                }
            } else {
                for pt in pta {
                    out_pts.get_mut(&pt).map(|ptr| ptr.append(&mut ptb.clone()));
                }
            }
        }
    }
}

pub fn xfer(i: &FlowXferIn) -> Vec<FlowXferOut> {
    {
        let mut gases = GAS.lock().unwrap();
        let gas = gases.entry(*i.loc).or_insert(i.gas);
        if *gas == 0 {
            return Vec::new();
        } else {
            *gas -= 1;
        }
    }
    let mut pts = i.pts.clone();
    i.ks.purge_pts(&mut pts);
    let mut updated = Vec::new();
    for c in i.cs.iter() {
        apply(&i.pts, &mut pts, &mut updated, c)
    }
    let tmps: Vec<_> = pts.keys()
        .filter(|v| match **v {
            Var::Temp { .. } => true,
            _ => false,
        })
        .cloned()
        .collect();
    for tmp in tmps {
        pts.remove(&tmp);
    }
    canonicalize(&mut pts);
    vec![FlowXferOut { pts2: pts }]
}

// Purge any entries which cannot currently be reached. We do this on the way in for register
// entries via insn killsets, for stack slots via return call special handling killsets.
// However, this still leaves dormant dyn variables, which will propagate around and bloat things.
fn canonicalize(pts: &mut PointsTo) {
    // Gather all pointed-to values
    let keys_to_purge = {
        let mut pointed_to: BTreeSet<&Var> = BTreeSet::new();
        for v in pts.values() {
            pointed_to.extend(v);
        }
        let mut keys_to_purge = Vec::new();
        for k in pts.keys() {
            if k.is_dyn() && !pointed_to.contains(k) {
                keys_to_purge.push(*k);
            }
        }
        if keys_to_purge.is_empty() {
            return;
        }
        keys_to_purge
    };
    for k in keys_to_purge {
        pts.remove(&k);
    }
    canonicalize(pts)
}

pub fn is_freed(i: &FlowIsFreedIn) -> Vec<FlowIsFreedOut> {
    match i.pts.get(i.v) {
        Some(pts) => pts.iter()
            .flat_map(|heap| match i.pts.get(heap) {
                Some(vars) => vars.iter()
                    .filter_map(|var| match *var {
                        Var::Freed { ref site } => Some(FlowIsFreedOut { loc: site.clone() }),
                        _ => None,
                    })
                    .collect::<Vec<_>>(), // This allocation shouldn't be needed, doing it to make typechecking none arm easier
                None => Vec::new(),
            })
            .collect(),
        None => Vec::new(),
    }
}
