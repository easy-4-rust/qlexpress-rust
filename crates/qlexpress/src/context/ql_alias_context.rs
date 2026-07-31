//! 别名外部上下文,对应 Java `com.alibaba.qlexpress4.runtime.context.QLAliasContext`。

use std::cell::RefCell;
use std::rc::Rc;

use crate::exception::QLException;
use crate::ql_options::Attachments;
use crate::runtime::context::express_context::ExpressContext;
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::data::MapItemValue;
use crate::runtime::value::{DataValue, QValue};

/// 别名上下文。对应 Java: com.alibaba.qlexpress4.runtime.context.QLAliasContext
/// (职责:把带有 `@QLAlias` 注解的宿主对象,按每个别名注册为外部变量)。
///
/// Java 语义要点:Java 版通过反射读取对象类上的 `@QLAlias` 注解,
/// 把 `别名 -> 对象` 放入一个 `HashMap`,`get` 时返回 `MapItemValue`。
/// Rust 无运行时注解(SPEC §4 显式注册策略),改为构造时显式传入
/// `(别名列表, 值)` 对,注册效果与 Java 完全一致。
pub struct QLAliasContext {
    /// 别名 -> 值 的映射(Java `Map<String, Object> context`)。
    context: Rc<RefCell<IndexMap>>,
}

impl QLAliasContext {
    /// 对应 Java 构造器 `QLAliasContext(Object... os)`。
    ///
    /// `aliased_values` 中每一项是 `(别名数组, 值)`:等价于 Java 中
    /// 一个类上声明了这些别名的对象被传入可变参数列表。
    /// 同一对象声明多个别名时,每个别名都指向同一个值(Java 中指向同一对象)。
    pub fn new(aliased_values: &[(&[&str], DataValue)]) -> Self {
        let mut context = IndexMap::new();
        for (aliases, value) in aliased_values {
            for alias in *aliases {
                context.insert(DataValue::string(*alias), value.clone());
            }
        }
        QLAliasContext {
            context: Rc::new(RefCell::new(context)),
        }
    }
}

impl ExpressContext for QLAliasContext {
    /// 对应 Java 方法 `get`:返回该别名的 `MapItemValue` 左值。
    fn get(
        &self,
        _attachments: &Attachments,
        variable_name: &str,
    ) -> Result<Option<QValue>, QLException> {
        let item: Rc<RefCell<dyn crate::runtime::left_value::LeftValue>> = Rc::new(RefCell::new(
            MapItemValue::new(Rc::clone(&self.context), DataValue::string(variable_name)),
        ));
        Ok(Some(QValue::Left(item)))
    }
}
