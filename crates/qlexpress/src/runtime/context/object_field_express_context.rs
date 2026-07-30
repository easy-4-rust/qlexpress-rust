//! 对象字段外部上下文,对应 Java `com.alibaba.qlexpress4.runtime.context.ObjectFieldExpressContext`。

use std::rc::Rc;

use crate::exception::QLException;
use crate::ql_options::Attachments;
use crate::runtime::context::express_context::ExpressContext;
use crate::runtime::native_registry::NativeRegistry;
use crate::runtime::value::{DataValue, QValue};

/// 对象字段上下文。对应 Java: com.alibaba.qlexpress4.runtime.context.ObjectFieldExpressContext
/// (职责:把一个宿主对象的公开字段/getter 暴露为脚本外部变量)。
///
/// Java 语义要点:Java 版 `get` 委托 `express4Runner.loadField(object, variableName)`,
/// 底层走反射读字段/getter;找不到时返回 `null`。
/// 按 SPEC §4,Rust 用 `NativeRegistry.load_field`(显式注册 + `NativeObject`
/// 字段读取)替代反射,找不到时返回 `Ok(None)`(即 Java 的 `null`)。
pub struct ObjectFieldExpressContext {
    /// 被暴露字段的宿主对象(Java `Object object`,此处为脚本世界中的值,
    /// 通常是 `DataValue::Object`)。
    object: DataValue,
    /// 成员解析注册表,承担 Java `Express4Runner.loadField` 的反射职责(SPEC §4)。
    registry: Rc<NativeRegistry>,
}

impl ObjectFieldExpressContext {
    /// 对应 Java 构造器 `ObjectFieldExpressContext(Object, Express4Runner)`。
    /// 第二参由 runner 换成其字段加载能力所依赖的 `NativeRegistry`。
    pub fn new(object: DataValue, registry: Rc<NativeRegistry>) -> Self {
        ObjectFieldExpressContext { object, registry }
    }
}

impl ExpressContext for ObjectFieldExpressContext {
    /// 对应 Java 方法 `get`:`express4Runner.loadField(object, variableName)`。
    fn get(
        &self,
        _attachments: &Attachments,
        variable_name: &str,
    ) -> Result<Option<QValue>, QLException> {
        // Java `Express4Runner#loadField` 显式传入 `skipSecurity=true`：
        // 对象上下文是宿主主动暴露的数据面，不应被脚本反射沙箱再次拦截。
        Ok(self
            .registry
            .load_field_with_security(&self.object, variable_name, true))
    }
}
