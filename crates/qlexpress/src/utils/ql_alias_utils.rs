//! Alias matching, mirroring Java `QLAliasUtils`.
//!
//! Java reads `@QLAlias` annotations via reflection; in Rust the alias
//! values are supplied explicitly (SPEC §4 native-registration strategy).

/// 定义 `QLAliasUtils` 对象。对应 Java 源码 `com/alibaba/qlexpress4/utils/QLAliasUtils.java:1` 的 `QLAliasUtils`；该方法为 Rust 同职责适配接口。
pub struct QLAliasUtils;

impl QLAliasUtils {
    /// Java `matchQLAlias`: true when `match_name` equals any alias value in
    /// any of the given alias groups.
    pub fn match_ql_alias(match_name: &str, ql_aliases: &[&[&str]]) -> bool {
        ql_aliases
            .iter()
            .any(|alias_group| alias_group.contains(&match_name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_any_alias_group() {
        let groups: [&[&str]; 2] = [&["size", "len"], &["count"]];
        assert!(QLAliasUtils::match_ql_alias("len", &groups));
        assert!(QLAliasUtils::match_ql_alias("count", &groups));
        assert!(!QLAliasUtils::match_ql_alias("other", &groups));
    }
}
