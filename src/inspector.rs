use std::collections::HashMap;

use crate::{
    bytecode::Bytefile,
    errors::LamaError,
    syntax::{Bytecode, Call},
};

struct Inspector<'a> {
    file: &'a mut Bytefile<'a>,
    result: HashMap<(Bytecode, Bytecode), u32>,
}

impl<'a> Inspector<'a> {
    fn new(file: &'a mut Bytefile<'a>) -> Self {
        Self {
            file,
            result: HashMap::new(),
        }
    }

    fn add(&mut self, prev: Bytecode, next: Bytecode) {
        self.result
            .entry((prev, next))
            .and_modify(|counter| *counter += 1)
            .or_insert(1);
    }

    fn inspect(mut self) -> Result<HashMap<(Bytecode, Bytecode), u32>, LamaError> {
        let mut last_bytecode: Option<Bytecode> = None;
        while let Some(x) = self.file.bytecode().decode()? {
            // eprintln!("Got {}", x);
            if let Some(ref y) = last_bytecode {
                self.add(y.clone(), x.clone());
            }

            self.update_jumps(&mut last_bytecode, x)?;
        }
        Ok(self.result)
    }

    fn update_jumps(
        &mut self,
        last_bytecode: &mut Option<Bytecode>,
        next_bytecode: Bytecode,
    ) -> Result<(), LamaError> {
        match next_bytecode {
            Bytecode::Jump(tgt) => {
                // We cannot go to the instruction after jump, but have to handle target
                self.handle_jump(&next_bytecode, tgt)?;
                *last_bytecode = None;
            }
            Bytecode::CondJump(_, tgt) => {
                // We can either go to the instruction after jump, or to the target
                self.handle_jump(&next_bytecode, tgt)?;
                *last_bytecode = Some(next_bytecode);
            }
            Bytecode::CallFunc(Call::Function(tgt, _)) => {
                // Any function ends in the End instruction
                // I do not know what to do with CALLC, but I don't think anything more interesting can be done
                self.handle_jump(&next_bytecode, tgt)?;
                *last_bytecode = Some(Bytecode::End);
            }
            _ => {
                // We are guaranteed to execute the next bytecode next
                *last_bytecode = Some(next_bytecode);
            }
        }
        Ok(())
    }

    fn handle_jump(&mut self, bytecode: &Bytecode, target: usize) -> Result<(), LamaError> {
        let instruction = self.file.bytecode().decode_at(target)?;
        self.add(bytecode.clone(), instruction);
        Ok(())
    }
}

pub fn inspect_bytecode(value: &[u8]) -> Result<HashMap<(Bytecode, Bytecode), u32>, LamaError> {
    let mut bytefile: Bytefile = value
        .try_into()
        .map_err(|k| LamaError::DisassemblyError { error: k })?;

    // eprintln!("{}", bytefile);
    let inspector = Inspector::new(&mut bytefile);
    inspector.inspect()
}
