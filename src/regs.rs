#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Hash)]
pub enum Reg {
    RAX,
    RBX,
    RCX,
    RDX,
    RSP,
    RBP,
    RSI,
    RDI,
    R8,
    R9,
    R10,
    R11,
    R12,
    R13,
    R14,
    R15,
}
use self::Reg::*;

pub const ARGS: &'static [Reg] = &[RDI, RSI, RDX, RCX, R8, R9];
pub const CALLER_SAVED: &[Reg] = &[RAX, RCX, RDX, R8, R9, R10, R11];
pub const RET_REG: Reg = RAX;

impl Reg {
    pub fn from_str(s: &str) -> Option<Reg> {
        match s {
            "RAX" => Some(RAX),
            "RBX" => Some(RBX),
            "RCX" => Some(RCX),
            "RDX" => Some(RDX),
            "RSP" => Some(RSP),
            "RBP" => Some(RBP),
            "RSI" => Some(RSI),
            "RDI" => Some(RDI),
            "R8" => Some(R8),
            "R9" => Some(R9),
            "R10" => Some(R10),
            "R11" => Some(R11),
            "R12" => Some(R12),
            "R13" => Some(R13),
            "R14" => Some(R14),
            "R15" => Some(R15),
            _ => None,
        }
    }
}
