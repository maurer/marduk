use datalog::*;
use steensgaard::{Constraint, Var};
use points_to::PointsTo;
use std::collections::BTreeSet;

fn apply(pts: &PointsTo, out_pts: &mut PointsTo, updated: &mut Vec<Var>, c: &Constraint) {
    match *c {
        // *a = &b
        Constraint::StackLoad { ref a, ref b } => for pt in pts.get(a) {
            out_pts.add_alias(pt, *b);
        },
        // a = &b;
        Constraint::AddrOf { ref a, ref b } => {
            if updated.contains(a) {
                out_pts.add_alias(*a, *b);
            } else {
                out_pts.replace_alias(*a, *b);
                updated.push(*a);
            }
        }
        // a = b;
        Constraint::Asgn { ref a, ref b } => {
            let ptb = pts.get(b);
            if updated.contains(a) {
                out_pts.extend_alias(*a, ptb);
            } else {
                out_pts.set_alias(*a, ptb);
                updated.push(*a);
            }
        }
        // a = *b;
        Constraint::Deref { ref a, ref b } => {
            let ptb = pts.get(b)
                .iter()
                .fold(BTreeSet::new(), |bs, ptb| &bs | &pts.get(ptb));
            if updated.contains(a) {
                out_pts.extend_alias(*a, ptb);
            } else {
                out_pts.set_alias(*a, ptb);
                updated.push(a.clone());
            }
        }
        // *a = b;
        Constraint::Write { ref a, ref b } => {
            let pta = pts.get(a);
            let ptb = pts.get(b);
            for pt in pta {
                out_pts.extend_alias(pt, ptb.clone());
            }
        }
        // *a = *b;
        Constraint::Xfer { ref a, ref b } => {
            let pta = pts.get(a);
            let ptb = pts.get(b)
                .iter()
                .fold(BTreeSet::new(), |bs, ptb| &bs | &pts.get(ptb));
            for pt in pta {
                out_pts.extend_alias(pt, ptb.clone());
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
    pts.remove_temps();
    pts.canonicalize();
    vec![FlowXferOut { pts2: pts }]
}

pub fn is_freed(i: &FlowIsFreedIn) -> Vec<FlowIsFreedOut> {
    i.pts
        .free_sites(i.v)
        .into_iter()
        .map(|site| FlowIsFreedOut { loc: site })
        .collect()
}
