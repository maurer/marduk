use datalog::*;
use load::Stack;
pub fn strip_stack(i: &ContextStripStackIn) -> Vec<ContextStripStackOut> {
    if i.stacked.stack == Stack::NoStack {
        Vec::new()
    } else {
        let mut free = i.stacked.clone();
        free.stack = Stack::NoStack;
        vec![ContextStripStackOut { free }]
    }
}

pub fn add_stack(i: &ContextAddStackIn) -> Vec<ContextAddStackOut> {
    let mut stacked = i.free.clone();
    stacked.stack = Stack::EmptyStack;
    vec![ContextAddStackOut { stacked }]
}

pub fn flow_only_context(i: &ContextFlowOnlyContextIn) -> Vec<ContextFlowOnlyContextOut> {
    let stacked_use = i.use_.is_stacked();
    let stacked_free = i.free.is_stacked();
    //assert_eq!(stacked_use, stacked_free);
    if stacked_use != stacked_free {
        eprintln!("stack_mismatch: {} -> {}", i.free, i.use_);
    }
    if stacked_use {
        vec![ContextFlowOnlyContextOut {}]
    } else {
        Vec::new()
    }
}

pub fn stack_fallthrough(i: &ContextStackFallthroughIn) -> Vec<ContextStackFallthroughOut> {
    let mut fallthrough_stacked = i.fallthrough.clone();
    fallthrough_stacked.stack = i.stacked.stack.clone();
    vec![ContextStackFallthroughOut {
        fallthrough_stacked,
    }]
}
