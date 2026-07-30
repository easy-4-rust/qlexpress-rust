//! Rust 宿主方法的 `@QLFunction` 扫描描述。

use std::rc::Rc;

use crate::runtime::function::custom_function::CustomFunction;

/// 一个由宿主显式暴露给 `add_obj_function/add_static_function` 扫描的方法。
///
/// 对应 Java `Class#getDeclaredMethods()` 返回的单个 `Method` 及其
/// `@QLFunction` 元数据。Rust 没有 JVM 运行时反射，因此由宿主实现
/// [`super::ql_function_provider::QLFunctionProvider`] 时生成同等描述。
pub struct QLFunctionMethod {
    method_name: String,
    is_public: bool,
    function_names: Option<Vec<String>>,
    function: Rc<dyn CustomFunction>,
}

impl QLFunctionMethod {
    /// 创建方法描述。
    ///
    /// - `method_name`：Rust 方法原名，对应 Java `Method#getName()`；
    /// - `is_public`：是否为公开方法，对应 `BasicUtil.isPublic(method)`；
    /// - `function_names`：`@QLFunction` 的值；`None` 表示未标注；
    /// - `function`：已经绑定实例（或静态目标）的调用实现。
    pub fn new(
        method_name: impl Into<String>,
        is_public: bool,
        function_names: Option<Vec<String>>,
        function: Rc<dyn CustomFunction>,
    ) -> Self {
        Self {
            method_name: method_name.into(),
            is_public,
            function_names,
            function,
        }
    }

    /// 返回宿主方法原名。对应 Java `Method#getName()`。
    pub fn method_name(&self) -> &str {
        &self.method_name
    }

    /// 返回方法是否公开。对应 Java `BasicUtil.isPublic(method)`。
    pub fn is_public(&self) -> bool {
        self.is_public
    }

    /// 返回 `@QLFunction` 声明的脚本函数名。
    ///
    /// `None` 表示方法未标注；`Some(empty)` 表示存在空值注解。
    pub fn function_names(&self) -> Option<&[String]> {
        self.function_names.as_deref()
    }

    /// 返回已绑定的函数调用实现。
    pub fn function(&self) -> Rc<dyn CustomFunction> {
        Rc::clone(&self.function)
    }
}
