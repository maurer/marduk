use bap::high::bil::Statement;
use bap::high::bil;

use std::collections::{BTreeMap, BTreeSet, HashMap};

use datalog::Loc;

#[derive(Clone, Eq, Ord, Hash, PartialOrd, PartialEq, Debug)]
pub enum Var {
    StackSlot {
        func_addr: Loc,
        offset: usize,
    },
    Register {
        site: Loc,
        register: String,
        tmp: bool,
    },
    Alloc {
        site: Loc,
    },
}

// Maps a register at a code address to the list of possible definition sites (for a specific
// location)
pub type DefChain = BTreeMap<String, BTreeSet<Loc>>;

fn move_walk<A, F: Fn(&bil::Variable, &bil::Expression, &mut DefChain, &Loc, &Loc) -> Vec<A>>(
    stmt: &Statement,
    defs: &mut DefChain,
    cur_addr: &Loc,
    func_addr: &Loc,
    f: &F,
) -> Vec<A> {
    match *stmt {
        Statement::Jump(_) | Statement::Special | Statement::CPUException(_) => Vec::new(),
        Statement::While { ref body, .. } => {
            // We pass over the body twice to get the flow sensitivity on variables right
            let mut out: Vec<A> = body.iter()
                .flat_map(|stmt| move_walk(stmt, defs, cur_addr, func_addr, f))
                .collect();
            out.extend(
                body.iter()
                    .flat_map(|stmt| move_walk(stmt, defs, cur_addr, func_addr, f))
                    .collect::<Vec<_>>(),
            );
            out
        }
        Statement::IfThenElse {
            ref then_clause,
            ref else_clause,
            ..
        } => {
            // Process left, then right, then merge defs
            let mut else_defs = defs.clone();
            let then_out: Vec<_> = then_clause
                .iter()
                .flat_map(|stmt| move_walk(stmt, defs, cur_addr, func_addr, f))
                .collect();
            let else_out: Vec<_> = else_clause
                .iter()
                .flat_map(|stmt| move_walk(stmt, &mut else_defs, cur_addr, func_addr, f))
                .collect();

            // Merge back else defs info
            for (k, v) in else_defs {
                let e = defs.entry(k).or_insert_with(BTreeSet::new);
                *e = e.union(&v).cloned().collect()
            }

            let mut out = then_out;
            out.extend(else_out);
            out
        }
        Statement::Move { ref lhs, ref rhs } => f(lhs, rhs, defs, cur_addr, func_addr),
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
enum E {
    AddrOf(Var),
    Base(Var),
    Deref(Var),
}

fn extract_expr(
    e: &bil::Expression,
    defs: &mut DefChain,
    cur_addr: &Loc,
    func_addr: &Loc,
) -> Vec<E> {
    use bap::high::bil::Expression::*;
    use num_traits::ToPrimitive;
    match *e {
        // TODO: Forward stack frame information in here so we can detect stack slots off %rbp
        Var(ref bv) => {
            if bv.type_ == bil::Type::Immediate(1) {
                Vec::new()
            } else if bv.name == "RSP" {
                vec![
                    E::AddrOf(self::Var::StackSlot {
                        func_addr: func_addr.clone(),
                        offset: 0,
                    }),
                ]
            } else {
                match defs.get(&bv.name) {
                    None => Vec::new(),
                    Some(sites) => sites
                        .iter()
                        .map(|site| {
                            E::Base(self::Var::Register {
                                site: site.clone(),
                                register: bv.name.clone(),
                                tmp: bv.tmp,
                            })
                        })
                        .collect(),
                }
            }
        }
        // Disabled for speed, enable for global tracking
        Const(_) => Vec::new(),
        Load { ref index, .. } => extract_expr(index, defs, cur_addr, func_addr)
            .into_iter()
            .map(|e| match e {
                E::Base(v) => E::Deref(v),
                E::AddrOf(v) => E::Base(v),
                _ => panic!("doubly nested load"),
            })
            .collect(),
        Store { .. } => panic!("Extracting on memory"),
        // Adjust here for field sensitivity
        BinOp {
            ref lhs,
            ref rhs,
            op,
        } => {
            let mut out = extract_expr(lhs, defs, cur_addr, func_addr);
            out.extend(extract_expr(lhs, defs, cur_addr, func_addr));
            if op == bil::BinOp::Add {
                if let Var(ref lv) = **lhs {
                    if lv.name == "RSP" {
                        match **rhs {
                            Const(ref bv) => vec![
                                E::AddrOf(self::Var::StackSlot {
                                    func_addr: func_addr.clone(),
                                    offset: bv.to_u64().unwrap() as usize,
                                }),
                            ],
                            _ => out,
                        }
                    } else {
                        out
                    }
                } else if let Var(ref rv) = **rhs {
                    if rv.name == "RSP" {
                        match **lhs {
                            Const(ref bv) => vec![
                                E::AddrOf(self::Var::StackSlot {
                                    func_addr: func_addr.clone(),
                                    offset: bv.to_u64().unwrap() as usize,
                                }),
                            ],
                            _ => out,
                        }
                    } else {
                        out
                    }
                } else {
                    out
                }
            } else {
                out
            }
        }
        IfThenElse {
            true_expr: ref lhs,
            false_expr: ref rhs,
            ..
        } => {
            let mut out = extract_expr(&*lhs, defs, cur_addr, func_addr);
            out.extend(extract_expr(&*rhs, defs, cur_addr, func_addr));
            out
        }
        Let { .. } => panic!("let unimpl"),
        Unknown { .. } | UnOp { .. } | Extract { .. } | Concat { .. } | Cast { .. } => Vec::new(),
    }
}

fn extract_move_var(
    lhs: &bil::Variable,
    rhs: &bil::Expression,
    defs: &mut DefChain,
    cur_addr: &Loc,
    func_addr: &Loc,
) -> Vec<Var> {
    match lhs.type_ {
        bil::Type::Memory { .. } => {
            use self::E::*;
            let (index, rhs) = if let bil::Expression::Store {
                ref index,
                ref value,
                ..
            } = *rhs
            {
                (index, value)
            } else {
                panic!("Writing to memory, but the expression isn't a store")
            };
            let lhs_vars = extract_expr(index, defs, cur_addr, func_addr);
            println!("lhs_vars={:?}", lhs_vars);
            let rhs_vars = extract_expr(rhs, defs, cur_addr, func_addr);
            println!("rhs_vars={:?}", rhs_vars);
            let mut out = Vec::new();
            for lhs_evar in lhs_vars {
                match lhs_evar {
                    Base(l) => out.push(l),
                    _ => (),
                }
            }
            for rhs_evar in rhs_vars {
                match rhs_evar {
                    Deref(v) => out.push(v),
                    _ => (),
                }
            }
            out
        }
        bil::Type::Immediate(_) => {
            let mut out = Vec::new();
            for eval in extract_expr(rhs, defs, cur_addr, func_addr) {
                match eval {
                    E::Deref(v) => out.push(v),
                    _ => (),
                }
            }
            out
        }
    }
}

fn extract_move(
    lhs: &bil::Variable,
    rhs: &bil::Expression,
    defs: &mut DefChain,
    cur_addr: &Loc,
    func_addr: &Loc,
) -> Vec<Constraint> {
    println!("Move extract, lhs={:?}, rhs={:?}", lhs, rhs);
    match lhs.type_ {
        bil::Type::Memory { .. } => {
            use self::E::*;
            let (index, rhs) = if let bil::Expression::Store {
                ref index,
                ref value,
                ..
            } = *rhs
            {
                (index, value)
            } else {
                panic!("Writing to memory, but the expression isn't a store")
            };
            let lhs_vars = extract_expr(index, defs, cur_addr, func_addr);
            println!("lhs_vars={:?}", lhs_vars);
            let rhs_vars = extract_expr(rhs, defs, cur_addr, func_addr);
            println!("rhs_vars={:?}", rhs_vars);
            let mut out = Vec::new();
            for lhs_evar in lhs_vars {
                for rhs_evar in rhs_vars.clone() {
                    out.push(match (lhs_evar.clone(), rhs_evar) {
                        (AddrOf(l), AddrOf(r)) => Constraint::AddrOf { a: l, b: r },
                        (AddrOf(l), Base(r)) => Constraint::Asgn { a: l, b: r },
                        (AddrOf(l), Deref(r)) => Constraint::Deref { a: l, b: r },
                        (Deref(_), _) => panic!("**a = x ?"),
                        (Base(_), AddrOf(_)) => panic!("*a = &b?"),
                        (Base(l), Base(r)) => Constraint::Write { a: l, b: r },
                        (Base(l), Deref(r)) => Constraint::Xfer { a: l, b: r },
                    })
                }
            }
            out
        }
        // Ignore flags
        bil::Type::Immediate(1) => Vec::new(),
        bil::Type::Immediate(_) => {
            let lv = Var::Register {
                site: cur_addr.clone(),
                register: lhs.name.clone(),
                tmp: lhs.tmp,
            };
            let out = extract_expr(rhs, defs, cur_addr, func_addr)
                .into_iter()
                .map(|eval| match eval {
                    E::AddrOf(var) => Constraint::AddrOf {
                        a: lv.clone(),
                        b: var,
                    },
                    E::Base(var) => Constraint::Asgn {
                        a: lv.clone(),
                        b: var,
                    },
                    E::Deref(var) => Constraint::Deref {
                        a: lv.clone(),
                        b: var,
                    },
                })
                .collect();
            if !lhs.tmp {
                // We've just overwritten a non-temporary, update the def chain
                let mut our_addr = BTreeSet::new();
                our_addr.insert(cur_addr.clone());
                defs.insert(lhs.name.clone(), our_addr);
            }
            out
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
pub enum Constraint {
    // a = &b;
    AddrOf { a: Var, b: Var },
    // a = b
    Asgn { a: Var, b: Var },
    // a = *b
    Deref { a: Var, b: Var },
    // *a = b
    Write { a: Var, b: Var },
    // *a = *b
    Xfer { a: Var, b: Var },
}

fn not_tmp(v: &Var) -> bool {
    match *v {
        Var::Register { ref tmp, .. } => !tmp,
        _ => true,
    }
}

pub fn extract_constraints(
    sema: &[Statement],
    mut defs: DefChain,
    cur: &Loc,
    func_loc: &Loc,
) -> Vec<Constraint> {
    let mut constraints = Vec::new();
    for stmt in sema {
        constraints.extend(move_walk(stmt, &mut defs, cur, func_loc, &extract_move));
    }
    constraints
}

pub fn extract_var_use(
    sema: &[Statement],
    mut defs: DefChain,
    cur: &Loc,
    func_loc: &Loc,
) -> Vec<Var> {
    let mut vars = Vec::new();
    for stmt in sema {
        vars.extend(move_walk(stmt, &mut defs, cur, func_loc, &extract_move_var));
    }
    vars
}

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
                pays.push(Some(v));
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
        match (self.points_to[ka], self.points_to[kb]) {
            (Some(pa), Some(pb)) => self.merge(pa, pb),
            (Some(pa), None) => self.points_to[kb] = Some(pa),
            (None, Some(pb)) => self.points_to[ka] = Some(pb),
            (None, None) => (),
        }
    }

    fn dump_sets(&self) -> Vec<Vec<Var>> {
        let mut merger: HashMap<usize, Vec<Var>> = HashMap::new();
        for (key, mvar) in self.pays.iter().enumerate() {
            match *mvar {
                Some(ref var) => merger
                    .entry(self.uf_find(key))
                    .or_insert(Vec::new())
                    .push(var.clone()),
                None => (),
            }
        }
        merger.into_iter().map(|x| x.1).collect()
    }

    fn process(&mut self, c: Constraint) {
        use self::Constraint::*;
        match c {
            // a = &b
            AddrOf { a, b } => self.process(Write { a, b }),
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
        }
    }
}

pub fn constraints_to_may_alias(cs: Vec<Constraint>) -> Vec<Vec<Var>> {
    let mut uf = UF::new();
    for c in cs {
        uf.process(c)
    }
    // We need to track temps during solving, but afterwards we only care about what's at
    // instruction boundaries.
    uf.dump_sets()
        .into_iter()
        .map(|vs| vs.into_iter().filter(not_tmp).collect())
        .collect()
}
