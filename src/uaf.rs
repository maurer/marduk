use datalog::*;
use var::Var;

pub fn reads_vars(i: &UafReadsVarsIn) -> Vec<UafReadsVarsOut> {
    //TODO this is not well modularized
    ::constraints::generation::extract_var_use(i.bil, i.loc, i.base)
        .into_iter()
        .map(|v| UafReadsVarsOut { v })
        .collect()
}

pub fn use_vars(i: &UafUseVarsIn) -> Vec<UafUseVarsOut> {
    vec![UafUseVarsOut {
        v: Var::Register { register: *i.r },
    }]
}
