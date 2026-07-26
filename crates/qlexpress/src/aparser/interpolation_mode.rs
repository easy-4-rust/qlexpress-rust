/// `InterpolationMode` 枚举的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/InterpolationMode.java`；具体对象路径见 `docs/对象级对照表.md`。
/// How to manage string interpolation, e.g. `"a ${t-c} b"`, mirroring Java
/// `InterpolationMode`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum InterpolationMode {
    /// Implement interpolation using a QLExpress script. (Java default.)
    #[default]
    Script,
    /// Implement interpolation using a variable name in the context.
    Variable,
    /// Disable interpolation; `${xxx}` is rendered verbatim.
    Disable,
}
