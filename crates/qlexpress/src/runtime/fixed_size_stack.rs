//! QVM 定长操作数栈。对应 Java
//! `com.alibaba.qlexpress4.runtime.FixedSizeStack`。

use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::parameters::Parameters;
use crate::runtime::value::QValue;

/// 按编译期最大栈深限制容量的 QVM 操作数栈。
///
/// 对应 Java: `com.alibaba.qlexpress4.runtime.FixedSizeStack`。Java 使用
/// `Value[]` 与游标保存元素；Rust 使用共享的定长槽位数组保存相同状态，
/// 使 [`Parameters`] 保留 Java `StackSwapParameters` 对原数组的实时窗口语义。
#[derive(Clone, Debug)]
pub struct FixedSizeStack {
    elements: Rc<RefCell<Vec<Option<QValue>>>>,
    cursor: usize,
}

impl FixedSizeStack {
    /// 创建指定容量的操作数栈。
    ///
    /// 对应 Java 构造器 `FixedSizeStack(int size)`。
    pub fn new(size: usize) -> Self {
        Self {
            elements: Rc::new(RefCell::new(vec![None; size])),
            cursor: 0,
        }
    }

    /// 返回固定容量。Rust 侧诊断方法，用于验证 Java `elements.length` 语义。
    /// 对应 Java：`FixedSizeStack#elements.length`。
    pub fn capacity(&self) -> usize {
        self.elements.borrow().len()
    }

    /// 返回当前元素数。Rust 侧诊断方法，对应 Java 私有游标 `cursor`。
    pub fn len(&self) -> usize {
        self.cursor
    }

    /// 判断栈是否为空。Rust 侧诊断方法，对应 Java `cursor == 0`。
    pub fn is_empty(&self) -> bool {
        self.cursor == 0
    }

    /// 判断栈是否已达到编译期容量。Rust 侧诊断方法，对应 Java 数组边界。
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity()
    }

    /// 将值压入栈顶。
    ///
    /// 对应 Java 方法 `FixedSizeStack#push(Value)`。容量不足时触发边界
    /// panic，与 Java `elements[cursor++]` 抛出的数组越界异常一致；正常编译
    /// 的指令序列不会超过 `QvmInstructionVisitor#getMaxStackSize`。
    pub fn push(&mut self, ele: QValue) {
        assert!(
            !self.is_full(),
            "operand stack overflow (capacity = {})",
            self.capacity()
        );
        self.elements.borrow_mut()[self.cursor] = Some(ele);
        self.cursor += 1;
    }

    /// 弹出栈顶值。
    ///
    /// 对应 Java 方法 `FixedSizeStack#pop()`；空栈时触发边界 panic。
    pub fn pop(&mut self) -> QValue {
        assert!(self.cursor > 0, "operand stack underflow");
        self.cursor -= 1;
        self.elements.borrow()[self.cursor]
            .clone()
            .expect("initialized operand stack slot")
    }

    /// 读取但不弹出栈顶值。
    ///
    /// 对应 Java 方法 `FixedSizeStack#peak()`（Java 原方法名即 `peak`）。
    pub fn peak(&self) -> QValue {
        assert!(self.cursor > 0, "operand stack underflow");
        self.elements.borrow()[self.cursor - 1]
            .clone()
            .expect("operand stack underflow")
    }

    /// 一次弹出栈顶 `n` 个参数，并保持从深到浅的参数顺序。
    ///
    /// 对应 Java 方法 `FixedSizeStack#pop(int)` 及其内部类
    /// `StackSwapParameters`。返回对象仍引用同一槽位数组：后续 `push` 若
    /// 覆盖这些槽位，旧参数窗口会像 Java 内部类一样观察到新值。
    pub fn pop_n(&mut self, n: usize) -> Parameters {
        assert!(n <= self.cursor, "operand stack underflow");
        self.cursor -= n;
        Parameters::stack_view(Rc::clone(&self.elements), self.cursor, n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::value::DataValue;

    #[test]
    fn push_pop_and_peak_match_java_cursor_semantics() {
        let mut stack = FixedSizeStack::new(2);
        stack.push(DataValue::Int(1).into());
        stack.push(DataValue::Int(2).into());
        assert_eq!(stack.peak().get(), DataValue::Int(2));
        assert_eq!(stack.pop().get(), DataValue::Int(2));
        assert_eq!(stack.pop().get(), DataValue::Int(1));
        assert!(stack.is_empty());
    }

    #[test]
    #[should_panic(expected = "operand stack overflow")]
    fn push_beyond_compiled_capacity_panics_like_java_array_access() {
        let mut stack = FixedSizeStack::new(1);
        stack.push(DataValue::Int(1).into());
        stack.push(DataValue::Int(2).into());
    }

    #[test]
    fn pop_n_preserves_java_parameter_order() {
        let mut stack = FixedSizeStack::new(3);
        stack.push(DataValue::Int(10).into());
        stack.push(DataValue::Int(20).into());
        stack.push(DataValue::Int(30).into());
        let parameters = stack.pop_n(2);
        assert_eq!(parameters.get_value(0), DataValue::Int(20));
        assert_eq!(parameters.get_value(1), DataValue::Int(30));
        assert_eq!(stack.peak().get(), DataValue::Int(10));
    }

    /// Java `StackSwapParameters` 直接保存原 `Value[]`；弹出后复用槽位会改变
    /// 已返回参数窗口的读取结果，不能用拥有型 `Vec` 快照替代。
    #[test]
    fn popped_parameters_observe_reused_stack_slots_like_java_array_view() {
        let mut stack = FixedSizeStack::new(3);
        stack.push(DataValue::Int(1).into());
        stack.push(DataValue::Int(2).into());
        stack.push(DataValue::Int(3).into());

        let parameters = stack.pop_n(2);
        assert_eq!(
            parameters.values(),
            vec![DataValue::Int(2), DataValue::Int(3)]
        );

        stack.push(DataValue::Int(9).into());
        assert_eq!(
            parameters.values(),
            vec![DataValue::Int(9), DataValue::Int(3)]
        );
        stack.push(DataValue::Int(8).into());
        assert_eq!(
            parameters.values(),
            vec![DataValue::Int(9), DataValue::Int(8)]
        );
    }

    /// 逐项对应 Java `FixedSizeStackTest#pushPopTest`。
    #[test]
    fn java_push_pop_test_contract() {
        let mut stack = FixedSizeStack::new(4);
        for value in 1..=4 {
            stack.push(DataValue::Int(value).into());
        }
        assert_eq!(stack.pop().get(), DataValue::Int(4));
        assert_eq!(stack.pop().get(), DataValue::Int(3));
        stack.push(DataValue::Int(5).into());
        stack.push(DataValue::Int(6).into());

        let parameters = stack.pop_n(3);
        assert_eq!(parameters.get_value(0), DataValue::Int(2));
        assert_eq!(parameters.get_value(1), DataValue::Int(5));
        assert_eq!(parameters.get_value(2), DataValue::Int(6));
        assert!(parameters.get(3).is_none());
    }
}
