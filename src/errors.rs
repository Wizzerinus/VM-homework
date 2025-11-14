use crate::syntax::Bytecode;

#[derive(Debug)]
#[allow(unused)]
pub enum DecodeError {
    NotEnoughBytes(usize),
    UnknownBytecode(u8),
    NoNullTerminator,
    BadDesignation(u8),
    JumpEOF,
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
#[allow(unused)]
pub enum DisassemblyErrorImpl {
    BadInitSection,
    BadSymbolsSection { expected: usize, obtained: usize },
    BadStringsSection { expected: usize, obtained: usize },
}

#[derive(Debug)]
#[allow(unused)]
pub enum RuntimeError {
    BinopFailed(String),
    EmptyStack,
    FullStack,
    EmptyMultiStack { expected: usize, obtained: usize },
    LoadAddrBanned,
    StoreRefBanned,
}

#[derive(Debug)]
#[allow(clippy::enum_variant_names)]
#[allow(unused)]
pub enum LamaError {
    DecodeError {
        error: DecodeError,
        while_decoding_at: usize,
    },
    DisassemblyError {
        error: DisassemblyErrorImpl,
    },
    RuntimeError {
        error: RuntimeError,
        while_running: Bytecode,
    },
}
