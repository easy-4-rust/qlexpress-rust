//! Java import 声明的作用域类别。

/// 包、内部类、普通类或别名导入。
///
/// 对应 Java: `com.alibaba.qlexpress4.aparser.ImportManager.ImportScope`。
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImportScope {
    /// `import java.lang.*;`
    Pack,
    /// `import a.b.Cls.*;`
    InnerCls,
    /// `import java.lang.String;`
    Cls,
    /// `import java.lang.String as Str;`
    Alias,
}
