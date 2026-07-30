//! Popped stack arguments, mirroring Java `com.alibaba.qlexpress4.runtime.Parameters`.

use crate::runtime::value::{DataValue, QValue};

/// 保持操作数栈弹出顺序的函数或操作符实参视图。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Parameters.java`；具体对象路径见 `docs/对象级对照表.md`。
/// The result of popping `n` elements from the operand stack, mirroring
/// Java `Parameters`. Elements keep their original stack order: index 0 is
/// the deepest of the popped elements (Java `StackSwapParameters`).
/// 对应 Java:
/// `com.alibaba.qlexpress4.runtime.FixedSizeStack.StackSwapParameters`；
/// Rust 让弹出结果拥有值，避免 Java 内部类对原栈数组和游标的借用。
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.runtime.Parameters。
pub struct Parameters {
    values: Vec<QValue>,
}

impl Parameters {
    /// 创建空参数列表。
    /// 对应 Java: `com.alibaba.qlexpress4.runtime.Parameters` 的无参构造器。
    pub fn new(values: Vec<QValue>) -> Self {
        Parameters { values }
    }

    /// 查询 value。
    /// 参数：`i`；返回：`DataValue`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Parameters.java`，方法 `getValue`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java default `getValue(int)`: inner data at position `i`,
    /// [`DataValue::Null`] when out of range (Java `null`).
    pub fn get_value(&self, i: usize) -> DataValue {
        self.get(i).map(QValue::get).unwrap_or(DataValue::Null)
    }

    /// 按索引或键读取对应值。
    /// 参数：`i`；返回：`Option<&QValue>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Parameters.java`，方法 `get`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `get(int)`: stack element at position `i`, `None` when `i`
    /// exceeds the parameters' length (Java returns `null`).
    /// 对应 Java: com.alibaba.qlexpress4.runtime.Parameters#get。
    pub fn get(&self, i: usize) -> Option<&QValue> {
        self.values.get(i)
    }

    /// 返回元素数量。
    /// 无显式参数；返回：`usize`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Parameters.java`，方法 `size`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `size()`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.Parameters#size。
    pub fn size(&self) -> usize {
        self.values.len()
    }

    /// 判断参数列表是否为空。
    /// 对应 Java: `Parameters#size() == 0`。
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// 按原始顺序提取所有参数值。
    /// 无显式参数；返回：`Vec<DataValue>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Parameters.java`，方法 `values`；Rust 侧按所有权与 `Result` 语义适配。
    /// Inner data of every parameter, in order.
    pub fn values(&self) -> Vec<DataValue> {
        self.values.iter().map(QValue::get).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_value_out_of_range_is_null_like_java() {
        let params = Parameters::new(vec![DataValue::Int(1).into()]);
        assert_eq!(params.get_value(0), DataValue::Int(1));
        assert_eq!(params.get_value(1), DataValue::Null);
        assert!(params.get(5).is_none());
        assert_eq!(params.size(), 1);
    }
}
