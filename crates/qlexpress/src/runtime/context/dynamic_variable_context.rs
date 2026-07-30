//! 动态变量外部上下文,对应 Java `com.alibaba.qlexpress4.runtime.context.DynamicVariableContext`。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::exception::QLException;
use crate::ql_options::Attachments;
use crate::runtime::context::express_context::ExpressContext;
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::data::MapItemValue;
use crate::runtime::value::{DataValue, QValue};

/// 动态脚本执行器,替代 Java 版持有的 `Express4Runner runner`。
///
/// Java 语义:`get` 命中动态变量时调用
/// `runner.execute(dynamicScript, this, qlOptions)` 并把结果包成 `DataValue`。
/// Rust 侧 `Express4Runner` 门面在后续阶段落地,这里抽象为闭包:
/// 入参为 `(动态脚本, 当前上下文)`,返回脚本执行结果。
/// 实现该闭包时应当用「本上下文」作为外部变量环境执行脚本,与 Java 传 `this` 一致。
/// 对应 Java: `DynamicVariableContext` 持有 runner 的 Rust 闭包适配。
pub type DynamicScriptRunner =
    Rc<dyn Fn(&str, &dyn ExpressContext) -> Result<DataValue, QLException>>;

/// 动态变量上下文。对应 Java: com.alibaba.qlexpress4.runtime.context.DynamicVariableContext
/// (职责:变量名既可映射到一段「动态脚本」(取值时即时执行),
/// 也可落到静态 Map 上下文)。
pub struct DynamicVariableContext {
    /// 动态脚本执行器(Java `Express4Runner runner`)。
    runner: DynamicScriptRunner,
    /// 静态变量来源(Java `Map<String, Object> staticContext`)。
    static_context: Rc<RefCell<IndexMap>>,
    /// 变量名 -> 动态脚本(Java `Map<String, String> dynamicContext`)。
    dynamic_context: RefCell<HashMap<String, String>>,
}

impl DynamicVariableContext {
    /// 对应 Java 构造器 `DynamicVariableContext(runner, staticContext, qlOptions)`。
    /// (`qlOptions` 在 Java 中仅用于转发给 `runner.execute`,Rust 由闭包自行捕获。)
    pub fn new(runner: DynamicScriptRunner, static_context: Rc<RefCell<IndexMap>>) -> Self {
        DynamicVariableContext {
            runner,
            static_context,
            dynamic_context: RefCell::new(HashMap::new()),
        }
    }

    /// 对应 Java 构造器 `DynamicVariableContext(runner, staticContext, qlOptions, dynamicContext)`。
    pub fn with_dynamic_context(
        runner: DynamicScriptRunner,
        static_context: Rc<RefCell<IndexMap>>,
        dynamic_context: HashMap<String, String>,
    ) -> Self {
        DynamicVariableContext {
            runner,
            static_context,
            dynamic_context: RefCell::new(dynamic_context),
        }
    }

    /// 对应 Java 方法 `put(String name, String valueExpression)`:
    /// 注册一个动态变量(取值时执行 `value_expression` 脚本)。
    pub fn put(&self, name: &str, value_expression: &str) {
        self.dynamic_context
            .borrow_mut()
            .insert(name.to_string(), value_expression.to_string());
    }
}

impl ExpressContext for DynamicVariableContext {
    /// 对应 Java 方法 `get`:动态变量优先,执行其脚本;否则回退静态 Map 的
    /// `MapItemValue`(Java `new MapItemValue(staticContext, variableName)`)。
    fn get(
        &self,
        _attachments: &Attachments,
        variable_name: &str,
    ) -> Result<Option<QValue>, QLException> {
        let dynamic_script = self.dynamic_context.borrow().get(variable_name).cloned();
        if let Some(dynamic_script) = dynamic_script {
            // Java: runner.execute(dynamicScript, this, qlOptions),把自身作为上下文递归求值。
            let result = (self.runner)(&dynamic_script, self)?;
            return Ok(Some(QValue::Data(result)));
        }
        let item: Rc<RefCell<dyn crate::runtime::left_value::LeftValue>> =
            Rc::new(RefCell::new(MapItemValue::new(
                Rc::clone(&self.static_context),
                DataValue::Str(variable_name.to_string()),
            )));
        Ok(Some(QValue::Left(item)))
    }
}
