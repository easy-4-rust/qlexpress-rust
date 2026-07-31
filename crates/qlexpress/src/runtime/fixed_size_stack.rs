//! QVM 定长操作数栈。对应 Java
//! `com.alibaba.qlexpress4.runtime.FixedSizeStack`。

use crate::runtime::parameters::Parameters;
use crate::runtime::value::QValue;

/// 按编译期最大栈深限制容量的 QVM 操作数栈。
///
/// 对应 Java: `com.alibaba.qlexpress4.runtime.FixedSizeStack`。Java 使用
/// `Value[]` 与游标保存元素；Rust 使用 `Vec<QValue>` 保存同样的有效区间，
/// 并在 `push` 时显式执行数组边界检查。
#[derive(Clone, Debug)]
pub struct FixedSizeStack {
    elements: Vec<QValue>,
    capacity: usize,
}

impl FixedSizeStack {
    /// 创建指定容量的操作数栈。
    ///
    /// 对应 Java 构造器 `FixedSizeStack(int size)`。
    pub fn new(size: usize) -> Self {
        Self {
            elements: Vec::with_capacity(size),
            capacity: size,
        }
    }

    /// 返回固定容量。Rust 侧诊断方法，用于验证 Java `elements.length` 语义。
    /// 对应 Java：`FixedSizeStack#elements.length`。
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// 返回当前元素数。Rust 侧诊断方法，对应 Java 私有游标 `cursor`。
    pub fn len(&self) -> usize {
        self.elements.len()
    }

    /// 判断栈是否为空。Rust 侧诊断方法，对应 Java `cursor == 0`。
    pub fn is_empty(&self) -> bool {
        self.elements.is_empty()
    }

    /// 判断栈是否已达到编译期容量。Rust 侧诊断方法，对应 Java 数组边界。
    pub fn is_full(&self) -> bool {
        self.len() == self.capacity
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
            self.capacity
        );
        self.elements.push(ele);
    }

    /// 弹出栈顶值。
    ///
    /// 对应 Java 方法 `FixedSizeStack#pop()`；空栈时触发边界 panic。
    pub fn pop(&mut self) -> QValue {
        self.elements.pop().expect("operand stack underflow")
    }

    /// 读取但不弹出栈顶值。
    ///
    /// 对应 Java 方法 `FixedSizeStack#peak()`（Java 原方法名即 `peak`）。
    pub fn peak(&self) -> QValue {
        self.elements
            .last()
            .cloned()
            .expect("operand stack underflow")
    }

    /// 一次弹出栈顶 `n` 个参数，并保持从深到浅的参数顺序。
    ///
    /// 对应 Java 方法 `FixedSizeStack#pop(int)` 及其内部类
    /// `StackSwapParameters`。Rust 的 [`Parameters`] 拥有弹出值，避免跨
    /// `RefCell` 借用，同时保持 `get(i)` 与 `size()` 的可观察语义。
    pub fn pop_n(&mut self, n: usize) -> Parameters {
        let len = self.elements.len();
        assert!(n <= len, "operand stack underflow");
        Parameters::new(self.elements.split_off(len - n))
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
