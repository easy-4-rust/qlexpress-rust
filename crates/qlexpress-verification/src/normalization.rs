//! Java/Rust 差分结果的稳定规范化。

use qlexpress::runtime::value::DataValue;

/// 对应 Java: 无（Rust 原生适配）。
pub fn normalize(value: &DataValue) -> String {
    match value {
        DataValue::Null => "null".to_string(),
        DataValue::Bool(value) => format!("bool:{value}"),
        DataValue::Byte(value) => format!("byte:{value}"),
        DataValue::Short(value) => format!("short:{value}"),
        DataValue::Int(value) => format!("int:{value}"),
        DataValue::Long(value) => format!("long:{value}"),
        DataValue::Float(value) => format!("float:{}", normalize_float(f64::from(*value))),
        DataValue::Double(value) => format!("double:{}", normalize_float(*value)),
        DataValue::BigInt(value) => format!("bigint:{value}"),
        DataValue::BigDec(value) => format!("bigdec:{value}"),
        DataValue::Char(value) => format!("char:{}", escape(&value.to_string())),
        DataValue::Str(value) => format!("string:{}", escape(value)),
        DataValue::List(values) => {
            let values = values.borrow();
            format!(
                "list:[{}]",
                values.iter().map(normalize).collect::<Vec<_>>().join(",")
            )
        }
        DataValue::Array(values) => {
            let values = values.borrow();
            format!(
                "array:[{}]",
                values.iter().map(normalize).collect::<Vec<_>>().join(",")
            )
        }
        DataValue::Map(values) => {
            let values = values.borrow();
            let entries = values
                .entries()
                .iter()
                .map(|(key, value)| format!("{}=>{}", normalize(key), normalize(value)))
                .collect::<Vec<_>>()
                .join(",");
            format!("map:{{{entries}}}")
        }
        DataValue::Lambda(_) => "lambda".to_string(),
        DataValue::Object(value) => format!("object:{}", value.borrow().native_type_name()),
    }
}

fn normalize_float(value: f64) -> String {
    if value.is_nan() {
        "NaN".to_string()
    } else if value == f64::INFINITY {
        "Infinity".to_string()
    } else if value == f64::NEG_INFINITY {
        "-Infinity".to_string()
    } else if value.fract() == 0.0 {
        format!("{value:.1}")
    } else {
        value.to_string()
    }
}

fn escape(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace(',', "\\,")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('{', "\\{")
        .replace('}', "\\}")
}
