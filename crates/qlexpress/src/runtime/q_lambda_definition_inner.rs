//! 指令序列 Lambda 定义,对应 Java `com.alibaba.qlexpress4.runtime.QLambdaDefinitionInner`
//! (含其内部类 `Param`)。
//! 职责:以指令序列 + 参数声明 + 最大栈深描述一个 Lambda 的编译期形态。
//! 本文件由 `qlambda.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::rc::Rc;

use crate::ql_options::QLOptions;
use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::delegate_qcontext::DelegateQContext;
use crate::runtime::instruction::Instruction;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambda;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qlambda_inner::QLambdaInner;

pub use super::param::Param;

impl Param {
    /// 构造参数声明。对应 Java 构造器 `Param(name, clazz)`。
    pub fn new(name: impl Into<String>, clazz: Option<TargetType>) -> Self {
        Param {
            name: name.into(),
            clazz,
        }
    }

    /// 参数名。对应 Java 方法 `getName`。
    pub fn name(&self) -> &str {
        &self.name
    }

    /// 参数声明类型(`None` = Java `null`)。对应 Java 方法 `getClazz`。
    pub fn clazz(&self) -> Option<TargetType> {
        self.clazz
    }
}

/// 指令序列 Lambda 定义。对应 Java: com.alibaba.qlexpress4.runtime.QLambdaDefinitionInner
///
/// Lambda defined by an instruction sequence, mirroring Java
/// `QLambdaDefinitionInner`.
pub struct QLambdaDefinitionInner {
    /// Function name.
    name: String,
    instructions: Vec<Instruction>,
    params_type: Vec<Param>,
    max_stack_size: usize,
}

impl QLambdaDefinitionInner {
    /// 构造 Lambda 定义。对应 Java 构造器
    /// `QLambdaDefinitionInner(name, instructions, paramsType, maxStackSize)`。
    pub fn new(
        name: impl Into<String>,
        instructions: Vec<Instruction>,
        params_type: Vec<Param>,
        max_stack_size: usize,
    ) -> Self {
        QLambdaDefinitionInner {
            name: name.into(),
            instructions,
            params_type,
            max_stack_size,
        }
    }

    /// 获取指令序列。对应 Java 字段 `instructions` 的访问。
    pub fn instructions(&self) -> &[Instruction] {
        &self.instructions
    }

    /// 获取参数声明列表。对应 Java 字段 `paramsType` 的访问。
    pub fn params_type(&self) -> &[Param] {
        &self.params_type
    }

    /// 获取最大栈深。对应 Java 字段 `maxStackSize` 的访问。
    pub fn max_stack_size(&self) -> usize {
        self.max_stack_size
    }
}

impl QLambdaDefinition for QLambdaDefinitionInner {
    /// 向下转型支持(供 api/parsecache Exporter 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    /// 物化为 `QLambdaInner`。对应 Java 方法 `toLambda`:在新的
    /// `DelegateQContext` 中捕获 `qContext` 的*当前作用域*(这正是闭包与
    /// 递归自引用可用的原因——定义处作用域的函数表从 Lambda 体可达)。
    /// Java `toLambda`: captures the *current scope* of `qContext` in a new
    /// `DelegateQContext` (this is what makes closures and recursive
    /// self-references work — the function table of the defining scope is
    /// reachable from the lambda body).
    fn to_lambda(
        self: Rc<Self>,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        new_env: bool,
    ) -> Rc<QLambda> {
        Rc::new(QLambda::Inner(QLambdaInner::new(
            self,
            DelegateQContext::new(Rc::clone(q_context.q_runtime()), q_context.current_scope()),
            ql_options.clone(),
            new_env,
        )))
    }

    fn println(&self, depth: usize, debug: &mut dyn FnMut(String)) {
        for (i, instruction) in self.instructions.iter().enumerate() {
            instruction.println(i, depth, debug);
        }
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn compiled_instruction_count(&self) -> usize {
        self.instructions.iter().fold(0usize, |total, instruction| {
            total.saturating_add(instruction.compiled_instruction_count())
        })
    }
}
