use crate::constraints::{Constraint, VarPath};
use crate::datalog::*;
use crate::points_to::{PointsTo, VarRef, VarSet};
use crate::regs::ARGS;
use crate::var::Var;

fn off_plus(base: &mut Option<u64>, off: Option<u64>) {
    match off {
        Some(off_val) => {
            if let Some(base_val) = base.as_mut() {
                *base_val += off_val;
            }
        }
        None => *base = None,
    }
}

fn lhs_resolve(pts: &PointsTo, vp: VarPath) -> Vec<VarRef> {
    // We can't assign to an address
    assert!(vp.derefs() > 1);
    // If derefs = 2, that means we're in base case e.g. writing to a register or variable directly
    if vp.derefs() == 2 {
        assert!(vp.offsets.last().unwrap().unwrap() == 0);
        // We're on the left here, which means if our last deref is nonzero, this is an address
        // You can't write (*a + b) = c;, which is what a nonzero value would indicate here.
        return vec![VarRef {
            var: vp.base,
            offset: vp.offsets[0],
        }];
    } else {
        // If derefs > 2, that means we need to query the pts and recurse
        let (offset_0, offsets_rest) = vp.offsets.split_at(1);
        let vr0 = VarRef {
            var: vp.base,
            offset: offset_0[0],
        };
        pts.get(&vr0)
            .iter()
            .flat_map(|vr| {
                let mut offsets = offsets_rest.to_vec();
                off_plus(&mut offsets[0], vr.offset);
                let vpp = VarPath {
                    base: vr.var.clone(),
                    offsets,
                };
                lhs_resolve(pts, vpp)
            })
            .collect()
    }
}

fn rhs_resolve(pts: &PointsTo, vp: VarPath) -> Vec<VarRef> {
    // If derefs = 1, we're in the base case - just talking about the variable plus an offset, it
    // fits in a VarRef
    if vp.derefs() == 1 {
        return vec![VarRef {
            var: vp.base,
            offset: vp.offsets[0],
        }];
    } else {
        let (offset_0, offsets_rest) = vp.offsets.split_at(1);
        let vr0 = VarRef {
            var: vp.base,
            offset: offset_0[0],
        };
        pts.get(&vr0)
            .iter()
            .flat_map(|vr| {
                let mut offsets = offsets_rest.to_vec();
                off_plus(&mut offsets[0], vr.offset);
                let vpp = VarPath {
                    base: vr.var.clone(),
                    offsets,
                };
                rhs_resolve(pts, vpp)
            })
            .collect()
    }
}

fn apply(pts: &mut PointsTo, c: &Constraint) {
    trace!("Applying {}", c);
    for rhs in &c.rhss {
        if let Var::Alloc { ref site, .. } = rhs.base {
            pts.make_stale(site);
        }
    }

    // This needs to be done afterwards, because we need all staleness applied first.
    let mut rhses = VarSet::new();
    for rhs in &c.rhss {
        rhses.extend(rhs_resolve(pts, rhs.clone()).into_iter());
    }

    trace!("RHS resolution:");
    for rhs in rhses.iter() {
        trace!("{}", rhs);
    }
    trace!("LHS resolution:");
    let lhsses = lhs_resolve(pts, c.lhs.clone());
    let extend = lhsses.len() > 1;
    for lhs in lhsses {
        trace!("{}", lhs);
        if extend {
            pts.extend_alias(lhs, &rhses);
        } else {
            pts.set_alias(lhs, rhses.clone());
        }
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
    trace!("stack_purge@{}->{}", i.src, i.dst);
    trace!("pre: {}", pts);
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
    trace!("post: {}", pts);
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
        pts: PointsTo::new(i.loc.clone()),
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
        pts: PointsTo::new(i.loc.clone()),
    }]
}
