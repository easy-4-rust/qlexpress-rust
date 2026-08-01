//! Popped stack arguments, mirroring Java `com.alibaba.qlexpress4.runtime.Parameters`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::value::{DataValue, QValue};

#[derive(Clone, Debug)]
enum ParameterStorage {
    Owned(Vec<QValue>),
    StackView {
        elements: Rc<RefCell<Vec<Option<QValue>>>>,
        start: usize,
        length: usize,
    },
}

/// 保持操作数栈弹出顺序的函数或操作符实参视图。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Parameters.java`；具体对象路径见 `docs/对象级对照表.md`。
/// The result of popping `n` elements from the operand stack, mirroring
/// Java `Parameters`. Elements keep their original stack order: index 0 is
/// the deepest of the popped elements (Java `StackSwapParameters`).
/// 对应 Java:
/// `com.alibaba.qlexpress4.runtime.FixedSizeStack.StackSwapParameters`；
/// Rust 的栈窗口保留对共享槽位数组的引用，以复现 Java 内部类的覆盖可见性；
/// 宿主直接构造的参数则使用拥有型存储。
#[derive(Clone, Debug)]
/// 对应 Java: com.alibaba.qlexpress4.runtime.Parameters。
pub struct Parameters {
    storage: ParameterStorage,
}

impl Parameters {
    /// 创建拥有型参数列表。
    ///
    /// Java `Parameters` 是接口，没有构造器；该 Rust 便捷入口供宿主适配器
    /// 直接提供参数值，栈弹出路径则使用内部 `Parameters::stack_view`。
    pub fn new(values: Vec<QValue>) -> Self {
        Parameters {
            storage: ParameterStorage::Owned(values),
        }
    }

    /// 创建 Java `FixedSizeStack.StackSwapParameters` 的共享槽位窗口。
    ///
    /// 后续操作数栈写入相同槽位时，读取结果同步变化，与 Java 内部类持有
    /// 原始 `Value[]` 引用的副作用一致。
    pub(crate) fn stack_view(
        elements: Rc<RefCell<Vec<Option<QValue>>>>,
        start: usize,
        length: usize,
    ) -> Self {
        Parameters {
            storage: ParameterStorage::StackView {
                elements,
                start,
                length,
            },
        }
    }

    /// 查询 value。
    /// 参数：`i`；返回：`DataValue`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Parameters.java`，方法 `getValue`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java default `getValue(int)`: inner data at position `i`,
    /// [`DataValue::Null`] when out of range (Java `null`).
    /// 对应 Java：`Parameters#getValue(int)`。
    pub fn get_value(&self, i: usize) -> DataValue {
        self.get(i)
            .map(|value| value.get())
            .unwrap_or(DataValue::Null)
    }

    /// 按索引或键读取对应值。
    /// 参数：`i`；返回：`Option<QValue>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Parameters.java`，方法 `get`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `get(int)`: stack element at position `i`, `None` when `i`
    /// exceeds the parameters' length (Java returns `null`).
    /// 对应 Java: com.alibaba.qlexpress4.runtime.Parameters#get。
    pub fn get(&self, i: usize) -> Option<QValue> {
        match &self.storage {
            ParameterStorage::Owned(values) => values.get(i).cloned(),
            ParameterStorage::StackView {
                elements,
                start,
                length,
            } => {
                if i >= *length {
                    None
                } else {
                    elements.borrow()[start + i].clone()
                }
            }
        }
    }

    /// 返回元素数量。
    /// 无显式参数；返回：`usize`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Parameters.java`，方法 `size`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `size()`.
    /// 对应 Java: com.alibaba.qlexpress4.runtime.Parameters#size。
    pub fn size(&self) -> usize {
        match &self.storage {
            ParameterStorage::Owned(values) => values.len(),
            ParameterStorage::StackView { length, .. } => *length,
        }
    }

    /// 判断参数列表是否为空。
    /// 对应 Java: `Parameters#size() == 0`。
    pub fn is_empty(&self) -> bool {
        self.size() == 0
    }

    /// 按原始顺序提取所有参数值。
    /// 无显式参数；返回：`Vec<DataValue>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/Parameters.java`，方法 `values`；Rust 侧按所有权与 `Result` 语义适配。
    /// Inner data of every parameter, in order.
    /// 对应 Java：逐项调用 `Parameters#getValue(int)` 的 Rust 批量便捷接口。
    pub fn values(&self) -> Vec<DataValue> {
        (0..self.size()).map(|i| self.get_value(i)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_value_out_of_range_is_null_like_java() {
        let params = Parameters::new(vec![DataValue::Int(1).into(), DataValue::Null.into()]);
        assert_eq!(params.get_value(0), DataValue::Int(1));
        assert!(params.get(1).is_some());
        assert_eq!(params.get_value(1), DataValue::Null);
        assert!(params.get(5).is_none());
        assert_eq!(params.get_value(5), DataValue::Null);
        assert_eq!(params.size(), 2);
    }
}
