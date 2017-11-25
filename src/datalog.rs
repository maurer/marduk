use mycroft_macros::mycroft_program;
use bap::high::bitvector::BitVector;
use bap::high::bil::Statement;
use bap::basic::Arch;
use avar::AVar;
type Bytes = Vec<u8>;
type Sema = Vec<Statement>;
type Stack = Vec<(String, BitVector)>;
type Chop = Vec<BitVector>;

fn new_stack() -> Stack {
    Vec::new()
}

fn new_chop() -> Chop {
    Vec::new()
}

const ZERO: usize = 0;
mycroft_program!(
    r#"
file {name: String, contents: Bytes}
segment {
    file_name: String,
    contents: Bytes,
    start: BitVector,
    end: BitVector,
    read: bool,
    write: bool,
    execute: bool
}
sym {
    file_name: String,
    name: String,
    start: BitVector,
    end: BitVector
}
lift {
    file_name: String,
    address: BitVector,
    bil: Sema,
    fallthrough: BitVector,
    disassembly: String,
    is_call: bool,
    is_ret: bool
}
succ_over {
    file_name: String,
    src: BitVector,
    dst: BitVector
}
live {
    file_name: String,
    addr: BitVector
}
prog_arch {
    file_name: String,
    arch: Arch
}
link_pad {
    file_name: String,
    pad_name: String,
    pad_addr: BitVector
}
path_alias {
    initial_file: String,
    initial_addr: BitVector,
    alias_set: u64,
    stack: Stack,
    chop: Chop,
    current_file: String,
    current_addr: BitVector,
    aliased_var: AVar,
    freed: bool
}
succ {
    file_name: String,
    src_addr: BitVector,
    dst_addr: BitVector,
    is_call: bool
}
free_call {
    file_name: String,
    addr: BitVector
}
malloc_call {
    file_name: String,
    addr: BitVector
}
path_alias_trace {
    initial_file: String,
    initial_addr: BitVector,
    alias_set: u64,
    stack: Stack,
    current_file: String,
    current_addr: BitVector,
    aliased_var: AVar,
    freed: bool,
    steps: usize
}
use_after_free_flow {
    initial_file: String,
    initial_addr: BitVector,
    alias_set: u64,
    stack: Stack,
    final_file: String,
    final_addr: BitVector,
    loc: AVar
}
use_after_free {
    initial_file: String,
    initial_addr: BitVector,
    alias_set: u64,
    stack: Stack,
    final_file: String,
    final_addr: BitVector,
    loc: AVar,
    steps: usize
}
func {
    file_name: String,
    entry: BitVector,
    addr: BitVector
}
call_site {
    call_file: String,
    call_addr: BitVector,
    dst_file: String,
    dst_addr: BitVector
}
skip_func {
    file_name: String,
    addr: BitVector
}
func_uses {
    file_name: String,
    addr: BitVector,
    var: AVar
}

?func: func {
    file_name: file,
    entry: entry,
    addr: addr
}
?call_site: call_site {
    call_file: call_file,
    call_addr: call_addr,
    dst_file,
    dst_addr
}
?succ: succ {
    file_name: name,
    src_addr: src,
    dst_addr: dst,
    is_call: call 
}
?get_disasms: lift {file_name: file_name, address: addr, disassembly: disasm}
?get_free_call: free_call {file_name: name, addr: addr}
?get_malloc_call: malloc_call {file_name: name, addr: addr}
?get_alias: path_alias {initial_file: file0, initial_addr: addr0, current_file: file, current_addr: addr, aliased_var: a_var, freed: freed, alias_set: alias_set}
?get_uaf_flow: use_after_free_flow {
    initial_file: name,
    initial_addr: addr,
    alias_set: alias
}
?link_pad: link_pad {file_name: name, pad_name: pad_name, pad_addr: addr}
?live: live {file_name: name, addr: addr}

bap_dump_segments: segment {
    file_name,
    contents: seg_contents,
    start,
    end,
    read,
    write,
    execute
} <- file {name: file_name, contents} + ::funcs::dump_segments
objdump_dump_plt: link_pad {
    file_name,
    pad_name,
    pad_addr
} <- file {name: file_name, contents} + ::funcs::dump_plt
bap_dump_syms: sym {
    file_name,
    name,
    start,
    end
} <- file {name: file_name, contents} + ::funcs::dump_syms
syms_live: live {file_name, addr} <- sym {file_name: file_name, start: addr}
bap_sema: lift {
    file_name: file_name,
    address: addr,
    bil: bil,
    disassembly: disasm,
    fallthrough: fall,
    is_call: call,
    is_ret: ret
} <- live {file_name: file_name, addr: addr} & segment {file_name: file_name, contents: seg_contents, start: seg_start, end: seg_end} & prog_arch {file_name: file_name, arch: arch} + ::funcs::lift
sema_succ: succ {file_name: file_name, src_addr: src_addr, dst_addr, is_call} <- lift {file_name: file_name, address: src_addr, bil: bil, fallthrough: fall, is_call} + ::funcs::sema_succ
skip_computed_call: succ {file_name: file_name, src_addr: src_addr, dst_addr: fall_addr, is_call: ~false} <- lift {file_name: file_name, address: src_addr, fallthrough: fall_addr, is_call: ~true, bil: bil} + ::funcs::is_computed_jump
live_succ_live: live { file_name: file_name, addr: dst_addr } <- succ {file_name: file_name, src_addr: src_addr, dst_addr} & live {file_name: file_name, addr: src_addr}
live_call_live: live { file_name: file_name, addr: fall } <- lift { file_name: file_name, fallthrough: fall, is_call: ~true }
bap_arch: prog_arch { file_name: file_name, arch: arch } <- file {name: file_name, contents: contents} + ::funcs::get_arch
malloc_call_by_name: malloc_call {file_name: file_name, addr: addr} <- link_pad { file_name: file_name, pad_name: func_name, pad_addr: tgt_addr } & succ { file_name: file_name, src_addr: addr, dst_addr: tgt_addr, is_call: ~true } + ::funcs::is_malloc_name
free_call_by_name: free_call {file_name: file_name, addr: addr} <- link_pad { file_name: file_name, pad_name: func_name, pad_addr: tgt_addr } & succ { file_name: file_name, src_addr: addr, dst_addr: tgt_addr, is_call: ~true } + ::funcs::is_free_name
puts_uses: func_uses {file_name: name, addr: addr, var: ~(::avar::get_arg0())} <- link_pad {file_name: name, pad_name: ~("puts".to_string()), pad_addr: tgt} & succ {file_name: name, src_addr: addr, dst_addr: tgt }
skip_dyn: skip_func { file_name: name, addr: addr } <- link_pad { file_name: name, pad_addr: tgt } & succ {file_name: name, src_addr: addr, dst_addr: tgt, is_call: ~true}

succ_over_normal: succ_over { file_name: name, src: src, dst } <- succ { file_name: name, src_addr: src, dst_addr: dst, is_call: ~false }
succ_over_skip_call: succ_over { file_name: name, src: src, dst } <- succ { file_name: name, src_addr: src, is_call: ~true } & lift { file_name: name, address: src, fallthrough: dst }
call_site_internal: call_site { call_file: name, call_addr: src_addr, dst_file: name, dst_addr } <- succ { file_name: name, src_addr: src_addr, dst_addr, is_call: ~true }
call_site_dyn: call_site { call_file: src_name, call_addr: src_addr, dst_file: dst_name, dst_addr } <- succ { file_name: name, src_addr: src_addr, dst_addr: pad_addr, is_call: ~true } & link_pad {file_name: src_name, pad_name: func_name, pad_addr: pad} & sym { file_name: dst_name, name: func_name, start: dst_addr }
func_start: func {file_name: name, entry: addr, addr: addr } <- sym { file_name: name, start: addr }
func_walk_over: func {file_name: name, entry: entry, addr: next } <- func {file_name: name, entry: entry, addr: addr } & succ_over {file_name: name, src: addr, dst: next}
flow_start_malloc: path_alias {initial_file: file, initial_addr: addr, alias_set: ~ZERO, stack: ~(new_stack()), chop: ~(new_chop()), current_file: file, current_addr: step, aliased_var: ~(::avar::get_ret()), freed: ~false} <- malloc_call {file_name: file, addr: addr} & lift {file_name: file, address: addr, fallthrough: step}

flow_free: path_alias {initial_file: file0, initial_addr: addr0, alias_set: alias_set, stack: stack, chop: chop, current_file: free_file, current_addr: next, aliased_var: a_var, freed: ~true} <- path_alias {initial_file: file0, initial_addr: addr0, alias_set: alias_set, stack: stack, chop: chop, current_file: free_file, current_addr: free_addr, aliased_var: ~(::avar::get_arg0())} & lift {file_name: free_file, address: free_addr, fallthrough: next} & free_call {file_name: free_file, addr: free_addr} & path_alias {initial_file: file0, initial_addr: addr0, alias_set: alias_set, stack: stack, chop: chop, current_file: free_file, current_addr: free_addr, aliased_var: a_var}

flow_free2_hack: path_alias {initial_file: file0, initial_addr: addr0, alias_set: alias_set, stack: stack, chop: chop, current_file: free_file, current_addr: next, aliased_var: a_var, freed: ~true} <- path_alias {initial_file: file0, initial_addr: addr0, alias_set: alias_set, stack: stack, chop: chop, current_file: free_file, current_addr: free_addr, aliased_var: ~(::avar::get_arg_n(1))} & lift {file_name: free_file, address: free_addr, fallthrough: next} & free_call {file_name: free_file, addr: free_addr} & path_alias {initial_file: file0, initial_addr: addr0, alias_set: alias_set, stack: stack, chop: chop, current_file: free_file, current_addr: free_addr, aliased_var: a_var}

flow_prop: path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: stack, chop: chop, current_file: file, current_addr: next, aliased_var: a_var2, freed: freed} <- path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: stack, chop: chop, current_file: file, current_addr: addr, aliased_var: a_var, freed: freed} & lift {file_name: file, address: addr, bil: bil} & succ {file_name: file, src_addr: addr, dst_addr: next, is_call: ~false} + ::funcs::xfer_taint
flow_skip_func: path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: stack, chop: chop, current_file: file, current_addr: next, aliased_var: a_var, freed: freed} <- path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: stack, chop: chop, current_file: file, current_addr: addr, aliased_var: a_var, freed: freed} & skip_func { file_name: file, addr: addr} & lift {file_name: file, address: addr, fallthrough: next} + ::funcs::clobbers
flow_call: path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: stack2, chop: chop2, current_file: file2, current_addr: addr2, aliased_var: a_var, freed: freed} <- path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: stack, chop: chop, current_file: file1, current_addr: addr1, aliased_var: a_var, freed: freed} & call_site {call_file: file1, call_addr: addr1, dst_file: file2, dst_addr: addr2} & lift {file_name: file1, address: addr1, fallthrough: ret_addr} + ::funcs::call_stack_chop
flow_ret: path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: stack2, chop: chop, current_file: file2, current_addr: addr2, aliased_var: a_var, freed: freed} <- path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: stack, chop: chop, current_file: file1, current_addr: addr1, aliased_var: a_var, freed: freed} & lift {file_name: file1, address: addr1, is_ret: ~true} + ::funcs::ret_stack
flow_ret_notarget: path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: ~(new_stack()), chop: chop2, current_file: call_file, current_addr: dst_addr, aliased_var: a_var, freed: freed} <- path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: ~(new_stack()), chop: chop, current_file: file1, current_addr: addr1, aliased_var: a_var, freed: freed} & lift {file_name: file1, address: addr1, is_ret: ~true} & lift {file_name: call_file, address: call_addr, fallthrough: dst_addr} & call_site {call_file: call_file, call_addr: call_addr, dst_file: file1, dst_addr: func_addr} & func {file_name: file1, entry: func_addr, addr: addr1} + ::funcs::ret_no_stack

flow_final: use_after_free_flow {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: stack, final_file: file1, final_addr: addr1, loc: a_var} <- path_alias {initial_file: file0, initial_addr: addr0, alias_set: a_s, stack: stack, chop: chop, current_file: file1, current_addr: addr1, aliased_var: a_var, freed: ~true} & lift {file_name: file_1, address: addr1, bil: bil} + ::funcs::flow_use
"#
);
//DEFERRED:
/*
*/

//flow_start_heap
//flow_free_2_hack
pub use self::mycroft_program::*;
