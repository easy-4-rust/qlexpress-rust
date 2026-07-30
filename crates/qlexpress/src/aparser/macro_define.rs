//! Macro definition holder, mirroring Java `MacroDefine`.
//!
//! Java stores `List<QLInstruction>`; the Rust instruction type arrives in
//! Stage 3, so the type is generic over the instruction representation `I`
//! (Stage 3 will instantiate it with `runtime::instruction::Instruction`).

/// `MacroDefine` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/MacroDefine.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `MacroDefine`.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.aparser.MacroDefine。
pub struct MacroDefine<I> {
    instructions: Vec<I>,
    last_stmt_express: bool,
}

impl<I> MacroDefine<I> {
    /// 创建对象实例。
    /// 参数：`instructions`、`last_stmt_express`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/MacroDefine.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new MacroDefine(instructions, lastStmtExpress)`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.MacroDefine#new。
    pub fn new(instructions: Vec<I>, last_stmt_express: bool) -> Self {
        MacroDefine {
            instructions,
            last_stmt_express,
        }
    }

    /// 处理 macro instructions 对应的领域职责。
    /// 无显式参数；返回：`&[I]`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/instruction/QLInstruction.java`，方法 `macroInstructions`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getMacroInstructions`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.MacroDefine#macroInstructions。
    pub fn macro_instructions(&self) -> &[I] {
        &self.instructions
    }

    /// 判断 last stmt express 条件。
    /// 无显式参数；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/MacroDefine.java`，方法 `isLastStmtExpress`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `isLastStmtExpress`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.MacroDefine#isLastStmtExpress。
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
