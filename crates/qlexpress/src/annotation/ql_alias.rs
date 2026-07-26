//! `@QLAlias` 注解的 Rust 平移,对应 Java `com.alibaba.qlexpress4.annotation.QLAlias`。
//!
//! Java 原文:
//! ```java
//! @Inherited
//! @Target({TYPE, FIELD, METHOD})
//! @Retention(RetentionPolicy.RUNTIME)
//! public @interface QLAlias {
//!     /** @return aliases */
//!     String[] value();
//! }
//! ```
//!
//! Rust 无注解机制,按 SPEC 平移为「元数据结构体 + 注册参数」:
//! 宿主在注册本地类型/字段/方法时,通过本结构体显式声明别名(等价于
//! Java 注解上的 `value`),别名在 Express4Runner 注册阶段消费。

/// QLAlias 元数据。对应 Java: com.alibaba.qlexpress4.annotation.QLAlias
/// (标注在 TYPE / FIELD / METHOD 上,声明脚本可用的别名)。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct QLAlias {
    /// 别名列表。对应 Java 注解元素 `value()`(`String[]`)。
    value: Vec<String>,
}

impl QLAlias {
    /// 构造别名元数据。对应 Java 注解使用 `@QLAlias({"a", "b"})`。
    pub fn new(value: Vec<String>) -> Self {
        QLAlias { value }
    }

    /// 别名列表。对应 Java 注解方法 `value()`。
    pub fn value(&self) -> &[String] {
        &self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_aliases() {
        let alias = QLAlias::new(vec!["money".to_string(), "amount".to_string()]);
        assert_eq!(alias.value(), &["money", "amount"]);
    }
}
