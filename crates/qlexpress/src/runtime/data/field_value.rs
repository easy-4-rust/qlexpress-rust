//! Assignable field backed by getter/setter closures, mirroring Java
//! `FieldValue` (which wraps `Supplier<Object>` / `Consumer<Object>`).

use std::cell::RefCell;

use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::left_value::LeftValue;
use crate::runtime::value::{DataValue, Value};

/// Mirrors Java `FieldValue`: an l-value defined by a pair of accessor
/// operations plus the declared field type.
pub struct FieldValue {
    get_op: Box<dyn Fn() -> DataValue>,
    set_op: RefCell<Box<dyn FnMut(DataValue)>>,
    define_type: Option<TargetType>,
}

impl FieldValue {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/runtime/data/FieldValue.java:18` 的 `FieldValue::<init>`。
    pub fn new(
        get_op: Box<dyn Fn() -> DataValue>,
        set_op: Box<dyn FnMut(DataValue)>,
        define_type: Option<TargetType>,
    ) -> Self {
        FieldValue {
            get_op,
            set_op: RefCell::new(set_op),
            define_type,
        }
    }
}

impl Value for FieldValue {
    fn get(&self) -> DataValue {
        (self.get_op)()
    }

    fn type_name(&self) -> &'static str {
        self.get().data_type_name()
    }
}

impl LeftValue for FieldValue {
    fn defined_type(&self) -> Option<TargetType> {
        self.define_type
    }

    fn set_inner(&mut self, new_value: DataValue) {
        (self.set_op.borrow_mut())(new_value);
    }

    /// Java returns `null`.
    fn symbol_name(&self) -> Option<&str> {
        None
    }
}

impl std::fmt::Debug for FieldValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FieldValue")
            .field("define_type", &self.define_type)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;
    use std::rc::Rc;

    #[test]
    fn closures_drive_get_and_set() {
        let cell = Rc::new(RefCell::new(DataValue::Int(1)));
        let getter = {
            let cell = Rc::clone(&cell);
            move || cell.borrow().clone()
        };
        let setter = {
            let cell = Rc::clone(&cell);
            move |v: DataValue| *cell.borrow_mut() = v
        };
        let mut field = FieldValue::new(Box::new(getter), Box::new(setter), Some(TargetType::Int));
        assert_eq!(field.get(), DataValue::Int(1));
        // Typed set converts Long -> Int through ObjTypeConvertor.
        field
            .set(DataValue::Long(5), &PureErrReporter::INSTANCE)
            .unwrap();
        assert_eq!(*cell.borrow(), DataValue::Int(5));
    }
}
