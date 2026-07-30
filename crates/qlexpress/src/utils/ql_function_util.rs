//! QLFunction name helpers, mirroring Java `QLFunctionUtil`.
//!
//! Java inspects the `@QLFunction` annotation reflectively; in Rust the
//! function names declared for a native method are supplied explicitly at
//! registration time (SPEC §4), so this helper operates on an optional
//! name list.

/// 解析宿主方法声明的 QL 函数名。
/// 对应 Java: `com.alibaba.qlexpress4.utils.QLFunctionUtil`；Rust 由注册阶段显式传入注解元数据。
pub struct QLFunctionUtil;

impl QLFunctionUtil {
    /// 查询 ql function value。
    /// 参数：`ql_function_names`；返回：`Option<&[String]>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/annotation/QLFunction.java`，方法 `getQlFunctionValue`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getQLFunctionValue`: the names declared for the method.
    /// Returns `None` when the method carries no `QLFunction` names.
    pub fn get_ql_function_value(ql_function_names: Option<&[String]>) -> Option<&[String]> {
        ql_function_names
    }

    /// 判断 ql function for method 条件。
    /// 参数：`ql_function_names`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/annotation/QLFunction.java`，方法 `containsQlFunctionForMethod`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `containsQLFunctionForMethod`: whether any `QLFunction` names
    /// were declared for the method.
    /// 对应 Java: com.alibaba.qlexpress4.utils.QLFunctionUtil#containsQlFunctionForMethod。
    pub fn contains_ql_function_for_method(ql_function_names: Option<&[String]>) -> bool {
        ql_function_names.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_and_values() {
        let names = vec!["add".to_string(), "sum".to_string()];
        assert!(QLFunctionUtil::contains_ql_function_for_method(Some(
            &names
        )));
        assert_eq!(
            QLFunctionUtil::get_ql_function_value(Some(&names)),
            Some(&names[..])
        );
        assert!(!QLFunctionUtil::contains_ql_function_for_method(None));
        assert_eq!(QLFunctionUtil::get_ql_function_value(None), None);
    }
}
