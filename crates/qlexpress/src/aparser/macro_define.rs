//! Macro definition holder, mirroring Java `MacroDefine`.
//!
//! Java stores `List<QLInstruction>`; the Rust instruction type arrives in
//! Stage 3, so the type is generic over the instruction representation `I`
//! (Stage 3 will instantiate it with `runtime::instruction::Instruction`).

use std::cell::{Ref, RefCell, RefMut};
use std::rc::Rc;

/// 保存宏展开所需的预编译指令及其是否产生返回值。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/MacroDefine.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `MacroDefine`.
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.aparser.MacroDefine。
pub struct MacroDefine<I> {
    instructions: Option<Rc<RefCell<Vec<I>>>>,
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
            instructions: Some(Rc::new(RefCell::new(instructions))),
            last_stmt_express,
        }
    }

    /// 从可空共享指令列表创建对象，完整保留 Java 构造器的引用与 `null` 语义。
    /// 对应 Java: com.alibaba.qlexpress4.aparser.MacroDefine#MacroDefine。
    pub fn from_shared(instructions: Option<Rc<RefCell<Vec<I>>>>, last_stmt_express: bool) -> Self {
        MacroDefine {
            instructions,
            last_stmt_express,
        }
    }

    /// 返回宏展开使用的预编译指令序列。
    /// 无显式参数；返回 Java 可空 `List` 的共享只读借用。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/instruction/QLInstruction.java`，方法 `macroInstructions`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getMacroInstructions`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.MacroDefine#macroInstructions。
    pub fn macro_instructions(&self) -> Option<Ref<'_, Vec<I>>> {
        self.instructions
            .as_ref()
            .map(|instructions| instructions.borrow())
    }

    /// 返回 Java getter 所暴露实时列表的可变借用。
    /// 对应 Java: com.alibaba.qlexpress4.aparser.MacroDefine#getMacroInstructions。
    pub fn macro_instructions_mut(&self) -> Option<RefMut<'_, Vec<I>>> {
        self.instructions
            .as_ref()
            .map(|instructions| instructions.borrow_mut())
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
        assert_eq!(
            define
                .macro_instructions()
                .expect("non-null instructions")
                .as_slice(),
            &["insn1", "insn2"]
        );
        assert!(define.is_last_stmt_express());
    }

    #[test]
    fn preserves_java_live_list_identity_and_nullability() {
        let shared = Rc::new(RefCell::new(vec!["first"]));
        let define = MacroDefine::from_shared(Some(Rc::clone(&shared)), false);
        shared.borrow_mut().push("external");
        define
            .macro_instructions_mut()
            .expect("non-null instructions")
            .push("getter");
        assert_eq!(shared.borrow().as_slice(), &["first", "external", "getter"]);
        assert!(!define.is_last_stmt_express());

        let null_define = MacroDefine::<&str>::from_shared(None, true);
        assert!(null_define.macro_instructions().is_none());
        assert!(null_define.macro_instructions_mut().is_none());
        assert!(null_define.is_last_stmt_express());
    }
}
