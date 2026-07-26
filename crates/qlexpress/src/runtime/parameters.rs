//! Popped stack arguments, mirroring Java `com.alibaba.qlexpress4.runtime.Parameters`.

use crate::runtime::value::{DataValue, QValue};

/// The result of popping `n` elements from the operand stack, mirroring
/// Java `Parameters`. Elements keep their original stack order: index 0 is
/// the deepest of the popped elements (Java `StackSwapParameters`).
#[derive(Clone, Debug)]
pub struct Parameters {
    values: Vec<QValue>,
}

impl Parameters {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/runtime/Parameters.java:1` 的 `Parameters`；该方法为 Rust 同职责适配接口。
    pub fn new(values: Vec<QValue>) -> Self {
        Parameters { values }
    }

    /// Java default `getValue(int)`: inner data at position `i`,
    /// [`DataValue::Null`] when out of range (Java `null`).
    pub fn get_value(&self, i: usize) -> DataValue {
        self.get(i).map(QValue::get).unwrap_or(DataValue::Null)
    }

    /// Java `get(int)`: stack element at position `i`, `None` when `i`
    /// exceeds the parameters' length (Java returns `null`).
    pub fn get(&self, i: usize) -> Option<&QValue> {
        self.values.get(i)
    }

    /// Java `size()`.
    pub fn size(&self) -> usize {
        self.values.len()
    }

    /// 执行 `is_empty` 公开操作。对应 Java 源码 `com/alibaba/qlexpress4/runtime/Parameters.java:1` 的 `Parameters`；该方法为 Rust 同职责适配接口。
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

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
