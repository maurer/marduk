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

fn apply(pts: &mut PointsTo, c: &Constraint) {
    match *c {
        // a = &b;
        Constraint::AddrOf { ref a, ref b } => {
            let mut bs = BTreeSet::new();
            bs.insert(b.clone());
            pts.insert(a.clone(), bs);
        }
        // a = b;
        Constraint::Asgn { ref a, ref b } => {
            let ptb = pt_get(pts, b);
            pts.insert(a.clone(), ptb);
        }
        // a = *b;
        Constraint::Deref { ref a, ref b } => {
            let ptb = pt_get(pts, b)
                .iter()
                .fold(BTreeSet::new(), |bs, ptb| &bs | &pt_get(pts, ptb));
            pts.insert(a.clone(), ptb);
        }
        // *a = b;
        Constraint::Write { ref a, ref b } => {
            let pta = pt_get(pts, a);
            if pta.len() == 1 {
                let mut bs = BTreeSet::new();
                bs.insert(b.clone());
                pts.insert(pta.iter().next().unwrap().clone(), bs);
            } else {
                for pt in pta {
                    pts.get_mut(&pt).unwrap().insert(b.clone());
                }
            }
        }
        // *a = *b;
        Constraint::Xfer { ref a, ref b } => {
            let pta = pt_get(pts, a);
            let ptb = pt_get(pts, b)
                .iter()
                .fold(BTreeSet::new(), |bs, ptb| &bs | &pt_get(pts, ptb));
            if pta.len() == 1 {
                pts.insert(pta.iter().next().unwrap().clone(), ptb);
            } else {
                for pt in pta {
                    pts.get_mut(&pt).unwrap().append(&mut ptb.clone())
                }
            }
        }
    }
}

pub fn xfer(i: &FlowXferIn) -> Vec<FlowXferOut> {
    let mut pts = i.pts.clone();
    for c in i.cs.iter() {
        apply(&mut pts, c)
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
