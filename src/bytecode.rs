use std::pin::Pin;

use crate::{
    errors::DisassemblyErrorImpl,
    syntax::{Code, Fixnum},
};

#[allow(unused)]
#[derive(Debug)]
struct PubSymbol {
    name_offset: usize,
    bytecode_offset: usize,
}

#[derive(Debug)]
#[allow(unused)]
pub struct Bytefile<'a> {
    pub_symbols: Vec<PubSymbol>,
    strings: &'a [u8],
    bytecode: Code<'a>,
    globals: Pin<Box<[Fixnum]>>,
}

impl<'a> TryFrom<&'a [u8]> for Bytefile<'a> {
    type Error = DisassemblyErrorImpl;

    fn try_from(value: &'a [u8]) -> Result<Self, Self::Error> {
        let string_pool_size = u32::from_le_bytes(
            value
                .get(0..4)
                .ok_or(DisassemblyErrorImpl::BadInitSection)?
                .try_into()
                .unwrap(),
        ) as usize;
        let glob_size = u32::from_le_bytes(
            value
                .get(4..8)
                .ok_or(DisassemblyErrorImpl::BadInitSection)?
                .try_into()
                .unwrap(),
        ) as usize;
        let symbols_num = u32::from_le_bytes(
            value
                .get(8..12)
                .ok_or(DisassemblyErrorImpl::BadInitSection)?
                .try_into()
                .unwrap(),
        ) as usize;
        let mut pub_symbols = Vec::default();
        for i in 0..symbols_num {
            let sym = value.get(12 + 8 * i..20 + 8 * i).ok_or(
                DisassemblyErrorImpl::BadSymbolsSection {
                    expected: symbols_num,
                    obtained: i,
                },
            )?;
            pub_symbols.push(PubSymbol {
                name_offset: u32::from_le_bytes(sym[0..4].try_into().unwrap()) as usize,
                bytecode_offset: u32::from_le_bytes(sym[4..8].try_into().unwrap()) as usize,
            });
        }
        let strings = value
            .get(12 + 8 * symbols_num..12 + 8 * symbols_num + string_pool_size)
            .ok_or_else(|| DisassemblyErrorImpl::BadStringsSection {
                expected: string_pool_size,
                obtained: value.len() - 12 - 8 * symbols_num,
            })?;
        Ok(Self {
            pub_symbols,
            strings,
            bytecode: Code::new(&value[12 + 8 * symbols_num + string_pool_size..]),
            globals: Pin::new(vec![Fixnum::default(); glob_size].into_boxed_slice()),
        })
    }
}

impl<'a> Bytefile<'a> {
    pub fn bytecode(&mut self) -> &mut Code<'a> {
        &mut self.bytecode
    }
}
