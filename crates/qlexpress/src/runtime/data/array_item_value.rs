//! Assignable array element, mirroring Java `ArrayItemValue`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::left_value::LeftValue;
use crate::runtime::value::{DataValue, Value};

/// `ArrayItemValue` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/ArrayItemValue.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Mirrors Java `ArrayItemValue`: an l-value view of `array[index]`.
///
/// Java aliases a live Java array; Rust uses a shared `Rc<RefCell<Vec>>` so
/// the l-value and the owning array observe the same storage.
pub struct ArrayItemValue {
    array: Rc<RefCell<Vec<DataValue>>>,
    index: usize,
    /// Element type of the array (Java `array.getClass().getComponentType()`);
    /// `None` mirrors a Java `Object[]` component type.
    component_type: Option<TargetType>,
}

impl ArrayItemValue {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/runtime/data/ArrayItemValue.java:16` 的 `ArrayItemValue::<init>`。
    pub fn new(array: Rc<RefCell<Vec<DataValue>>>, index: usize) -> Self {
        ArrayItemValue {
            array,
            index,
            component_type: None,
        }
    }

    /// 指定数组元素的目标类型，用于写回时执行 Java 兼容转换。
    /// 对应 Java: `ArrayItemValue` 从数组 component type 推导赋值类型。
    pub fn with_component_type(
        array: Rc<RefCell<Vec<DataValue>>>,
        index: usize,
        component_type: TargetType,
    ) -> Self {
        ArrayItemValue {
            array,
            index,
            component_type: Some(component_type),
        }
    }
}

impl Value for ArrayItemValue {
    fn get(&self) -> DataValue {
        self.array.borrow()[self.index].clone()
    }

    fn type_name(&self) -> &'static str {
        self.get().data_type_name()
    }
}

impl LeftValue for ArrayItemValue {
    /// Java `ArrayItemValue.getDefinedType`: the array's component type.
    fn defined_type(&self) -> Option<TargetType> {
        self.component_type
    }

    /// Java `Array.set(array, index, newValue)`.
    fn set_inner(&mut self, new_value: DataValue) {
        self.array.borrow_mut()[self.index] = new_value;
    }

    /// Java returns `null`.
    fn symbol_name(&self) -> Option<&str> {
        None
    }
}

impl std::fmt::Debug for ArrayItemValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArrayItemValue")
            .field("index", &self.index)
            .field("component_type", &self.component_type)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;

    #[test]
    fn set_writes_through_to_shared_array() {
        let array = Rc::new(RefCell::new(vec![DataValue::Int(1), DataValue::Int(2)]));
        let mut item = ArrayItemValue::new(Rc::clone(&array), 1);
        assert_eq!(item.get(), DataValue::Int(2));
        item.set(DataValue::Str("s".into()), &PureErrReporter::INSTANCE)
            .unwrap();
        assert_eq!(array.borrow()[1], DataValue::Str("s".into()));
    }

    #[test]
    fn component_type_is_checked() {
        let array = Rc::new(RefCell::new(vec![DataValue::Int(1)]));
        let mut item = ArrayItemValue::with_component_type(Rc::clone(&array), 0, TargetType::Int);
        item.set(DataValue::Long(9), &PureErrReporter::INSTANCE)
            .unwrap();
        assert_eq!(array.borrow()[0], DataValue::Int(9));
        assert!(item
            .set(DataValue::Str("x".into()), &PureErrReporter::INSTANCE)
            .is_err());
    }
}
