use std::{ffi::CString, fmt::Display};

use crate::{
    errors::{DecodeError, LamaError},
    utils::{CStringFormattable, decode_cstring},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(unused)]
pub(crate) enum Designation {
    Global(usize),
    Local(usize),
    Argument(usize),
    Access(usize),
}

impl Display for Designation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (sym, n) = match self {
            Designation::Global(n) => ('G', n),
            Designation::Local(n) => ('L', n),
            Designation::Argument(n) => ('A', n),
            Designation::Access(n) => ('C', n),
        };
        f.write_fmt(format_args!("{}({})", sym, n))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Condition {
    Zero,
    NonZero,
}

impl Display for Condition {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Condition::Zero => f.write_str("z "),
            Condition::NonZero => f.write_str("nz"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(unused)]
pub(crate) enum Call {
    Read,
    Write,
    Length,
    String,
    MkArray(usize),
    Function(Target, usize),
}

impl Display for Call {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Call::Read => f.write_str("Lread"),
            Call::Write => f.write_str("Lwrite"),
            Call::Length => f.write_str("Llen"),
            Call::String => f.write_str("Lstr"),
            Call::MkArray(n) => f.write_fmt(format_args!("Barray {}", n)),
            Call::Function(tgt, cnt) => f.write_fmt(format_args!("{:#06x} {}", tgt, cnt)),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Pattern {
    TopStringsEqual,
    IsArray,
    IsString,
    IsSExpression,
    IsInt,
    IsNotInt,
    IsClosure,
}

impl Display for Pattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Pattern::TopStringsEqual => "%",
            Pattern::IsArray => "#array",
            Pattern::IsString => "#string",
            Pattern::IsSExpression => "#sexp",
            Pattern::IsInt => "unboxed",
            Pattern::IsNotInt => "boxed",
            Pattern::IsClosure => "#closure",
        })
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Binop {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Lte,
    Gt,
    Gte,
    Eq,
    Neq,
    And,
    Or,
}

impl Display for Binop {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let sym = match self {
            Binop::Add => "+",
            Binop::Sub => "-",
            Binop::Mul => "*",
            Binop::Div => "/",
            Binop::Mod => "%",
            Binop::Lt => "<",
            Binop::Lte => "<=",
            Binop::Gt => ">",
            Binop::Gte => ">=",
            Binop::Eq => "==",
            Binop::Neq => "!=",
            Binop::And => "&&",
            Binop::Or => "!!",
        };
        f.write_str(sym)
    }
}

// Temporary while I figure out how jumps work.
type Target = usize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[allow(unused)]
pub(crate) enum Bytecode {
    Binop(Binop),
    IntConst(i32),
    StringConst(CString),
    SExpression(CString, usize),
    Load(Designation),
    LoadAddr(Designation),
    Store(Designation),
    StoreReference,
    StoreComposite,
    GetElement,
    Jump(Target),
    CondJump(Condition, Target),
    // Begin has two bytecodes - 0x52 for no closure
    // and 0x53 for closure
    Begin {
        arg_count: usize,
        local_count: usize,
        has_closure: bool,
    },
    End,
    Closure {
        name: Target,
        designations: Vec<Designation>,
    },
    CallClosure(usize),
    CallFunc(Call),
    Return,
    Drop,
    Duplicate,
    SwapTop,
    Tag(CString, usize),
    Array(usize),
    CheckPattern(Pattern),
    MatchCaseFailure {
        line: usize,
        column: usize,
    },
    Lineno(usize),
    // Public & Import not supported
}

impl Display for Bytecode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Bytecode::Binop(binop) => f.write_fmt(format_args!("BINOP   {}", binop)),
            Bytecode::IntConst(n) => f.write_fmt(format_args!("CONST   {}", n)),
            Bytecode::StringConst(cstring) => {
                f.write_fmt(format_args!("STRING  {}", CStringFormattable(cstring)))
            }
            Bytecode::SExpression(cstring, n) => f.write_fmt(format_args!(
                "SEXP    {} {}",
                CStringFormattable(cstring),
                n
            )),
            Bytecode::Load(designation) => f.write_fmt(format_args!("LD      {}", designation)),
            Bytecode::LoadAddr(designation) => f.write_fmt(format_args!("LDA     {}", designation)),
            Bytecode::Store(designation) => f.write_fmt(format_args!("ST      {}", designation)),
            Bytecode::StoreReference => f.write_str("STI"),
            Bytecode::StoreComposite => f.write_str("STC"),
            Bytecode::GetElement => f.write_str("ELEM"),
            Bytecode::Jump(t) => f.write_fmt(format_args!("JMP     {:#06x}", t)),
            Bytecode::CondJump(condition, t) => {
                f.write_fmt(format_args!("CJMP{}  {:#06x}", condition, t))
            }
            Bytecode::Begin {
                arg_count,
                local_count,
                has_closure: true,
            } => f.write_fmt(format_args!("CBEGIN  {} {}", arg_count, local_count)),
            Bytecode::Begin {
                arg_count,
                local_count,
                has_closure: false,
            } => f.write_fmt(format_args!("BEGIN   {} {}", arg_count, local_count)),
            Bytecode::End => f.write_str("END"),
            Bytecode::Closure {
                name,
                designations: _,
            } => f.write_fmt(format_args!("CLOSURE {:#06x}", name)),
            Bytecode::CallClosure(n) => f.write_fmt(format_args!("CALLC   {}", n)),
            Bytecode::CallFunc(call) => f.write_fmt(format_args!("CALL    {}", call)),
            Bytecode::Return => f.write_str("RET"),
            Bytecode::Drop => f.write_str("DROP"),
            Bytecode::Duplicate => f.write_str("DUP"),
            Bytecode::SwapTop => f.write_str("SWAP"),
            Bytecode::Tag(cstring, n) => f.write_fmt(format_args!(
                "TAG     {} {}",
                CStringFormattable(cstring),
                n
            )),
            Bytecode::Array(n) => f.write_fmt(format_args!("ARRAY   {}", n)),
            Bytecode::CheckPattern(pattern) => f.write_fmt(format_args!("PATT    {}", pattern)),
            Bytecode::MatchCaseFailure { line, column } => {
                f.write_fmt(format_args!("FAIL    {} {}", line, column))
            }
            Bytecode::Lineno(n) => f.write_fmt(format_args!("LINE    {}", n)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Code<'a> {
    slice: &'a [u8],
    pos: usize,
    strings: &'a [u8],
}

impl<'a> Code<'a> {
    pub fn new(slice: &'a [u8], strings: &'a [u8]) -> Self {
        Self {
            slice,
            pos: 0,
            strings,
        }
    }

    pub fn restart(&mut self) {
        self.pos = 0;
    }

    fn err(&self, error: DecodeError) -> LamaError {
        LamaError::DecodeError {
            error,
            while_decoding_at: self.pos,
        }
    }

    fn advance<const N: usize>(&mut self) -> Result<&'a [u8; N], LamaError> {
        if self.slice.len() < self.pos + N {
            return Err(self.err(DecodeError::NotEnoughBytes(N)));
        }

        let out: &'a [u8; N] = self.slice[self.pos..self.pos + N].try_into().unwrap();
        self.pos += N;
        Ok(out)
    }

    fn get_byte(&mut self) -> Result<u8, LamaError> {
        let xs = self.advance::<1>()?;
        Ok(xs[0])
    }

    fn get_i32(&mut self) -> Result<i32, LamaError> {
        let xs = self.advance::<4>()?;
        Ok(i32::from_ne_bytes(*xs))
    }

    fn get_usize(&mut self) -> Result<usize, LamaError> {
        let xs = self.advance::<4>()?;
        Ok(u32::from_ne_bytes(*xs) as usize)
    }

    fn get_target(&mut self) -> Result<Target, LamaError> {
        self.get_usize()
    }

    /// Если `typ = None`, то нужно достать тип переменной из байткода. (0 = global, 1 = local, 2 = arg, 3 = access)
    ///
    /// Если `typ = Some(x)`, то тип переменной уже был достан.
    fn get_designation(&mut self, m_typ: Option<u8>) -> Result<Designation, LamaError> {
        let typ = match m_typ {
            Some(x) => x,
            None => self.get_byte()?,
        };
        Ok(match typ {
            0 => Designation::Global(self.get_usize()?),
            1 => Designation::Local(self.get_usize()?),
            2 => Designation::Argument(self.get_usize()?),
            3 => Designation::Access(self.get_usize()?),
            x => return Err(self.err(DecodeError::BadDesignation(x))),
        })
    }

    fn get_string(&mut self) -> Result<CString, LamaError> {
        let idx = self.get_usize()?;
        decode_cstring(self.strings, idx).ok_or_else(|| self.err(DecodeError::NoNullTerminator))
    }

    pub(crate) fn decode_at(&mut self, pos: usize) -> Result<Bytecode, LamaError> {
        let old_idx = self.pos;
        self.pos = pos;
        let out = self.decode();
        self.pos = old_idx;
        out.and_then(|x| match x {
            Some(y) => Ok(y),
            None => Err(self.err(DecodeError::JumpEOF)),
        })
    }

    pub(crate) fn decode(&mut self) -> Result<Option<Bytecode>, LamaError> {
        if self.slice.len() == self.pos {
            return Ok(None);
        } else if self.slice.len() < self.pos {
            return Err(self.err(DecodeError::JumpEOF));
        }
        let x = self.get_byte()?;
        Ok(Some(match x {
            0xff => return Ok(None), // EOF
            // binops
            0x01 => Bytecode::Binop(Binop::Add),
            0x02 => Bytecode::Binop(Binop::Sub),
            0x03 => Bytecode::Binop(Binop::Mul),
            0x04 => Bytecode::Binop(Binop::Div),
            0x05 => Bytecode::Binop(Binop::Mod),
            0x06 => Bytecode::Binop(Binop::Lt),
            0x07 => Bytecode::Binop(Binop::Lte),
            0x08 => Bytecode::Binop(Binop::Gt),
            0x09 => Bytecode::Binop(Binop::Gte),
            0x0A => Bytecode::Binop(Binop::Eq),
            0x0B => Bytecode::Binop(Binop::Neq),
            0x0C => Bytecode::Binop(Binop::And),
            0x0D => Bytecode::Binop(Binop::Or),
            // Stack control
            0x10 => Bytecode::IntConst(self.get_i32()?),
            0x11 => Bytecode::StringConst(self.get_string()?),
            0x12 => Bytecode::SExpression(self.get_string()?, self.get_usize()?),
            0x13 => Bytecode::StoreReference,
            0x14 => Bytecode::StoreComposite,
            0x18 => Bytecode::Drop,
            0x19 => Bytecode::Duplicate,
            0x1A => Bytecode::SwapTop,
            0x1B => Bytecode::GetElement,
            0x20..=0x23 => Bytecode::Load(self.get_designation(Some(x & 0x03))?),
            0x30..=0x33 => Bytecode::LoadAddr(self.get_designation(Some(x & 0x03))?),
            0x40..=0x43 => Bytecode::Store(self.get_designation(Some(x & 0x03))?),
            // rip control
            0x15 => Bytecode::Jump(self.get_target()?),
            0x50 => Bytecode::CondJump(Condition::Zero, self.get_target()?),
            0x51 => Bytecode::CondJump(Condition::NonZero, self.get_target()?),
            0x17 => Bytecode::Return,
            0x70 => Bytecode::CallFunc(Call::Read),
            0x71 => Bytecode::CallFunc(Call::Write),
            0x72 => Bytecode::CallFunc(Call::Length),
            0x73 => Bytecode::CallFunc(Call::String),
            0x74 => Bytecode::CallFunc(Call::MkArray(self.get_usize()?)),
            // Making functions
            0x16 => Bytecode::End,
            0x52 => Bytecode::Begin {
                arg_count: self.get_usize()?,
                local_count: self.get_usize()?,
                has_closure: false,
            },
            0x53 => Bytecode::Begin {
                arg_count: self.get_usize()?,
                local_count: self.get_usize()?,
                has_closure: true,
            },
            0x54 => {
                let target = self.get_target()?;
                let des_count = self.get_usize()?;
                let mut designs = Vec::new();
                for _ in 0..des_count {
                    designs.push(self.get_designation(None)?);
                }
                Bytecode::Closure {
                    name: target,
                    designations: designs,
                }
            }
            0x55 => Bytecode::CallClosure(self.get_usize()?),
            0x56 => Bytecode::CallFunc(Call::Function(self.get_target()?, self.get_usize()?)),
            // Pattern matching
            0x57 => Bytecode::Tag(self.get_string()?, self.get_usize()?),
            0x58 => Bytecode::Array(self.get_usize()?),
            0x59 => Bytecode::MatchCaseFailure {
                line: self.get_usize()?,
                column: self.get_usize()?,
            },
            0x5A => Bytecode::Lineno(self.get_usize()?),
            0x60 => Bytecode::CheckPattern(Pattern::TopStringsEqual),
            0x61 => Bytecode::CheckPattern(Pattern::IsString),
            0x62 => Bytecode::CheckPattern(Pattern::IsArray),
            0x63 => Bytecode::CheckPattern(Pattern::IsSExpression),
            0x64 => Bytecode::CheckPattern(Pattern::IsNotInt),
            0x65 => Bytecode::CheckPattern(Pattern::IsInt),
            0x66 => Bytecode::CheckPattern(Pattern::IsClosure),

            // Error
            x => return Err(self.err(DecodeError::UnknownBytecode(x))),
        }))
    }
}
