use bap::high::bil::{Statement, Type, Variable};
use datalog::*;
use load::Loc;
use points_to::PointsTo;
use regs::{Reg, RET_REG};
use std::collections::BTreeSet;
use std::str::FromStr;
use var::Var;

#[derive(Debug, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
pub enum KillSpec {
    Registers(Vec<Reg>),
    StackFrame(Loc),
}

impl KillSpec {
    pub fn empty() -> Self {
        KillSpec::Registers(Vec::new())
    }
    fn kill(&self, v: &Var) -> bool {
        // TODO: suppress kill if definition site and kill site are equal?
        use self::KillSpec::*;
        use var::Var::*;
        match (self, v) {
            (&Registers(ref rs), &Register {ref register, ..}) => rs.contains(register),
            (&StackFrame(ref l), &StackSlot { ref func_addr, .. }) => func_addr == l,
            (&StackFrame(_), &Register { ref register, .. }) => register != &RET_REG,
            _ => false,
        }
    }
    pub fn purge_pts(&self, pts: &mut PointsTo) {
        pts.remove_predicate(|v| self.kill(v));
    }
}

// Datalog functions

pub fn killspec_regs(i: &UseDefKillspecRegsIn) -> Vec<UseDefKillspecRegsOut> {
    vec![UseDefKillspecRegsOut {
        ks: KillSpec::empty()
    }]
}

pub fn stack_wipe(i: &UseDefStackWipeIn) -> Vec<UseDefStackWipeOut> {
    vec![UseDefStackWipeOut {
        ks: KillSpec::StackFrame(i.base.clone()),
    }]
}

fn is_normal_reg(r: &Variable) -> bool {
    match r.type_ {
        Type::Immediate(x) => x > 1,
        _ => false,
    }
}

fn defines_stmt(stmt: &Statement, defs: &mut BTreeSet<Reg>) {
    match *stmt {
        Statement::Move { ref lhs, .. } => {
            if !lhs.tmp && is_normal_reg(lhs) {
                if let Ok(reg) = Reg::from_str(lhs.name.as_str()) {
                    defs.insert(reg);
                }
            }
        }
        Statement::While { ref body, .. } => for stmt in body {
            defines_stmt(stmt, defs);
        },
        Statement::IfThenElse {
            ref then_clause,
            ref else_clause,
            ..
        } => {
            for stmt in then_clause {
                defines_stmt(stmt, defs);
            }
            for stmt in else_clause {
                defines_stmt(stmt, defs);
            }
        }
        _ => (),
    }
}

pub fn defines(i: &UseDefDefinesIn) -> Vec<UseDefDefinesOut> {
    let mut defs = BTreeSet::new();
    for stmt in i.bil {
        defines_stmt(stmt, &mut defs);
    }
    vec![UseDefDefinesOut {
        registers: defs.into_iter().collect(),
    }]
}
