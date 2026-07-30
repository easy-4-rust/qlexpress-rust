//! 部分限定类型名的解析结果。

/// 已解析类型与尚未消费的路径起始位置。
///
/// 对应 Java:
/// `com.alibaba.qlexpress4.aparser.ImportManager.LoadPartQualifiedResult`。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadPartQualifiedResult {
    pub(crate) cls: Option<String>,
    pub(crate) rest_index: usize,
}
