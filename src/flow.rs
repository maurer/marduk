use constraints::Constraint;
use datalog::*;
use points_to::PointsTo;
use regs::ARGS;
use std::collections::BTreeSet;
use var::Var;

fn apply(pts: &mut PointsTo, c: &Constraint) {
    match *c {
        // *a = &b
        Constraint::StackLoad { ref a, ref b } => for pt in pts.get(a) {
            pts.add_alias(pt, b.clone());
        },
        // a = &b;
        Constraint::AddrOf { ref a, ref b } => {
            if let Var::Alloc { ref site, .. } = *b {
                pts.make_stale(site);
            }
            pts.replace_alias(a.clone(), b.clone());
        }
        // a = b;
        Constraint::Asgn { ref a, ref b } => {
            let ptb = pts.get(b);
            pts.set_alias(a.clone(), ptb);
        }
        // a = *b;
        Constraint::Deref { ref a, ref b } => {
            let ptb = pts
                .get(b)
                .iter()
                .fold(BTreeSet::new(), |bs, ptb| &bs | &pts.get(ptb));
            pts.set_alias(a.clone(), ptb);
        }
        // *a = b;
        Constraint::Write { ref a, ref b } => {
            let pta = pts.get(a);
            let ptb = pts.get(b);
            for pt in pta {
                pts.extend_alias(pt, ptb.clone());
            }
        }
        // *a = *b;
        Constraint::Xfer { ref a, ref b } => {
            let pta = pts.get(a);
            let ptb = pts
                .get(b)
                .iter()
                .fold(BTreeSet::new(), |bs, ptb| &bs | &pts.get(ptb));
            for pt in pta {
                pts.extend_alias(pt, ptb.clone());
            }
        }
        // a = const
        Constraint::Clobber { ref v } => pts.clobber(v),
    }
}

pub fn xfer(i: &FlowXferIn) -> Vec<FlowXferOut> {
    trace!("addr {}:\n{}", i.loc, i.pts);
    let mut pts = i.pts.clone();
    for c in i.cs {
        trace!("{}\n", c);
        apply(&mut pts, c);
    }
    pts.remove_temps();
    pts.canonicalize();
    trace!("prepurge:\n{}", pts);
    pts.purge_dead(i.vars);
    trace!("postlive:\n{}", pts);
    trace!("ks:\n{:?}", i.ks);
    i.ks.purge_pts(&mut pts);
    trace!("postpurge:\n{}", pts);
    vec![FlowXferOut { pts2: pts }]
}

pub fn is_freed(i: &FlowIsFreedIn) -> Vec<FlowIsFreedOut> {
    i.pts
        .free_sites(i.v)
        .into_iter()
        .map(|site| FlowIsFreedOut { loc: site })
        .collect()
}

pub fn stack_purge(i: &FlowStackPurgeIn) -> Vec<FlowStackPurgeOut> {
    let mut pts = i.pts.clone();
    pts.clear_live();
    pts.clear_frames();
    pts.only_regs(ARGS);
    //TODO: Now that I have clear_frames, can drop_stack here be replaced by a call to
    //canonicalize()?
    pts.drop_stack();
    let new_live: Vec<_> = i
        .pts
        .pt_to()
        .into_iter()
        .filter(|v| v.is_dyn() || v.is_stack())
        .collect();
    pts.add_live(new_live);
    pts.add_frame(i.dst.clone());
    vec![FlowStackPurgeOut { pts2: pts }]
}

pub fn dyn_clear(i: &FlowDynClearIn) -> Vec<FlowDynClearOut> {
    let mut pts = i.pts.clone();
    pts.clear_live();
    pts.clear_frames();
    pts.add_frame(i.base.clone());
    vec![FlowDynClearOut { pts2: pts }]
}

pub fn base_pts(i: &FlowBasePtsIn) -> Vec<FlowBasePtsOut> {
    vec![FlowBasePtsOut {
        pts: PointsTo::new(i.base.clone()),
    }]
}

pub fn promote_loc(i: &FlowPromoteLocIn) -> Vec<FlowPromoteLocOut> {
    vec![FlowPromoteLocOut {
        src_promoted: vec![i.src.clone()],
    }]
}

pub fn count(i: &FlowCountIn) -> Vec<FlowCountOut> {
    vec![FlowCountOut {
        count: i.preds.len(),
    }]
}

pub fn empty_pts(i: &FlowEmptyPtsIn) -> Vec<FlowEmptyPtsOut> {
    vec![FlowEmptyPtsOut {
        pts: PointsTo::new(i.loc.clone())
    }]
}
