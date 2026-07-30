//! Assignable array element, mirroring Java `ArrayItemValue`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::JavaArray;
use crate::runtime::left_value::LeftValue;
use crate::runtime::native_registry::NativeRegistry;
use crate::runtime::value::{DataValue, Value};

/// 指向共享数组指定下标并支持读取和写回的左值。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/ArrayItemValue.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Mirrors Java `ArrayItemValue`: an l-value view of `array[index]`.
///
/// Java aliases a live Java array; Rust uses a shared `Rc<RefCell<JavaArray>>` so
/// the l-value and the owning array observe the same storage.
/// 对应 Java: com.alibaba.qlexpress4.runtime.data.ArrayItemValue。
pub struct ArrayItemValue {
    array: Rc<RefCell<JavaArray>>,
    index: usize,
    /// Element type of the array (Java `array.getClass().getComponentType()`);
    /// `None` mirrors a Java `Object[]` component type.
    component_type: Option<ClassRef>,
    type_registry: Option<Rc<NativeRegistry>>,
}

impl ArrayItemValue {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/runtime/data/ArrayItemValue.java:16` 的 `ArrayItemValue::<init>`。
    pub fn new(array: Rc<RefCell<JavaArray>>, index: usize) -> Self {
        let (component_type, type_registry) = {
            let borrowed = array.borrow();
            (
                Some(borrowed.component_type().clone()),
                borrowed.type_registry().map(Rc::clone),
            )
        };
        ArrayItemValue {
            array,
            index,
            component_type,
            type_registry,
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
    fn defined_type(&self) -> Option<ClassRef> {
        self.component_type.clone()
    }

    fn type_registry(&self) -> Option<&NativeRegistry> {
        self.type_registry.as_deref()
    }

    /// Java `Array.set(array, index, newValue)`.
    fn set_inner(&mut self, new_value: DataValue) {
        self.array.borrow_mut().set(self.index, new_value);
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
        let array = Rc::new(RefCell::new(JavaArray::object(vec![
            DataValue::Int(1),
            DataValue::Int(2),
        ])));
        let mut item = ArrayItemValue::new(Rc::clone(&array), 1);
        assert_eq!(item.get(), DataValue::Int(2));
        item.set(DataValue::Str("s".into()), &PureErrReporter::INSTANCE)
            .unwrap();
        assert_eq!(array.borrow()[1], DataValue::Str("s".into()));
    }

    #[test]
    fn component_type_is_checked() {
        let array = Rc::new(RefCell::new(JavaArray::typed(
            vec![DataValue::Int(1)],
            ClassRef::Primitive(
                crate::runtime::data::convert::obj_type_convertor::TargetType::Int,
            ),
            Rc::new(NativeRegistry::new()),
        )));
        let mut item = ArrayItemValue::new(Rc::clone(&array), 0);
        item.set(DataValue::Long(9), &PureErrReporter::INSTANCE)
            .unwrap();
        assert_eq!(array.borrow()[0], DataValue::Int(9));
        assert!(item
            .set(DataValue::Str("x".into()), &PureErrReporter::INSTANCE)
            .is_err());
    }
}
