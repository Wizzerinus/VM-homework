use std::fmt::Display;

use crate::{
    errors::DisassemblyErrorImpl,
    syntax::Code,
    utils::{CStringFormattable, decode_cstring_pool},
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
    globals_count: usize,
}

impl<'a> Display for Bytefile<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{} public symbols\n", self.pub_symbols.len()))?;

        f.write_str("Strings:\n")?;
        match decode_cstring_pool(self.strings) {
            None => f.write_str("\t(unable to decode)")?,
            Some(x) => {
                for (idx, y) in x {
                    f.write_fmt(format_args!("\t{:#06x}\t{}\n", idx, CStringFormattable(&y)))?;
                }
            }
        };

        f.write_str("Bytecode:\n")?;
        let mut bc_clone = self.bytecode.clone();
        bc_clone.restart();
        loop {
            match bc_clone.decode() {
                Ok(None) => break,
                Ok(Some(x)) => f.write_fmt(format_args!("\t{}\n", x))?,
                Err(e) => {
                    f.write_fmt(format_args!("\t(decoding error: {:?})", e))?;
                    break;
                }
            }
        }

        Ok(())
    }
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
            bytecode: Code::new(&value[12 + 8 * symbols_num + string_pool_size..], strings),
            globals_count: glob_size,
        })
    }
}

impl<'a> Bytefile<'a> {
    pub fn bytecode(&mut self) -> &mut Code<'a> {
        &mut self.bytecode
    }
}
