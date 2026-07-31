//! QVM 栈值适配类型，承载不可变数据或可赋值左值。

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::runtime::left_value::LeftValue;
use crate::runtime::value::DataValue;

/// QVM 操作数栈元素。
///
/// Java 无同名对象；它对应 Java 栈上统一的 `Value` 引用，并显式区分
/// `DataValue` 与 `LeftValue`，以保留赋值别名语义。
#[derive(Clone)]
pub enum QValue {
    /// 不可变数据值，对应 Java `DataValue`。
    Data(DataValue),
    /// 可赋值共享槽位，对应 Java `LeftValue` 实现。
    Left(Rc<RefCell<dyn LeftValue>>),
}

impl QValue {
    /// 取得内部数据。对应 Java 方法 `Value#get()`。
    pub fn get(&self) -> DataValue {
        match self {
            QValue::Data(value) => value.clone(),
            QValue::Left(left_value) => left_value.borrow().get(),
        }
    }

    /// 取得 Java 风格类型名。对应 Java 方法 `Value#getTypeName()`。
    pub fn type_name(&self) -> &'static str {
        match self {
            QValue::Data(value) => value.data_type_name(),
            QValue::Left(left_value) => left_value.borrow().type_name(),
        }
    }

    /// 将左值快照为不可变数据。对应 Java 方法
    /// `ValueUtils#toImmutable(Value)`。
    pub fn to_immutable(&self) -> QValue {
        match self {
            QValue::Data(_) => self.clone(),
            QValue::Left(left_value) => QValue::Data(left_value.borrow().get()),
        }
    }

    /// 返回内部左值；不可赋值数据返回 `None`。Rust 便捷方法。
    /// 对应 Java：`value instanceof LeftValue` 后的强制类型转换。
    pub fn as_left(&self) -> Option<&Rc<RefCell<dyn LeftValue>>> {
        match self {
            QValue::Left(left_value) => Some(left_value),
            QValue::Data(_) => None,
        }
    }

    /// 判断是否为 Java `LeftValue`。Rust 便捷方法。
    /// 对应 Java: 无（Rust 原生适配）。
    pub fn is_left(&self) -> bool {
        matches!(self, QValue::Left(_))
    }
}

impl From<DataValue> for QValue {
    fn from(value: DataValue) -> QValue {
        QValue::Data(value)
    }
}

impl fmt::Debug for QValue {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QValue::Data(value) => write!(formatter, "Data({value:?})"),
            QValue::Left(left_value) => {
                write!(formatter, "Left({:?})", left_value.borrow().get())
            }
        }
    }
}
