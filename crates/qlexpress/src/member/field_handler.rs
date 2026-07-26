//! 字段处理器,对应 Java `com.alibaba.qlexpress4.member.FieldHandler`。
//!
//! 适配说明(SPEC §4):Java 版通过反射遍历声明字段并读取 `@QLAlias`
//! 注解,且沿父类链递归;Rust 无运行时注解与继承信息,改为在
//! [`NativeType`] 上显式注册的字段表/别名表内查找,匹配语义不变。

use crate::runtime::native_type::NativeType;
use crate::utils::ql_alias_utils::QLAliasUtils;

/// 字段处理器。对应 Java: com.alibaba.qlexpress4.member.FieldHandler
/// (职责:按属性名(含别名)在类型上定位字段)。
pub struct FieldHandler;

/// 优选处理。对应 Java: `FieldHandler.Preferred`(静态嵌套类)。
pub struct Preferred;

impl Preferred {
    /// 对应 Java 方法 `Preferred.preHandleAlias(Class<?>, String)`:
    /// 若 `property_name` 命中某字段的 `@QLAlias` 别名,返回该字段的真名;
    /// 否则原样返回 `property_name`。
    ///
    /// Java 沿父类链递归;Rust 的 `NativeType.field_aliases`
    /// (字段名 -> 别名列表)即注册时的「拍平」结果,等价于递归后的全集。
    pub fn pre_handle_alias<'a>(native_type: &'a NativeType, property_name: &'a str) -> &'a str {
        for (field_name, aliases) in &native_type.field_aliases {
            let group: Vec<&str> = aliases.iter().map(String::as_str).collect();
            if QLAliasUtils::match_ql_alias(property_name, &[&group]) {
                return field_name;
            }
        }
        property_name
    }

    /// 对应 Java 方法 `Preferred.gatherFieldRecursive(Class<?>, String)`:
    /// 按真名或别名命中字段名,未命中返回 `None`(Java 返回 `null`)。
    pub fn gather_field_recursive(
        native_type: &NativeType,
        property_name: &str,
    ) -> Option<String> {
        // 真名命中(Java: propertyName.equals(field.getName()))。
        if native_type.fields.contains_key(property_name)
            || native_type.static_fields.contains_key(property_name)
        {
            return Some(property_name.to_string());
        }
        // 别名命中(Java: QLAliasUtils.matchQLAlias)。
        for (field_name, aliases) in &native_type.field_aliases {
            let group: Vec<&str> = aliases.iter().map(String::as_str).collect();
            if QLAliasUtils::match_ql_alias(property_name, &[&group]) {
                return Some(field_name.clone());
            }
        }
        None
    }
}
