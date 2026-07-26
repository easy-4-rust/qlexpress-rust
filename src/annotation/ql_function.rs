//! `@QLFunction` 注解的 Rust 平移,对应 Java `com.alibaba.qlexpress4.annotation.QLFunction`。
//!
//! Java 原文:
//! ```java
//! @Inherited
//! @Target({METHOD})
//! @Retention(RetentionPolicy.RUNTIME)
//! public @interface QLFunction {
//!     /** @return function names */
//!     String[] value();
//! }
//! ```
//!
//! Rust 无注解机制,按 SPEC 平移为「元数据结构体 + 注册参数」:
//! 宿主在注册本地方法为脚本函数时,通过本结构体显式声明函数名列表
//! (等价于 Java 注解上的 `value`),在 Express4Runner 的
//! `addFunctionOfClassMethod` 等注册路径中消费。

/// QLFunction 元数据。对应 Java: com.alibaba.qlexpress4.annotation.QLFunction
/// (标注在 METHOD 上,声明脚本可调用的函数名)。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QLFunction {
    /// 函数名列表。对应 Java 注解元素 `value()`(`String[]`)。
    value: Vec<String>,
}

impl QLFunction {
    /// 构造函数元数据。对应 Java 注解使用 `@QLFunction({"f1", "f2"})`。
    pub fn new(value: Vec<String>) -> Self {
        QLFunction { value }
    }

    /// 函数名列表。对应 Java 注解方法 `value()`。
    pub fn value(&self) -> &[String] {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_function_names() {
        let meta = QLFunction::new(vec!["max".to_string()]);
        assert_eq!(meta.value(), &["max"]);
    }
}
