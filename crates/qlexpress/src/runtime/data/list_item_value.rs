//! Assignable list element, mirroring Java `ListItemValue`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::left_value::LeftValue;
use crate::runtime::value::{DataValue, Value};

/// Mirrors Java `ListItemValue`: an l-value view of `list[index]`.
pub struct ListItemValue {
    list: Rc<RefCell<Vec<DataValue>>>,
    index: usize,
}

impl ListItemValue {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/runtime/data/ListItemValue.java:16` 的 `ListItemValue::<init>`。
    pub fn new(list: Rc<RefCell<Vec<DataValue>>>, index: usize) -> Self {
        ListItemValue { list, index }
    }
}

impl Value for ListItemValue {
    fn get(&self) -> DataValue {
        self.list.borrow()[self.index].clone()
    }

    fn type_name(&self) -> &'static str {
        self.get().data_type_name()
    }
}

impl LeftValue for ListItemValue {
    /// Java returns `Object.class` — every value is accepted
    /// ([`TargetType::Any`]).
    fn defined_type(&self) -> Option<TargetType> {
        Some(TargetType::Any)
    }

    /// Java `list.set(index, newValue)`.
    fn set_inner(&mut self, new_value: DataValue) {
        self.list.borrow_mut()[self.index] = new_value;
    }

    /// Java returns `null`.
    fn symbol_name(&self) -> Option<&str> {
        None
    }
}

impl std::fmt::Debug for ListItemValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ListItemValue")
            .field("index", &self.index)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;

    #[test]
    fn set_accepts_any_type_like_java_object_component() {
        let list = Rc::new(RefCell::new(vec![DataValue::Int(1)]));
        let mut item = ListItemValue::new(Rc::clone(&list), 0);
        item.set(
            DataValue::Str("anything".into()),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
        assert_eq!(list.borrow()[0], DataValue::Str("anything".into()));
        assert_eq!(item.symbol_name(), None);
    }
}
