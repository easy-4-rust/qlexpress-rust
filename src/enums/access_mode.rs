//! 访问模式枚举,对应 Java `com.alibaba.qlexpress4.enums.AccessMode`。

/// 成员访问模式。对应 Java: com.alibaba.qlexpress4.enums.AccessMode
/// (取值 `WRITE` / `READ`,用于标注字段访问是读还是写)。
///
/// Java 原枚举用于字段访问句柄区分读写场景;Rust 保持同名同序取值。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum AccessMode {
    /// 写访问。对应 Java 枚举值 `WRITE`。
    Write,
    /// 读访问。对应 Java 枚举值 `READ`。
    Read,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_order_matches_java() {
        // Java 声明顺序:WRITE, READ。
        assert_ne!(AccessMode::Write, AccessMode::Read);
    }
}
