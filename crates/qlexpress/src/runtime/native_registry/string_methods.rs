/// 参数不匹配错误,对应 Java `QLErrorCodes.INVOKE_METHOD_WITH_WRONG_ARGUMENTS`。
fn wrong_args(method: &str) -> QLException {
    QLException::for_test(
        QLExceptionKind::Runtime,
        format!("invoke method '{method}' with wrong arguments"),
        error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
    )
}

/// 内建方法分派:对应 Java 中 String/List/Map/Number/Boolean 上真实存在、
/// 脚本可直接调用的方法集合。
fn builtin_method(bean: &DataValue, method_name: &str) -> Option<NativeMethod> {
    match bean {
        DataValue::Str(_) => string_method(method_name),
        DataValue::List(_) => list_method(method_name),
        DataValue::Map(_) => map_method(method_name),
        v if v.is_number() => number_method(method_name),
        DataValue::Bool(_) => bool_method(method_name),
        _ => None,
    }
}

/// Java 默认扩展函数分派。`ReflectLoader` 在隔离策略判断之前解析
/// `FilterExtensionFunction` / `MapExtensionFunction`。
fn builtin_extension_method(bean: &DataValue, method_name: &str) -> Option<NativeMethod> {
    if !matches!(bean, DataValue::List(_)) {
        return None;
    }
    match method_name {
        "filter" => Some(Rc::new(|bean, args| {
            crate::runtime::function::FilterExtensionFunction::instance().invoke(bean, args)
        })),
        "map" => Some(Rc::new(|bean, args| {
            crate::runtime::function::MapExtensionFunction::instance().invoke(bean, args)
        })),
        _ => None,
    }
}

/// Java `String.trim()`：只删除两端 code unit `<= U+0020` 的字符。
fn java_string_trim(value: &JavaString) -> JavaString {
    value.trim()
}

/// Java `String.equalsIgnoreCase` 的逐 UTF-16 字符比较。
fn java_string_equals_ignore_case(left: &JavaString, right: &JavaString) -> bool {
    let left_units = left.utf16_units();
    let right_units = right.utf16_units();
    if left_units.len() != right_units.len() {
        return false;
    }
    left_units
        .iter()
        .zip(right_units.iter())
        .all(|(left, right)| java_char_equals_ignore_case(*left, *right))
}

fn java_char_equals_ignore_case(left: u16, right: u16) -> bool {
    if left == right {
        return true;
    }
    let Some(left_char) = char::from_u32(u32::from(left)) else {
        return false;
    };
    let Some(right_char) = char::from_u32(u32::from(right)) else {
        return false;
    };
    let left_upper = simple_uppercase(left_char);
    let right_upper = simple_uppercase(right_char);
    left_upper == right_upper || simple_lowercase(left_upper) == simple_lowercase(right_upper)
}

fn simple_uppercase(value: char) -> char {
    let mut mapped = value.to_uppercase();
    match (mapped.next(), mapped.next()) {
        (Some(single), None) => single,
        _ => value,
    }
}

fn simple_lowercase(value: char) -> char {
    let mut mapped = value.to_lowercase();
    match (mapped.next(), mapped.next()) {
        (Some(single), None) => single,
        _ => value,
    }
}

/// `java.lang.String` 的脚本可用方法集合。
fn string_method(name: &str) -> Option<NativeMethod> {
    let f: NativeMethod = match name {
        "length" => Rc::new(|bean, _| match bean {
            DataValue::Str(s) => Ok(DataValue::Int(s.len() as i32)),
            _ => Err(wrong_args("length")),
        }),
        "isEmpty" => Rc::new(|bean, _| match bean {
            DataValue::Str(s) => Ok(DataValue::Bool(s.is_empty())),
            _ => Err(wrong_args("isEmpty")),
        }),
        "charAt" => Rc::new(|bean, args| match (bean, int_arg(args, 0)) {
            (DataValue::Str(s), Some(i)) if i >= 0 => s
                .char_at(i as usize)
                .map(DataValue::Char)
                .ok_or_else(|| wrong_args("charAt")),
            _ => Err(wrong_args("charAt")),
        }),
        "contains" => Rc::new(|bean, args| match (bean, string_arg(args, 0)) {
            (DataValue::Str(s), Some(sub)) => Ok(DataValue::Bool(s.contains(sub))),
            _ => Err(wrong_args("contains")),
        }),
        "startsWith" => Rc::new(|bean, args| match (bean, string_arg(args, 0), args.len()) {
            (DataValue::Str(s), Some(prefix), 1) => Ok(DataValue::Bool(s.starts_with(prefix))),
            (DataValue::Str(s), Some(prefix), 2) => {
                let Some(offset) = int_arg(args, 1) else {
                    return Err(wrong_args("startsWith"));
                };
                Ok(DataValue::Bool(s.starts_with_at(prefix, offset)))
            }
            _ => Err(wrong_args("startsWith")),
        }),
        "endsWith" => Rc::new(|bean, args| match (bean, string_arg(args, 0)) {
            (DataValue::Str(s), Some(p)) => Ok(DataValue::Bool(s.ends_with(p))),
            _ => Err(wrong_args("endsWith")),
        }),
        "concat" => Rc::new(|bean, args| match (bean, string_arg(args, 0), args.len()) {
            (DataValue::Str(value), Some(suffix), 1) => Ok(DataValue::Str(value.concat(suffix))),
            _ => Err(wrong_args("concat")),
        }),
        "indexOf" => Rc::new(|bean, args| match (bean, string_arg(args, 0), args.len()) {
            (DataValue::Str(s), Some(sub), 1) => {
                Ok(DataValue::Int(java_string_index_of(s, sub, 0)))
            }
            (DataValue::Str(s), Some(sub), 2) => {
                let Some(from_index) = int_arg(args, 1) else {
                    return Err(wrong_args("indexOf"));
                };
                Ok(DataValue::Int(java_string_index_of(s, sub, from_index)))
            }
            _ => Err(wrong_args("indexOf")),
        }),
        "toUpperCase" => Rc::new(|bean, _| match bean {
            DataValue::Str(s) => Ok(DataValue::Str(s.to_uppercase())),
            _ => Err(wrong_args("toUpperCase")),
        }),
        "toLowerCase" => Rc::new(|bean, _| match bean {
            DataValue::Str(s) => Ok(DataValue::Str(s.to_lowercase())),
            _ => Err(wrong_args("toLowerCase")),
        }),
        "trim" => Rc::new(|bean, _| match bean {
            DataValue::Str(s) => Ok(DataValue::Str(java_string_trim(s))),
            _ => Err(wrong_args("trim")),
        }),
        "substring" => Rc::new(|bean, args| match bean {
            DataValue::Str(s) => {
                let Some(begin) = int_arg(args, 0) else {
                    return Err(wrong_args("substring"));
                };
                if begin < 0 {
                    return Err(wrong_args("substring"));
                }
                let end = int_arg(args, 1).unwrap_or(s.len() as i64);
                if end < 0 {
                    return Err(wrong_args("substring"));
                }
                let (begin, end) = (begin as usize, end as usize);
                s.substring(begin, end)
                    .map(DataValue::Str)
                    .ok_or_else(|| wrong_args("substring"))
            }
            _ => Err(wrong_args("substring")),
        }),
        "replace" => Rc::new(
            |bean, args| match (bean, string_arg(args, 0), string_arg(args, 1)) {
                (DataValue::Str(s), Some(from), Some(to)) => {
                    Ok(DataValue::Str(s.replace(from, to)))
                }
                _ => Err(wrong_args("replace")),
            },
        ),
        "equals" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Str(s), Some(DataValue::Str(o))) => Ok(DataValue::Bool(s == o)),
            (DataValue::Str(_), Some(_)) => Ok(DataValue::Bool(false)),
            _ => Err(wrong_args("equals")),
        }),
        "equalsIgnoreCase" => Rc::new(|bean, args| match (bean, string_arg(args, 0)) {
            (DataValue::Str(s), Some(o)) => {
                Ok(DataValue::Bool(java_string_equals_ignore_case(s, o)))
            }
            (DataValue::Str(_), None) if matches!(args, [DataValue::Null]) => {
                Ok(DataValue::Bool(false))
            }
            _ => Err(wrong_args("equalsIgnoreCase")),
        }),
        "compareTo" => Rc::new(|bean, args| match (bean, string_arg(args, 0)) {
            (DataValue::Str(left), Some(right)) if args.len() == 1 => {
                Ok(DataValue::Int(java_string_compare_to(left, right)))
            }
            _ => Err(wrong_args("compareTo")),
        }),
        "split" => Rc::new(|bean, args| match (bean, string_arg(args, 0), args.len()) {
            (DataValue::Str(value), Some(pattern), 1) => java_regex_split(value, pattern, 0),
            (DataValue::Str(value), Some(pattern), 2) => {
                let Some(limit) = int_arg(args, 1) else {
                    return Err(wrong_args("split"));
                };
                java_regex_split(value, pattern, limit as i32)
            }
            _ => Err(wrong_args("split")),
        }),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.java_string_value_of()))),
        _ => return None,
    };
    Some(f)
}
