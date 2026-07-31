//! getter 候选方法及其优先级。
//!
//! 来源对象：`com.alibaba.qlexpress4.member.MethodHandler.GetterCandidateMethod`。

use std::rc::Rc;

use crate::runtime::native_type::NativeMethod;

/// getter 候选方法及其选择优先级。
///
/// 对应 Java：
/// `com.alibaba.qlexpress4.member.MethodHandler.GetterCandidateMethod`。
/// Java 以 `Method` 保存反射方法；Rust 保存显式注册的 [`NativeMethod`]。
#[derive(Clone)]
pub struct GetterCandidateMethod {
    method: NativeMethod,
    priority: i32,
}

impl GetterCandidateMethod {
    /// 创建 getter 候选。
    ///
    /// 对应 Java：`GetterCandidateMethod(Method, int)`。
    ///
    /// # 参数
    ///
    /// - `method`：显式注册的方法实现。
    /// - `priority`：候选优先级。
    pub fn new(method: NativeMethod, priority: i32) -> Self {
        Self { method, priority }
    }

    /// 返回候选方法。
    ///
    /// 对应 Java：`GetterCandidateMethod#getMethod()`。
    pub fn get_method(&self) -> NativeMethod {
        Rc::clone(&self.method)
    }

    /// 替换候选方法。
    ///
    /// 对应 Java：`GetterCandidateMethod#setMethod(Method)`。
    ///
    /// # 参数
    ///
    /// - `method`：新的显式注册方法。
    pub fn set_method(&mut self, method: NativeMethod) {
        self.method = method;
    }

    /// 返回候选优先级。
    ///
    /// 对应 Java：`GetterCandidateMethod#getPriority()`。
    pub fn get_priority(&self) -> i32 {
        self.priority
    }

    /// 设置候选优先级。
    ///
    /// 对应 Java：`GetterCandidateMethod#setPriority(int)`。
    ///
    /// # 参数
    ///
    /// - `priority`：新的选择优先级。
    pub fn set_priority(&mut self, priority: i32) {
        self.priority = priority;
    }
}
