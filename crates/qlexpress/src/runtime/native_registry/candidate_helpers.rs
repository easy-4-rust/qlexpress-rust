/// 将显式登记的 Java 接口候选转发到对应宿主对象。
///
/// 候选选择已由 [`MemberResolver`] 完成；此处只承担 Java
/// `Method.invoke` 的调用阶段。
fn native_object_method(method_name: &'static str) -> NativeMethod {
    Rc::new(move |bean, args| match bean {
        DataValue::Object(object) => object.borrow_mut().call_method(method_name, args),
        _ => Err(wrong_args(method_name)),
    })
}

/// 返回值的 Java 运行时类型名；宿主对象使用其显式注册的原生类名。
///
/// 对应 Java `bean.getClass().getName()`。
fn native_type_name(bean: &DataValue) -> String {
    match bean {
        DataValue::Object(object) => object.borrow().native_type_name().to_string(),
        _ => bean.data_type_name().to_string(),
    }
}

fn runtime_class_ref(value: &DataValue) -> ClassRef {
    crate::utils::basic_util::BasicUtil::type_of_value(value)
}

fn is_numeric_java_name(name: &str) -> bool {
    matches!(
        name,
        "byte"
            | "java.lang.Byte"
            | "short"
            | "java.lang.Short"
            | "int"
            | "java.lang.Integer"
            | "long"
            | "java.lang.Long"
            | "float"
            | "java.lang.Float"
            | "double"
            | "java.lang.Double"
            | "java.math.BigInteger"
            | "java.math.BigDecimal"
    )
}

/// JDK 内建类层级；Java 侧由 `Class#isAssignableFrom` 提供。
fn builtin_assignable(param_name: &str, arg_name: &str) -> bool {
    if param_name == "java.lang.Number" && is_numeric_java_name(arg_name) {
        return true;
    }
    let boxed_scalar = is_numeric_java_name(arg_name)
        || matches!(arg_name, "java.lang.Boolean" | "java.lang.Character");
    match param_name {
        "java.lang.Comparable" => boxed_scalar || arg_name == "java.lang.String",
        "java.io.Serializable" => {
            boxed_scalar || arg_name == "java.lang.String" || arg_name.starts_with('[')
        }
        "java.lang.CharSequence" => arg_name == "java.lang.String",
        "java.util.List" | "java.util.Collection" | "java.lang.Iterable" => matches!(
            arg_name,
            "java.util.ArrayList" | "java.util.LinkedList" | "java.util.List"
        ),
        "java.util.Set" => matches!(
            arg_name,
            "java.util.HashSet" | "java.util.LinkedHashSet" | "java.util.TreeSet"
        ),
        "java.util.Map" => matches!(
            arg_name,
            "java.util.HashMap" | "java.util.LinkedHashMap" | "java.util.TreeMap"
        ),
        _ => false,
    }
}

fn convert_candidate_arguments(
    arguments: &[DataValue],
    parameter_types: &[ClassRef],
    var_args: bool,
) -> Vec<DataValue> {
    ParametersTypeConvertor::cast(arguments, parameter_types, var_args)
}

fn wrap_method_candidate(candidate: &NativeMethodCandidate) -> NativeMethod {
    let method = Rc::clone(&candidate.method);
    let parameter_types = candidate.parameter_types.clone();
    let var_args = candidate.var_args;
    Rc::new(move |bean, arguments| {
        let converted = convert_candidate_arguments(arguments, &parameter_types, var_args);
        method(bean, &converted)
    })
}

// ---------------------------------------------------------------------------
// 内建方法子集(SPEC §4:String/List/Map/数组/数值/布尔 常用方法,
// 对齐 Java 版脚本可直接调用的 JDK 方法)。
// ---------------------------------------------------------------------------

/// 取第 `i` 个整数参数(Java 侧由反射按 `int` 参数类型自动拆箱转换)。
fn int_arg(args: &[DataValue], i: usize) -> Option<i64> {
    args.get(i).and_then(|v| {
        if v.is_number() {
            Some(crate::runtime::data::convert::to_i64(v))
        } else {
            None
        }
    })
}

/// 严格读取 Java `String` 实参；反射不会把任意对象隐式 `toString()`。
fn string_arg(args: &[DataValue], index: usize) -> Option<&JavaString> {
    match args.get(index) {
        Some(DataValue::Str(value)) => Some(value),
        _ => None,
    }
}

/// Java `String.compareTo` 按 UTF-16 code unit 字典序比较并返回首个差值。
fn java_string_compare_to(left: &JavaString, right: &JavaString) -> i32 {
    left.compare_to(right)
}

/// Java `String.indexOf(String, int)` 的 UTF-16 索引规则。
fn java_string_index_of(value: &JavaString, needle: &JavaString, from_index: i64) -> i32 {
    value.index_of(needle, from_index)
}

/// Java `String.split(regex, limit)`，包括零宽首匹配和尾空串规则。
fn java_regex_split(
    value: &JavaString,
    pattern: &JavaString,
    limit: i32,
) -> Result<DataValue, QLException> {
    let value = value.as_str().ok_or_else(|| {
        QLException::host_error(
            QLExceptionKind::Runtime,
            "regex split cannot cross the UTF-8 host boundary with an unpaired UTF-16 surrogate",
            "java.lang.IllegalArgumentException",
        )
    })?;
    let pattern = pattern.as_str().ok_or_else(|| {
        QLException::host_error(
            QLExceptionKind::Runtime,
            "regex pattern cannot cross the UTF-8 host boundary with an unpaired UTF-16 surrogate",
            "java.util.regex.PatternSyntaxException",
        )
    })?;
    let regex = Regex::new(pattern).map_err(|error| {
        QLException::host_error(
            QLExceptionKind::Runtime,
            error.to_string(),
            "java.util.regex.PatternSyntaxException",
        )
    })?;
    let mut parts = Vec::new();
    let mut previous_end = 0usize;
    for matched in regex.find_iter(value) {
        if limit > 0 && parts.len() >= limit.saturating_sub(1) as usize {
            break;
        }
        // Java Pattern.split:输入开头的零宽匹配不产生前导空元素。
        if matched.start() == 0 && matched.end() == 0 && parts.is_empty() {
            previous_end = matched.end();
            continue;
        }
        parts.push(value[previous_end..matched.start()].to_string());
        previous_end = matched.end();
    }
    parts.push(value[previous_end..].to_string());
    if limit == 0 {
        while parts.last().is_some_and(String::is_empty) {
            parts.pop();
        }
    }
    // Java 对空输入始终返回一个包含空字符串的数组。
    if value.is_empty() && parts.is_empty() {
        parts.push(String::new());
    }
    Ok(DataValue::array_with_component(
        parts.into_iter().map(DataValue::string).collect(),
        ClassRef::from_name("java.lang.String"),
    ))
}
