use bap::high::bil::{Statement, Type, Variable};
use datalog::*;
use load::Loc;
use points_to::PointsTo;
use regs::{Reg, ARGS, RET_REG};
use std::collections::{BTreeMap, BTreeSet};
use std::str::FromStr;
use var::Var;

// Maps a register at a code address to the list of possible definition sites (for a specific
// location)
pub type DefChain = BTreeMap<Reg, Vec<Loc>>;

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
        use self::KillSpec::*;
        use var::Var::*;
        match (self, v) {
            (&Registers(ref regs), &Register { ref register, .. }) => regs.contains(register),
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
        ks: KillSpec::Registers(i.registers.clone()),
    }]
}

pub fn stack_wipe(i: &UseDefStackWipeIn) -> Vec<UseDefStackWipeOut> {
    vec![UseDefStackWipeOut {
        ks: KillSpec::StackFrame(*i.base),
    }]
}

pub fn only_args(i: &UseDefOnlyArgsIn) -> Vec<UseDefOnlyArgsOut> {
    if !ARGS.contains(&i.register) {
        Vec::new()
    } else {
        vec![UseDefOnlyArgsOut {}]
    }
}

pub fn only_ret(i: &UseDefOnlyRetIn) -> Vec<UseDefOnlyRetOut> {
    if i.register == &RET_REG {
        vec![UseDefOnlyRetOut {}]
    } else {
        Vec::new()
    }
}

pub fn promote_def(i: &UseDefPromoteDefIn) -> Vec<UseDefPromoteDefOut> {
    vec![UseDefPromoteDefOut { defs: vec![*i.def] }]
}

pub fn expand_registers(i: &UseDefExpandRegistersIn) -> Vec<UseDefExpandRegistersOut> {
    i.registers
        .iter()
        .map(|register| UseDefExpandRegistersOut {
            register: *register,
        })
        .collect()
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

pub fn def_chain(i: &UseDefDefChainIn) -> Vec<UseDefDefChainOut> {
    let mut out = BTreeMap::new();
    out.insert(i.register.clone(), i.defs.to_vec());
    vec![UseDefDefChainOut { dc: out }]
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

pub fn exclude_registers(i: &UseDefExcludeRegistersIn) -> Vec<UseDefExcludeRegistersOut> {
    if i.prev_defines.contains(i.register) {
        Vec::new()
    } else {
        vec![UseDefExcludeRegistersOut {}]
    }
}
