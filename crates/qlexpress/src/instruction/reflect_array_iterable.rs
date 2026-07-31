//! Java 反射数组的可迭代适配器。
//!
//! 来源对象：
//! `com.alibaba.qlexpress4.runtime.instruction.ForEachInstruction.ReflectArrayIterable`。

use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::data::JavaArray;

use super::reflect_array_iterator::ReflectArrayIterator;

/// 将共享 Java 数组包装为可重复创建迭代器的适配器。
///
/// 对应 Java：
/// `ForEachInstruction.ReflectArrayIterable`。Java 持有任意反射数组对象；
/// Rust 数组统一为共享 [`JavaArray`]，但仍保留每次 `iterator` 从零开始的语义。
#[derive(Clone)]
pub(crate) struct ReflectArrayIterable {
    arr_obj: Rc<RefCell<JavaArray>>,
}

impl ReflectArrayIterable {
    /// 创建数组可迭代适配器。
    ///
    /// 对应 Java 私有构造器 `ReflectArrayIterable(Object)`。
    ///
    /// # 参数
    ///
    /// - `arr_obj`：需要按 Java 反射数组规则遍历的共享数组。
    pub(crate) fn new(arr_obj: Rc<RefCell<JavaArray>>) -> Self {
        Self { arr_obj }
    }

    /// 创建一个游标从零开始的数组迭代器。
    ///
    /// 对应 Java：`ReflectArrayIterable#iterator()`。
    ///
    /// # 返回值
    ///
    /// 返回持有同一数组引用、游标独立的新迭代器。
    pub(crate) fn iterator(&self) -> ReflectArrayIterator {
        ReflectArrayIterator::new(Rc::clone(&self.arr_obj))
    }
}
