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

pub const ARGS: &[Reg] = &[RDI, RSI, RDX, RCX, R8, R9];
pub const CALLER_SAVED: &[Reg] = &[RAX, RCX, RDX, R8, R9, R10, R11];
pub const RET_REG: Reg = RAX;

pub fn caller_saved() -> Vec<Reg> {
    CALLER_SAVED.to_vec()
}

impl ::std::str::FromStr for Reg {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, ()> {
        match s {
            "RAX" => Ok(RAX),
            "RBX" => Ok(RBX),
            "RCX" => Ok(RCX),
            "RDX" => Ok(RDX),
            "RSP" => Ok(RSP),
            "RBP" => Ok(RBP),
            "RSI" => Ok(RSI),
            "RDI" => Ok(RDI),
            "R8" => Ok(R8),
            "R9" => Ok(R9),
            "R10" => Ok(R10),
            "R11" => Ok(R11),
            "R12" => Ok(R12),
            "R13" => Ok(R13),
            "R14" => Ok(R14),
            "R15" => Ok(R15),
            _ => Err(()),
        }
    }
}
