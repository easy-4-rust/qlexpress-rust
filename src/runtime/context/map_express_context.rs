//! Map 支撑的外部上下文,对应 Java `com.alibaba.qlexpress4.runtime.context.MapExpressContext`。

use std::cell::RefCell;
use std::rc::Rc;

use crate::exception::QLException;
use crate::ql_options::Attachments;
use crate::runtime::context::express_context::ExpressContext;
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::data::MapItemValue;
use crate::runtime::value::{DataValue, QValue};

/// Map 外部上下文。对应 Java: com.alibaba.qlexpress4.runtime.context.MapExpressContext
/// (职责:以宿主传入的 `Map<String, Object>` 作为外部变量来源)。
///
/// Java 语义要点:`get` 恒返回 `new MapItemValue(source, variableName)`,
/// 即使 key 不存在也返回一个可赋值的左值视图——
/// 这样在 `polluteUserContext` 模式下,脚本对变量的写入会穿透回宿主的 Map
/// (Java 靠 `MapItemValue` 持有 Map 引用实现写穿;Rust 用 `Rc<RefCell<IndexMap>>`
/// 复现同一别名语义)。
pub struct MapExpressContext {
    /// 外部变量来源(Java `Map<String, Object> source`)。
    source: Rc<RefCell<IndexMap>>,
}

impl MapExpressContext {
    /// 对应 Java 构造器 `MapExpressContext(Map<String, Object> source)`。
    pub fn new(source: Rc<RefCell<IndexMap>>) -> Self {
        MapExpressContext { source }
    }

    /// 取底层来源 Map(测试与宿主回读用)。Java 无对应方法(Rust 便捷访问器)。
    pub fn source(&self) -> &Rc<RefCell<IndexMap>> {
        &self.source
    }
}

impl ExpressContext for MapExpressContext {
    /// 对应 Java 方法 `get`:恒返回该 key 的 `MapItemValue` 左值。
    fn get(
        &self,
        _attachments: &Attachments,
        variable_name: &str,
    ) -> Result<Option<QValue>, QLException> {
        let item: Rc<RefCell<dyn crate::runtime::left_value::LeftValue>> =
            Rc::new(RefCell::new(MapItemValue::new(
                Rc::clone(&self.source),
                DataValue::Str(variable_name.to_string()),
            )));
        Ok(Some(QValue::Left(item)))
    }
}
