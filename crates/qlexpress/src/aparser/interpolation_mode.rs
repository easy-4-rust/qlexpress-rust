/// `InterpolationMode` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/InterpolationMode.java`；具体对象路径见 `docs/对象级对照表.md`。
/// How to manage string interpolation, e.g. `"a ${t-c} b"`, mirroring Java
/// `InterpolationMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
/// 对应 Java: com.alibaba.qlexpress4.aparser.InterpolationMode。
pub enum InterpolationMode {
    /// Implement interpolation using a QLExpress script. (Java default.)
    #[default]
    Script = 0,
    /// Implement interpolation using a variable name in the context.
    Variable = 1,
    /// Disable interpolation; `${xxx}` is rendered verbatim.
    Disable = 2,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOURCE_PARITY: Java 声明顺序 `SCRIPT, VARIABLE, DISABLE` 决定
    /// `Enum#ordinal()`，默认初始化选用 SCRIPT。
    #[test]
    fn variants_and_default_match_java_declaration_order() {
        assert_eq!(InterpolationMode::Script as u8, 0);
        assert_eq!(InterpolationMode::Variable as u8, 1);
        assert_eq!(InterpolationMode::Disable as u8, 2);
        assert_eq!(InterpolationMode::default(), InterpolationMode::Script);
    }
}
