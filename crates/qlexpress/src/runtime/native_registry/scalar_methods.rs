/// `java.lang.Number` 子类的脚本可用方法子集。
fn number_method(name: &str) -> Option<NativeMethod> {
    use crate::runtime::data::convert::{to_f64, to_i64};
    let f: NativeMethod = match name {
        "intValue" => Rc::new(|bean, _| Ok(DataValue::Int(to_i64(bean) as i32))),
        "longValue" => Rc::new(|bean, _| Ok(DataValue::Long(to_i64(bean)))),
        "doubleValue" => Rc::new(|bean, _| Ok(DataValue::Double(to_f64(bean)))),
        "floatValue" => Rc::new(|bean, _| Ok(DataValue::Float(to_f64(bean) as f32))),
        "shortValue" => Rc::new(|bean, _| Ok(DataValue::Short(to_i64(bean) as i16))),
        "byteValue" => Rc::new(|bean, _| Ok(DataValue::Byte(to_i64(bean) as i8))),
        "compareTo" => Rc::new(|bean, args| match args.first() {
            Some(other) if other.is_number() => {
                let ord = number_compare(bean, other).unwrap_or(std::cmp::Ordering::Equal);
                Ok(DataValue::Int(match ord {
                    std::cmp::Ordering::Less => -1,
                    std::cmp::Ordering::Equal => 0,
                    std::cmp::Ordering::Greater => 1,
                }))
            }
            _ => Err(wrong_args("compareTo")),
        }),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.java_string_value_of()))),
        _ => return None,
    };
    Some(f)
}

/// `java.lang.Boolean` 的脚本可用方法子集。
fn bool_method(name: &str) -> Option<NativeMethod> {
    let f: NativeMethod = match name {
        "booleanValue" => Rc::new(|bean, _| match bean {
            DataValue::Bool(b) => Ok(DataValue::Bool(*b)),
            _ => Err(wrong_args("booleanValue")),
        }),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.java_string_value_of()))),
        _ => return None,
    };
    Some(f)
}
