use datalog::*;
use regs::ARGS;
use var::Var;

pub fn free_args(i: &UafFreeArgsIn) -> Vec<UafFreeArgsOut> {
    let mut out = Vec::new();
    for arg_n in i.args {
        if let Some(defs) = i.dc.get(&ARGS[*arg_n]) {
            for def in defs {
                out.push(UafFreeArgsOut {
                    arg: Var::Register {
                        site: def.clone(),
                        register: ARGS[*arg_n],
                    },
                });
            }
        }
    }
    out
}

pub fn expand_vars(i: &UafExpandVarsIn) -> Vec<UafExpandVarsOut> {
    i.vs
        .iter()
        .map(|v| UafExpandVarsOut { v2: v.clone() })
        .collect()
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
                site: site.clone(),
                register: *i.r,
            },
        })
        .collect()
}
