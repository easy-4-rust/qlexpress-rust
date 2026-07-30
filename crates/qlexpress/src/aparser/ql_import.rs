//! 单个 QlExpress import 声明。

use super::import_scope::ImportScope;

/// 包、类型、内部类或类型别名导入。
///
/// 对应 Java: `com.alibaba.qlexpress4.aparser.ImportManager.QLImport`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QLImport {
    pub(crate) scope: ImportScope,
    pub(crate) target: String,
    pub(crate) alias: Option<String>,
}
