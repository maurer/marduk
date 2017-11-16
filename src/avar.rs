use bap;
use bap::high::bitvector::BitVector;
use bap::high::bil::Variable;

#[derive(Debug, Clone, Hash, PartialOrd, PartialEq, Eq)]
pub struct AVar {
    pub inner: Variable,
    pub offset: Option<BitVector>,
}

impl AVar {
    pub fn not_temp(&self) -> bool {
        !self.inner.tmp
    }
    pub fn is_clobbered(&self) -> bool {
        match self.inner.name.as_str() {
            "RAX" | "RCX" | "RDX" | "R8" | "R9" | "R10" | "R11" => true,
            _ => false,
        }
    }
}

pub fn get_arg0() -> AVar {
    AVar {
        inner: Variable {
            name: "RDI".to_string(),
            type_: bap::high::bil::Type::Immediate(64),
            tmp: false,
            index: 0,
        },
        offset: None,
    }
}

pub fn get_arg_n(n: u8) -> AVar {
    let name = match n {
        0 => "RDI",
        1 => "RSI",
        2 => "RDX",
        3 => "RCX",
        4 => "R8",
        5 => "R9",
        _ => panic!("No implementation for argument {}", n),
    };
    AVar {
        inner: Variable {
            name: name.to_string(),
            type_: bap::high::bil::Type::Immediate(64),
            tmp: false,
            index: 0,
        },
        offset: None,
    }
}

pub fn get_ret() -> AVar {
    AVar {
        inner: Variable {
            name: "RAX".to_string(),
            type_: bap::high::bil::Type::Immediate(64),
            tmp: false,
            index: 0,
        },
        offset: None,
    }
}

impl ::std::fmt::Display for AVar {
    fn fmt(&self, f: &mut ::std::fmt::Formatter) -> ::std::result::Result<(), ::std::fmt::Error> {
        write!(f, "{}", self.inner)?;
        match self.offset {
            Some(ref off) => write!(f, "+{}", off),
            None => Ok(()),
        }
    }
}
