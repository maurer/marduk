use datalog::*;
use regs::ARGS;
use var::Var;

pub fn free_args(i: &UafFreeArgsIn) -> Vec<UafFreeArgsOut> {
    i.args
        .iter()
        .cloned()
        .flat_map(|arg_n| {
            i.dc[&ARGS[arg_n]].iter().map(move |loc| UafFreeArgsOut {
                arg: Var::Register {
                    site: *loc,
                    register: ARGS[arg_n],
                },
            })
        })
        .collect()
}

pub fn expand_vars(i: &UafExpandVarsIn) -> Vec<UafExpandVarsOut> {
    i.vs.iter().map(|v| UafExpandVarsOut { v2: *v }).collect()
}

pub fn reads_vars(i: &UafReadsVarsIn) -> Vec<UafReadsVarsOut> {
    //TODO this is not well modularized
    ::constraints::generation::extract_var_use(i.bil, i.dc.clone(), i.loc, i.base)
        .into_iter()
        .map(|v| UafReadsVarsOut { v })
        .collect()
}

pub fn use_vars(i: &UafUseVarsIn) -> Vec<UafUseVarsOut> {
    i.dc[i.r]
        .iter()
        .map(|site| UafUseVarsOut {
            v: Var::Register {
                site: *site,
                register: *i.r,
            },
        })
        .collect()
}
