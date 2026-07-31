//! Java 反射数组迭代器。
//!
//! 来源对象：
//! `com.alibaba.qlexpress4.runtime.instruction.ForEachInstruction.ReflectArrayIterable.ReflectArrayIterator`。

use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::data::JavaArray;
use crate::runtime::value::DataValue;

/// 按游标逐项读取共享 Java 数组。
///
/// 对应 Java：
/// `ForEachInstruction.ReflectArrayIterable.ReflectArrayIterator`。
pub(crate) struct ReflectArrayIterator {
    arr_obj: Rc<RefCell<JavaArray>>,
    cursor: usize,
}

impl ReflectArrayIterator {
    /// 从数组起始位置创建迭代器。
    ///
    /// # 参数
    ///
    /// - `arr_obj`：迭代期间保持共享引用语义的数组。
    pub(crate) fn new(arr_obj: Rc<RefCell<JavaArray>>) -> Self {
        Self { arr_obj, cursor: 0 }
    }

    /// 判断游标是否仍小于数组长度。
    ///
    /// 对应 Java：`ReflectArrayIterator#hasNext()`。
    pub(crate) fn has_next(&self) -> bool {
        self.cursor < self.arr_obj.borrow().len()
    }
}

impl Iterator for ReflectArrayIterator {
    type Item = DataValue;

    /// 返回当前元素并将游标加一。
    ///
    /// 对应 Java：`ReflectArrayIterator#next()`。通过 Rust `Iterator`
    /// 契约在越界时返回 `None`，循环调用点与 Java 增强 for 的行为一致。
    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_next() {
            return None;
        }
        let item = self.arr_obj.borrow().get(self.cursor).cloned();
        self.cursor += 1;
        item
    }
}
