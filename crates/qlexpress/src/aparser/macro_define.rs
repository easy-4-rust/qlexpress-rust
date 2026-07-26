//! Macro definition holder, mirroring Java `MacroDefine`.
//!
//! Java stores `List<QLInstruction>`; the Rust instruction type arrives in
//! Stage 3, so the type is generic over the instruction representation `I`
//! (Stage 3 will instantiate it with `runtime::instruction::Instruction`).

/// Java `MacroDefine`.
#[derive(Clone, Debug)]
pub struct MacroDefine<I> {
    instructions: Vec<I>,
    last_stmt_express: bool,
}

impl<I> MacroDefine<I> {
    /// Java `new MacroDefine(instructions, lastStmtExpress)`.
    pub fn new(instructions: Vec<I>, last_stmt_express: bool) -> Self {
        MacroDefine {
            instructions,
            last_stmt_express,
        }
    }

    /// Java `getMacroInstructions`.
    pub fn macro_instructions(&self) -> &[I] {
        &self.instructions
    }

    /// Java `isLastStmtExpress`.
    pub fn is_last_stmt_express(&self) -> bool {
        self.last_stmt_express
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stores_instructions_and_flag() {
        let define = MacroDefine::new(vec!["insn1", "insn2"], true);
        assert_eq!(define.macro_instructions(), &["insn1", "insn2"]);
        assert!(define.is_last_stmt_express());
    }
}
