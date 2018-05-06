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
