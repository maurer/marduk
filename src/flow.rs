use datalog::*;
use steensgaard::{Constraint, Var};
use datalog::PointsTo;
use std::collections::BTreeSet;

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
            } else {
                out_pts.insert(a.clone(), ptb);
                updated.push(a.clone());
            }
        }
        // a = *b;
        Constraint::Deref { ref a, ref b } => {
            let ptb = pt_get(pts, b)
                .iter()
                .fold(BTreeSet::new(), |bs, ptb| &bs | &pt_get(pts, ptb));
            if updated.contains(a) {
                out_pts.get_mut(a).unwrap().extend(ptb);
            } else {
                out_pts.insert(a.clone(), ptb);
                updated.push(a.clone());
            }
        }
        // *a = b;
        Constraint::Write { ref a, ref b } => {
            // TODO: does this need 'updated' logic?
            let pta = pt_get(pts, a);
            let ptb = pt_get(pts, b);
            if pta.len() == 1 {
                out_pts.insert(pta.iter().next().unwrap().clone(), ptb);
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
                out_pts.insert(pta.iter().next().unwrap().clone(), ptb);
            } else {
                for pt in pta {
                    out_pts.get_mut(&pt).map(|ptr| ptr.append(&mut ptb.clone()));
                }
            }
        }
    }
}

pub fn xfer(i: &FlowXferIn) -> Vec<FlowXferOut> {
    let mut pts = i.pts.clone();
    i.ks.purge_pts(&mut pts);
    let mut updated = Vec::new();
    for c in i.cs.iter() {
        apply(&i.pts, &mut pts, &mut updated, c)
    }
    let tmps: Vec<_> = pts.keys()
        .filter(|v| match **v {
            Var::Register { tmp: true, .. } => true,
            _ => false,
        })
        .cloned()
        .collect();
    for tmp in tmps {
        pts.remove(&tmp);
    }
    vec![FlowXferOut { pts2: pts }]
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
