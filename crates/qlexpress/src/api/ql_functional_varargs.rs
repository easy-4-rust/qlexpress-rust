//! 变参函数接口,对应 Java `com.alibaba.qlexpress4.api.QLFunctionalVarargs`。
//! 职责:脚本侧可变参数函数(Java 函数式接口)的 Rust 契约。

use crate::exception::QLException;
use crate::runtime::value::DataValue;

/// 变参函数契约。对应 Java: com.alibaba.qlexpress4.api.QLFunctionalVarargs
/// (`@FunctionalInterface`,`Object call(Object... params)`)。
///
/// Java 以 `Object...` 接收任意个数参数、以 `Object` 返回并以异常报错;
/// Rust 侧参数为 [`DataValue`] 切片,返回 `Result<DataValue, QLException>`
/// (Java 抛异常 ↔ Rust 返回 `Err`)。
pub trait QLFunctionalVarargs {
    /// 调用变参函数。对应 Java 方法 `call(Object... params)`。
    fn call(&self, params: &[DataValue]) -> Result<DataValue, QLException>;
}

/// 让闭包/函数指针直接充当变参函数(Java lambda 实现 `@FunctionalInterface`
/// 的等价物)。
impl<F> QLFunctionalVarargs for F
where
    F: Fn(&[DataValue]) -> Result<DataValue, QLException>,
{
    fn call(&self, params: &[DataValue]) -> Result<DataValue, QLException> {
        (self)(params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closure_as_varargs_function() {
        // Java: (params) -> params.length
        let f = |params: &[DataValue]| Ok(DataValue::Int(params.len() as i32));
        assert_eq!(
            QLFunctionalVarargs::call(&f, &[DataValue::Int(1), DataValue::Str("x".to_string())],)
                .ok(),
            Some(DataValue::Int(2))
        );
        assert_eq!(
            QLFunctionalVarargs::call(&f, &[]).ok(),
            Some(DataValue::Int(0))
        );
    }
}
