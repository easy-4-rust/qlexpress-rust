//! Value model, mirroring Java `Value` / `DataValue` (SPEC §3.1).

use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::qlambda::QLambda;

/// Corresponds to the Java `Value` interface: every value in the script
/// world.
pub trait Value {
    /// Extract the inner data (Java `Value.get()`).
    fn get(&self) -> DataValue;

    /// Java `Value.getTypeName()`.
    fn type_name(&self) -> &'static str;
}

/// Host (native) object stored in [`DataValue::Object`], replacing Java
/// reflection access with explicit registration (SPEC §4/§6).
///
/// Full registry machinery lives in `class_supplier.rs` (later stage).
pub trait NativeObject: std::any::Any {
    /// Java reflective field read.
    fn get_field(&self, name: &str) -> Option<DataValue>;

    /// Java reflective method invocation.
    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException>;

    /// Native type name, used in error messages like the Java class name.
    fn native_type_name(&self) -> &str;

    /// Downcast support (used e.g. by `CastInstruction` to recognise the
    /// `MetaClass` wrapper, Java `instanceof MetaClass`).
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl fmt::Debug for dyn NativeObject {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "NativeObject({})", self.native_type_name())
    }
}

/// Corresponds to Java `DataValue`: a `Value` holding concrete data.
///
/// Variant set fixed by SPEC §3.1. `BigDec` stores a decimal string to keep
/// full precision without external dependencies; `BigInt` approximates
/// `BigInteger` with `i128`.
#[derive(Clone)]
pub enum DataValue {
    /// Java `Value.NULL_VALUE` content.
    Null,
    Bool(bool),
    Byte(i8),
    Short(i16),
    Int(i32),
    Long(i64),
    Float(f32),
    Double(f64),
    /// `BigInteger` approximation (SPEC §3.1).
    BigInt(i128),
    /// `BigDecimal`: decimal string storage, parsed on demand.
    BigDec(String),
    Char(char),
    Str(String),
    /// Mutable list shared by `Rc` so item l-values (`ListItemValue`) alias
    /// the live list, like Java's `ArrayList` references (Stage 3a change;
    /// same pattern as `Map`).
    List(Rc<RefCell<Vec<DataValue>>>),
    /// Ordered map (Java `LinkedHashMap`-like), dependency-free.
    Map(Rc<RefCell<IndexMap>>),
    /// Java array; shared by `Rc` so `ArrayItemValue` aliases live storage.
    Array(Rc<RefCell<Vec<DataValue>>>),
    Lambda(Rc<QLambda>),
    /// Host object (SPEC §6).
    Object(Rc<RefCell<dyn NativeObject>>),
}

impl DataValue {
    /// Java `Value.NULL_VALUE`.
    pub const NULL_VALUE: DataValue = DataValue::Null;

    pub fn is_null(&self) -> bool {
        matches!(self, DataValue::Null)
    }

    /// Whether this value is any numeric kind (Java `value instanceof Number`).
    pub fn is_number(&self) -> bool {
        matches!(
            self,
            DataValue::Byte(_)
                | DataValue::Short(_)
                | DataValue::Int(_)
                | DataValue::Long(_)
                | DataValue::Float(_)
                | DataValue::Double(_)
                | DataValue::BigInt(_)
                | DataValue::BigDec(_)
        )
    }

    /// Java class-style type name used in QLExpress error messages
    /// (Java `Value.getTypeName()`).
    pub fn data_type_name(&self) -> &'static str {
        match self {
            DataValue::Null => "com.alibaba.qlexpress4.runtime.Nothing",
            DataValue::Bool(_) => "java.lang.Boolean",
            DataValue::Byte(_) => "java.lang.Byte",
            DataValue::Short(_) => "java.lang.Short",
            DataValue::Int(_) => "java.lang.Integer",
            DataValue::Long(_) => "java.lang.Long",
            DataValue::Float(_) => "java.lang.Float",
            DataValue::Double(_) => "java.lang.Double",
            DataValue::BigInt(_) => "java.math.BigInteger",
            DataValue::BigDec(_) => "java.math.BigDecimal",
            DataValue::Char(_) => "java.lang.Character",
            DataValue::Str(_) => "java.lang.String",
            DataValue::List(_) => "java.util.ArrayList",
            DataValue::Map(_) => "java.util.LinkedHashMap",
            DataValue::Array(_) => "java.lang.Object[]",
            DataValue::Lambda(_) => "com.alibaba.qlexpress4.runtime.QLambda",
            DataValue::Object(_) => "com.alibaba.qlexpress4.NativeObject",
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            DataValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            DataValue::Str(s) => Some(s),
            _ => None,
        }
    }

    /// Convenience constructor for [`DataValue::List`].
    pub fn list(items: Vec<DataValue>) -> DataValue {
        DataValue::List(Rc::new(RefCell::new(items)))
    }

    /// Convenience constructor for [`DataValue::Array`].
    pub fn array(items: Vec<DataValue>) -> DataValue {
        DataValue::Array(Rc::new(RefCell::new(items)))
    }

    /// Convenience constructor for [`DataValue::Map`].
    pub fn map(map: IndexMap) -> DataValue {
        DataValue::Map(Rc::new(RefCell::new(map)))
    }

    /// Java `String.valueOf`-style rendering used in error messages and
    /// string concatenation (`null` for [`DataValue::Null`]).
    pub fn string_value_of(&self) -> String {
        match self {
            DataValue::Null => "null".to_string(),
            DataValue::Bool(b) => b.to_string(),
            DataValue::Byte(v) => v.to_string(),
            DataValue::Short(v) => v.to_string(),
            DataValue::Int(v) => v.to_string(),
            DataValue::Long(v) => v.to_string(),
            DataValue::Float(v) => {
                if v.fract() == 0.0 && v.is_finite() {
                    format!("{v:.1}")
                } else {
                    v.to_string()
                }
            }
            DataValue::Double(v) => {
                // Java Double.toString: integral values render with ".0".
                if v.fract() == 0.0 && v.is_finite() {
                    format!("{v:.1}")
                } else {
                    v.to_string()
                }
            }
            DataValue::BigInt(v) => v.to_string(),
            DataValue::BigDec(v) => v.clone(),
            DataValue::Char(c) => c.to_string(),
            DataValue::Str(s) => s.clone(),
            other => format!("{other:?}"),
        }
    }
}

impl Value for DataValue {
    fn get(&self) -> DataValue {
        self.clone()
    }

    fn type_name(&self) -> &'static str {
        self.data_type_name()
    }
}

/// `PartialEq` implements the QLExpress equality semantics used by the
/// compare operators: numeric values compare across types after promotion
/// (`1 == 1L == 1.0 == 1.00(BigDecimal)`), containers compare structurally,
/// lambdas/objects compare by identity. (Java implements this inside the
/// operator classes via `NumberMath.compareTo`; centralizing it here keeps
/// `IndexMap` lookups consistent.)
impl PartialEq for DataValue {
    fn eq(&self, other: &Self) -> bool {
        use DataValue::*;
        match (self, other) {
            (Null, Null) => true,
            (Bool(a), Bool(b)) => a == b,
            (Char(a), Char(b)) => a == b,
            (Str(a), Str(b)) => a == b,
            (List(a), List(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Array(a), Array(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Map(a), Map(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Lambda(a), Lambda(b)) => Rc::ptr_eq(a, b),
            (Object(a), Object(b)) => Rc::ptr_eq(a, b),
            _ => {
                if self.is_number() && other.is_number() {
                    crate::runtime::data::convert::number_compare(self, other)
                        == Some(std::cmp::Ordering::Equal)
                } else {
                    false
                }
            }
        }
    }
}

impl fmt::Debug for DataValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use DataValue::*;
        match self {
            Null => write!(f, "Null"),
            Bool(v) => write!(f, "Bool({v})"),
            Byte(v) => write!(f, "Byte({v})"),
            Short(v) => write!(f, "Short({v})"),
            Int(v) => write!(f, "Int({v})"),
            Long(v) => write!(f, "Long({v})"),
            Float(v) => write!(f, "Float({v})"),
            Double(v) => write!(f, "Double({v})"),
            BigInt(v) => write!(f, "BigInt({v})"),
            BigDec(v) => write!(f, "BigDec({v})"),
            Char(v) => write!(f, "Char({v:?})"),
            Str(v) => write!(f, "Str({v:?})"),
            List(v) => f.debug_tuple("List").field(&v.borrow()).finish(),
            Map(v) => f.debug_tuple("Map").field(&v.borrow()).finish(),
            Array(v) => f.debug_tuple("Array").field(&v.borrow()).finish(),
            Lambda(v) => f.debug_tuple("Lambda").field(v).finish(),
            Object(v) => write!(f, "Object({})", v.borrow().native_type_name()),
        }
    }
}

/// One operand-stack element, mirroring the Java `Value` references stored
/// on the QVM stack: either an immutable datum (`DataValue`) or a shared
/// assignable slot (`LeftValue`, e.g. variables, field/item values).
///
/// Java `ValueUtils.toImmutable` becomes [`QValue::to_immutable`].
#[derive(Clone)]
pub enum QValue {
    /// Immutable value (Java `DataValue`).
    Data(DataValue),
    /// Shared assignable value (Java `LeftValue` implementations).
    Left(Rc<RefCell<dyn crate::runtime::left_value::LeftValue>>),
}

impl QValue {
    /// Java `Value.get()`.
    pub fn get(&self) -> DataValue {
        match self {
            QValue::Data(v) => v.clone(),
            QValue::Left(l) => l.borrow().get(),
        }
    }

    /// Java `Value.getTypeName()`.
    pub fn type_name(&self) -> &'static str {
        match self {
            QValue::Data(v) => v.data_type_name(),
            QValue::Left(l) => l.borrow().type_name(),
        }
    }

    /// Java `ValueUtils.toImmutable`: `LeftValue` snapshots to its current
    /// data; anything else is returned unchanged.
    pub fn to_immutable(&self) -> QValue {
        match self {
            QValue::Data(_) => self.clone(),
            QValue::Left(l) => QValue::Data(l.borrow().get()),
        }
    }

    /// The inner `LeftValue`, when this element is assignable.
    pub fn as_left(&self) -> Option<&Rc<RefCell<dyn crate::runtime::left_value::LeftValue>>> {
        match self {
            QValue::Left(l) => Some(l),
            _ => None,
        }
    }

    pub fn is_left(&self) -> bool {
        matches!(self, QValue::Left(_))
    }
}

impl From<DataValue> for QValue {
    fn from(v: DataValue) -> QValue {
        QValue::Data(v)
    }
}

impl fmt::Debug for QValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            QValue::Data(v) => write!(f, "Data({v:?})"),
            QValue::Left(l) => write!(f, "Left({:?})", l.borrow().get()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_equality_promotes_across_types() {
        assert_eq!(DataValue::Int(1), DataValue::Long(1));
        assert_eq!(DataValue::Int(1), DataValue::Double(1.0));
        assert_eq!(DataValue::Byte(1), DataValue::Short(1));
        assert_eq!(DataValue::BigDec("1.00".to_string()), DataValue::Int(1));
        assert_ne!(DataValue::Int(1), DataValue::Int(2));
        assert_ne!(DataValue::Int(1), DataValue::Bool(true));
    }

    #[test]
    fn structural_and_identity_equality() {
        assert_eq!(
            DataValue::list(vec![DataValue::Int(1), DataValue::Str("a".into())]),
            DataValue::list(vec![DataValue::Long(1), DataValue::Str("a".into())])
        );
        assert_eq!(DataValue::Null, DataValue::Null);
        assert_ne!(DataValue::Null, DataValue::Int(0));
        assert_eq!(DataValue::Char('a'), DataValue::Char('a'));
        assert_ne!(DataValue::Char('a'), DataValue::Str("a".into()));
    }

    #[test]
    fn type_names_match_java_class_names() {
        assert_eq!(DataValue::Int(0).type_name(), "java.lang.Integer");
        assert_eq!(
            DataValue::Null.type_name(),
            "com.alibaba.qlexpress4.runtime.Nothing"
        );
        assert_eq!(DataValue::BigInt(0).type_name(), "java.math.BigInteger");
    }
}
