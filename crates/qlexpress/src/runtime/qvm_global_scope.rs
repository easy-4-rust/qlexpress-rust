//! 全局作用域,对应 Java `com.alibaba.qlexpress4.runtime.QvmGlobalScope`。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::exception::QLException;
use crate::ql_options::{Attachments, SharedAttachments};
use crate::runtime::context::{EmptyContext, ExpressContext, MapExpressContext};
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::data::AssignableDataValue;
use crate::runtime::function::CustomFunction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::scope::SharedFunctionTable;
use crate::runtime::value::{DataValue, QValue};

/// 根作用域:持有外部变量/函数与脚本全局级新建的变量。
/// 对应 Java: com.alibaba.qlexpress4.runtime.QvmGlobalScope
///
/// Stage 5a 接线:外部变量查找走 [`ExpressContext`](Java `ExpressContext
/// externalVariable`),替换 Stage 3a 的 `IndexMap` 占位;`new` 兼容构造器
/// 内部以 [`MapExpressContext`] 包装传入的 Map,行为与 Stage 3a 一致
/// (外部条目是可写穿的 `MapItemValue`)。
pub struct QvmGlobalScope {
    /// 外部变量上下文(Java `ExpressContext externalVariable`)。
    external_variable: Rc<dyn ExpressContext>,
    /// 脚本中首次提及的变量(Java `Map<String, LeftValue> newVariables`)。
    new_variables: HashMap<String, Rc<RefCell<dyn LeftValue>>>,
    /// 外部(宿主注册)函数(Java `Map<String, CustomFunction> externalFunction`)。
    ///
    /// Java 直接保存 runner 的同一张 Map；这里也共享同一注册表，使已经
    /// 创建的 Lambda 能观察后续宿主函数注册。
    external_functions: Rc<RefCell<HashMap<String, Rc<dyn CustomFunction>>>>,
    /// 用户附加数据(Java `qlOptions.getAttachments()`,每次查找时透传给上下文)。
    attachments: SharedAttachments,
    /// Java `qlOptions.isPolluteUserContext()`,每次查找时判定。
    pollute_user_context: bool,
}

impl QvmGlobalScope {
    /// 判断脚本变量是否已在全局作用域创建，且不触发外部上下文查询。
    /// 对应 Java：`Map.containsKey` 级别的符号存在性判断。
    pub fn has_declared_symbol(&self, var_name: &str) -> bool {
        self.new_variables.contains_key(var_name)
    }

    /// Stage 3a 兼容构造器:以 `IndexMap` 为外部变量来源
    /// (内部包装为 [`MapExpressContext`],对应 Java 以
    /// `MapExpressContext` 作为 `externalVariable` 的用法)。
    pub fn new(
        external_variables: Rc<RefCell<IndexMap>>,
        external_functions: HashMap<String, Rc<dyn CustomFunction>>,
        pollute_user_context: bool,
    ) -> Self {
        Self::with_context(
            Rc::new(MapExpressContext::new(external_variables)),
            external_functions,
            HashMap::new(),
            pollute_user_context,
        )
    }

    /// 对应 Java 构造器 `QvmGlobalScope(ExpressContext, Map, QLOptions)`。
    /// (`QLOptions` 在此展开为本作用域实际读取的两个字段:
    /// `attachments` 与 `pollute_user_context`。)
    pub fn with_context(
        external_variable: Rc<dyn ExpressContext>,
        external_functions: HashMap<String, Rc<dyn CustomFunction>>,
        attachments: Attachments,
        pollute_user_context: bool,
    ) -> Self {
        Self::with_shared_context(
            external_variable,
            Rc::new(RefCell::new(external_functions)),
            Rc::new(RefCell::new(attachments)),
            pollute_user_context,
        )
    }

    /// 使用共享宿主函数表创建全局作用域。
    ///
    /// 对应 Java 构造器直接保存 `externalFunction` Map 引用的行为；runner
    /// 通过此入口保证已创建 Lambda 与后续函数注册共享同一可见性边界。
    ///
    /// # 参数
    ///
    /// - `external_variable`：外部变量上下文。
    /// - `external_functions`：runner 生命周期内共享的宿主函数表。
    /// - `attachments`：查询外部上下文时透传的附加数据。
    /// - `pollute_user_context`：赋值是否写穿外部上下文。
    pub fn with_shared_context(
        external_variable: Rc<dyn ExpressContext>,
        external_functions: Rc<RefCell<HashMap<String, Rc<dyn CustomFunction>>>>,
        attachments: SharedAttachments,
        pollute_user_context: bool,
    ) -> Self {
        QvmGlobalScope {
            external_variable,
            new_variables: HashMap::new(),
            external_functions,
            attachments,
            pollute_user_context,
        }
    }

    /// 空全局作用域(无外部变量/函数)。
    /// 对应 Java 以 `ExpressContext.EMPTY_CONTEXT` 构造的用法。
    pub fn empty() -> Self {
        Self::with_context(
            Rc::new(EmptyContext::new()),
            HashMap::new(),
            HashMap::new(),
            false,
        )
    }

    /// 对应 Java 方法 `getSymbol(String)`:
    /// 脚本变量优先;否则查询外部上下文——`polluteUserContext` 时直接返回
    /// 外部值(Java 靠引用别名实现写穿),否则把外部当前值拷贝为新的脚本变量;
    /// 外部返回 `null`(`Ok(None)`)时新建初始化为 `null` 的脚本变量。
    pub fn get_symbol(
        &mut self,
        var_name: &str,
    ) -> Result<Rc<RefCell<dyn LeftValue>>, QLException> {
        if let Some(new_variable) = self.new_variables.get(var_name) {
            return Ok(Rc::clone(new_variable));
        }
        // Java: Value externalValue = externalVariable.get(qlOptions.getAttachments(), varName);
        let attachments = self.attachments.borrow();
        let external_value = self.external_variable.get(&attachments, var_name)?;
        drop(attachments);
        if let Some(external) = external_value {
            if self.pollute_user_context {
                // Java 直接返回外部 Value:左值(MapItemValue)写穿宿主 Map;
                // 不可变数据(Java DataValue)包一层可赋值壳,写操作落于壳内
                // (Java 中 DataValue 本就不可写)。
                let symbol: Rc<RefCell<dyn LeftValue>> = match external {
                    QValue::Left(left) => left,
                    QValue::Data(data) => Rc::new(RefCell::new(AssignableDataValue::new(
                        var_name.to_string(),
                        data,
                    ))),
                };
                return Ok(symbol);
            }
            let initial = external.get();
            let new_variable: Rc<RefCell<dyn LeftValue>> =
                Rc::new(RefCell::new(AssignableDataValue::new(var_name, initial)));
            self.new_variables
                .insert(var_name.to_string(), Rc::clone(&new_variable));
            Ok(new_variable)
        } else {
            let new_variable: Rc<RefCell<dyn LeftValue>> = Rc::new(RefCell::new(
                AssignableDataValue::new(var_name, DataValue::Null),
            ));
            self.new_variables
                .insert(var_name.to_string(), Rc::clone(&new_variable));
            Ok(new_variable)
        }
    }

    /// 对应 Java 方法 `defineLocalSymbol`:全局作用域不支持。
    pub fn define_local_symbol(&mut self, _var_name: &str) -> ! {
        panic!("UnsupportedOperationException: defineLocalSymbol on QvmGlobalScope")
    }

    /// 对应 Java 方法 `defineFunction`:全局作用域不支持。
    pub fn define_function(&mut self, _function_name: &str) -> ! {
        panic!("UnsupportedOperationException: defineFunction on QvmGlobalScope")
    }

    /// 对应 Java 方法 `getFunction`:此处仅外部函数可见。
    pub fn get_function(&self, function_name: &str) -> Option<Rc<dyn CustomFunction>> {
        self.external_functions.borrow().get(function_name).cloned()
    }

    /// 对应 Java 方法 `getFunctionTable`。
    ///
    /// Rust 返回共享句柄，保留 Java 返回实际可变函数表的写穿语义。
    pub fn function_table(&self) -> SharedFunctionTable {
        Rc::clone(&self.external_functions)
    }

    /// 脚本自建变量表(Java `newVariables`)。
    /// 对应 Java：`QvmGlobalScope#newVariables` 字段。
    pub fn new_variables(&self) -> &HashMap<String, Rc<RefCell<dyn LeftValue>>> {
        &self.new_variables
    }

    /// 外部变量上下文(Java `externalVariable` 字段)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.QvmGlobalScope#externalVariable。
    pub fn external_variable(&self) -> &Rc<dyn ExpressContext> {
        &self.external_variable
    }
}
