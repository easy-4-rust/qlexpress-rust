//! Assignable list element, mirroring Java `ListItemValue`.

use std::cell::RefCell;
use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::JavaArrayList;
use crate::runtime::left_value::LeftValue;
use crate::runtime::value::{DataValue, Value};

/// 指向共享列表指定下标并支持读取和写回的左值。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/data/ListItemValue.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Mirrors Java `ListItemValue`: an l-value view of `list[index]`.
/// 对应 Java: com.alibaba.qlexpress4.runtime.data.ListItemValue。
pub struct ListItemValue {
    list: Rc<RefCell<JavaArrayList>>,
    index: usize,
}

impl ListItemValue {
    /// 构造实例。对应 Java 源码 `com/alibaba/qlexpress4/runtime/data/ListItemValue.java:16` 的 `ListItemValue::<init>`。
    pub fn new(list: Rc<RefCell<JavaArrayList>>, index: usize) -> Self {
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
    /// (`java.lang.Object`)。
    fn defined_type(&self) -> Option<ClassRef> {
        Some(ClassRef::Named("java.lang.Object".to_string()))
    }

    /// Java `list.set(index, newValue)`.
    fn set_inner(&mut self, new_value: DataValue) -> Result<(), QLException> {
        self.list.borrow_mut().set(self.index, new_value);
        Ok(())
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
        let list = Rc::new(RefCell::new(JavaArrayList::new(vec![DataValue::Int(1)])));
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
