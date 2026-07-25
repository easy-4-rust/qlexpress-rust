//! 可调用脚本 Lambda 值,对应 Java `com.alibaba.qlexpress4.runtime.QLambda`。
//! 职责:脚本 Lambda 的统一调用契约(`call` / `getFunctionDefined`)。
//! Java 为接口 + 实现类(`QLambdaEmpty`、`QLambdaInner`、方法引用 Lambda
//! `QLambdaMethod`);Rust 以枚举变体聚合这些实现类,变体语义与原类一一对应。
//! 本文件由 `qlambda.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::data::lambda::QLambdaMethod;
use crate::runtime::function::CustomFunction;
use crate::runtime::q_result::QResult;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qlambda_empty::QLambdaEmpty;
use crate::runtime::qlambda_inner::QLambdaInner;
use crate::runtime::value::DataValue;

/// 可调用脚本 Lambda 值。对应 Java: com.alibaba.qlexpress4.runtime.QLambda(接口;
/// Rust 以枚举变体聚合其实现类 `QLambdaEmpty`/`QLambdaInner`/`QLambdaMethod`)
///
/// A callable script lambda value, mirroring the Java `QLambda` interface
/// and its implementations (`QLambdaEmpty`, `QLambdaInner`, and the
/// method-reference lambda `QLambdaMethod`).
pub enum QLambda {
    /// 空 Lambda:调用返回 `QResult.NEXT_INSTRUCTION`。
    /// 对应 Java `QLambdaEmpty.INSTANCE`(负载为其 Rust 对应类型 [`QLambdaEmpty`])。
    Empty(QLambdaEmpty),
    /// 指令序列 Lambda。对应 Java `QLambdaInner`。
    Inner(QLambdaInner),
    /// 对象方法作为 Lambda。对应 Java `data/lambda/QLambdaMethod`。
    Method(QLambdaMethod),
}

impl QLambda {
    /// 调用 Lambda。对应 Java 方法 `QLambda.call(Object... params)`。
    /// Java `QLambda.call(Object... params)`.
    pub fn call(&self, params: &[DataValue]) -> Result<QResult, QLException> {
        match self {
            QLambda::Empty(_) => Ok(QResult::NEXT_INSTRUCTION),
            QLambda::Inner(inner) => inner.call(params),
            QLambda::Method(method) => method.call(params),
        }
    }

    /// 调用 Lambda 并返回其内部定义的函数表。对应 Java 方法
    /// `QLambda.getFunctionDefined(Object... params)`。
    /// Java `QLambda.getFunctionDefined(Object... params)`.
    pub fn function_defined(
        &self,
        params: &[DataValue],
    ) -> Result<HashMap<String, Rc<dyn CustomFunction>>, QLException> {
        match self {
            QLambda::Inner(inner) => inner.function_defined(params),
            _ => Ok(HashMap::new()),
        }
    }
}

impl fmt::Debug for QLambda {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QLambda::Empty(_) => write!(f, "QLambdaEmpty"),
            QLambda::Inner(inner) => f
                .debug_struct("QLambdaInner")
                .field("name", &inner.lambda_definition.name())
                .field("params", &inner.lambda_definition.params_type())
                .field("new_env", &inner.new_env)
                .finish(),
            QLambda::Method(method) => write!(f, "QLambdaMethod({})", method.method_name()),
        }
    }
}
