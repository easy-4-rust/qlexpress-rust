//! Java 方法重载匹配优先级。

/// 参数类型匹配的优先级类别。
///
/// 对应 Java:
/// `com.alibaba.qlexpress4.runtime.MemberResolver.MatchPriority`。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchPriority {
    /// 不匹配。
    Mismatch,
    /// 扩展匹配，例如实参可赋给 Object。
    Extend,
    /// 数值窄化。
    NumberDemotion,
    /// 数值提升。
    NumberPromotion,
    /// 包装类型拆箱。
    Unbox,
    /// Lambda 适配函数式接口。
    Lambda,
    /// 类型完全相同。
    Equal,
}
