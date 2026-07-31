//! Alias matching, mirroring Java `QLAliasUtils`.
//!
//! Java reads `@QLAlias` annotations via reflection; in Rust the alias
//! values are supplied explicitly (SPEC §4 native-registration strategy).

/// 在显式注册的别名组中匹配脚本成员名。
/// 对应 Java: `com.alibaba.qlexpress4.utils.QLAliasUtils`；替代运行时注解反射。
pub struct QLAliasUtils;

impl QLAliasUtils {
    /// 判断名称是否命中任一 QL 别名。
    /// 参数：`match_name`、`ql_aliases`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/annotation/QLAlias.java`，方法 `matchQlAlias`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `matchQLAlias`: true when `match_name` equals any alias value in
    /// any of the given alias groups.
    /// 对应 Java：`QLAliasUtils#matchQLAlias(String,QLAlias[])`。
    pub fn match_ql_alias(match_name: &str, ql_aliases: &[&[&str]]) -> bool {
        ql_aliases
            .iter()
            .any(|alias_group| alias_group.contains(&match_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOURCE_PARITY: Java `QLAliasUtils#matchQLAlias` 按注解数组和每个
    /// value 数组逐项扫描，任一相等即返回 true。
    #[test]
    fn matches_any_alias_group() {
        let groups: [&[&str]; 2] = [&["size", "len"], &["count"]];
        assert!(QLAliasUtils::match_ql_alias("len", &groups));
        assert!(QLAliasUtils::match_ql_alias("count", &groups));
        assert!(!QLAliasUtils::match_ql_alias("other", &groups));
        assert!(!QLAliasUtils::match_ql_alias("size", &[]));
        assert!(!QLAliasUtils::match_ql_alias("size", &[&[]]));
    }
}
