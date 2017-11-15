use mycroft_macros::mycroft_program;
use bap::high::bitvector::BitVector;
use bap::high::bil::Statement;
use bap::basic::Arch;
use avar::AVar;
type Bytes = Vec<u8>;
type Sema = Vec<Statement>;
type Stack = Vec<(String, BitVector)>;
type Chop = Vec<BitVector>;

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
        stack: Stack,
        final_file: String,
        final_addr: BitVector,
        loc: AVar
    }
    use_after_free {
        initial_file: String,
        initial_addr: BitVector,
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

    ?get_disasms: lift {file_name: file_name, address: addr, disassembly: disasm}
    ?get_syms: sym {file_name: file_name, name: name}

    bap_dump_segments: segment {
        file_name: file_name,
        contents: seg_contents,
        start: start,
        end: end,
        read: read,
        write: write,
        execute: execute
    } <- file {name: file_name, contents: contents} + ::funcs::dump_segments
    objdump_dump_plt: link_pad {
        file_name: file_name,
        pad_name: pad_name,
        pad_addr: pad_addr
    } <- file {name: file_name, contents: contents} + ::funcs::dump_plt
    bap_dump_syms: sym {
        file_name: file_name,
        name: name,
        start: start,
        end: end
    } <- file {name: file_name, contents: contents} + ::funcs::dump_syms
    syms_live: live {file_name: file_name, addr: addr} <- sym {file_name: file_name, start: addr}
    bap_sema: lift {
        file_name: file_name,
        address: addr,
        bil: bil,
        disassembly: disasm,
        fallthrough: fall,
        is_call: call,
        is_ret: ret
    } <- live {file_name: file_name, addr: addr} & segment {file_name: file_name, contents: seg_contents, start: seg_start, end: seg_end} & prog_arch {file_name: file_name, arch: arch} + ::funcs::lift
    sema_succ: succ {file_name: file_name, src_addr: src_addr, dst_addr: dst_addr, is_call: is_call} <- lift {file_name: file_name, address: src_addr, bil: bil, fallthrough: fall, is_call: is_call} + ::funcs::sema_succ
    skip_computed_call: succ {file_name: file_name, src_addr: src_addr, dst_addr: fall_addr, is_call: ~false} <- lift {file_name: file_name, address: src_addr, fallthrough: fall_addr, is_call: ~true, bil: bil} + ::funcs::is_computed_jump
    live_succ_live: live { file_name: file_name, addr: dst_addr } <- succ {file_name: file_name, src_addr: src_addr, dst_addr: dst_addr} & live {file_name: file_name, addr: src_addr}
    live_call_live: live { file_name: file_name, addr: fall } <- lift { file_name: file_name, fallthrough: fall, is_call: ~true }
    bap_arch: prog_arch { file_name: file_name, arch: arch } <- file {name: file_name, contents: contents} + ::funcs::get_arch
    malloc_call_by_name: malloc_call {file_name: file_name, addr: addr} <- link_pad { file_name: file_name, pad_name: func_name, pad_addr: tgt_addr } & succ { file_name: file_name, src_addr: addr, dst_addr: tgt_addr, is_call: ~true } + ::funcs::is_malloc_name
    free_call_by_name: free_call {file_name: file_name, addr: addr} <- link_pad { file_name: file_name, pad_name: func_name, pad_addr: tgt_addr } & succ { file_name: file_name, src_addr: addr, dst_addr: tgt_addr, is_call: ~true } + ::funcs::is_free_name
    puts_uses: func_uses {file_name: name, addr: addr, var: ~(::avar::get_arg0())} <- link_pad {file_name: name, pad_name: ~("puts".to_string()), pad_addr: tgt} & succ {file_name: name, src_addr: addr, dst_addr: tgt }
    skip_dyn: skip_func { file_name: name, addr: addr } <- link_pad { file_name: name, pad_addr: tgt } & succ {file_name: name, src_addr: addr, dst_addr: tgt, is_call: ~true}

    succ_over_normal: succ_over { file_name: name, src: src, dst: dst } <- succ { file_name: name, src_addr: src, dst_addr: dst, is_call: ~false }
    succ_over_skip_call: succ_over { file_name: name, src: src, dst: dst } <- succ { file_name: name, src_addr: src, dst_addr: dst, is_call: ~true } & lift { file_name: name, address: src, fallthrough: dst }
    call_site_internal: call_site { call_file: name, call_addr: src_addr, dst_file: name, dst_addr: dst_addr } <- succ { file_name: name, src_addr: src_addr, dst_addr: dst_addr, is_call: ~true }
    call_site_dyn: call_site { call_file: src_name, call_addr: src_addr, dst_file: dst_name, dst_addr: dst_addr } <- succ { file_name: name, src_addr: src_addr, dst_addr: pad_addr, is_call: ~true } & link_pad {file_name: src_name, pad_name: func_name, pad_addr: pad} & sym { file_name: dst_name, name: func_name, start: dst_addr }
    func_start: func {file_name: name, entry: addr, addr: addr } <- sym { file_name: name, start: addr }
    func_walk_over: func {file_name: name, entry: entry, addr: next } <- func {file_name: name, entry: entry, addr: addr } & succ_over {file_name: name, src: addr, dst: next}
"#
);
pub use self::mycroft_program::*;
