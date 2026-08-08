/// `java.util.List` 的脚本可用方法子集。
fn list_method(name: &str) -> Option<NativeMethod> {
    let f: NativeMethod = match name {
        "size" => Rc::new(|bean, _| match bean {
            DataValue::List(l) => Ok(DataValue::Int(l.borrow().len() as i32)),
            _ => Err(wrong_args("size")),
        }),
        "isEmpty" => Rc::new(|bean, _| match bean {
            DataValue::List(l) => Ok(DataValue::Bool(l.borrow().is_empty())),
            _ => Err(wrong_args("isEmpty")),
        }),
        "parallelStream" | "stream" => Rc::new(|bean, args| match bean {
            DataValue::List(items) if args.is_empty() => {
                Ok(JavaStream::new(items.borrow().to_vec()).into_data_value())
            }
            _ => Err(wrong_args("parallelStream")),
        }),
        "get" => Rc::new(|bean, args| match (bean, int_arg(args, 0)) {
            (DataValue::List(l), Some(i)) => {
                let list = l.borrow();
                let idx = if i < 0 { i + list.len() as i64 } else { i };
                list.get(idx as usize).cloned().ok_or_else(|| {
                    QLException::host_error(
                        QLExceptionKind::Runtime,
                        format!("Index {i} out of bounds for length {}", list.len()),
                        error_codes::INDEX_OUT_BOUND,
                    )
                })
            }
            _ => Err(wrong_args("get")),
        }),
        "add" => Rc::new(|bean, args| match bean {
            DataValue::List(l) => {
                l.borrow_mut()
                    .push(args.first().cloned().unwrap_or(DataValue::Null));
                Ok(DataValue::Bool(true))
            }
            _ => Err(wrong_args("add")),
        }),
        "set" => Rc::new(|bean, args| match (bean, int_arg(args, 0), args.get(1)) {
            (DataValue::List(l), Some(i), Some(v)) => {
                let mut list = l.borrow_mut();
                let idx = if i < 0 { i + list.len() as i64 } else { i } as usize;
                if idx >= list.len() {
                    return Err(wrong_args("set"));
                }
                Ok(list.set(idx, v.clone()))
            }
            _ => Err(wrong_args("set")),
        }),
        "remove" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::List(l), Some(index)) if index.is_number() => {
                let mut list = l.borrow_mut();
                let i = crate::runtime::data::convert::to_i64(index);
                let idx = if i < 0 { i + list.len() as i64 } else { i } as usize;
                if idx >= list.len() {
                    return Err(wrong_args("remove"));
                }
                Ok(list.remove(idx))
            }
            (DataValue::List(l), Some(target)) => {
                let mut list = l.borrow_mut();
                match list.iter().position(|v| v == target) {
                    Some(pos) => {
                        list.remove(pos);
                        Ok(DataValue::Bool(true))
                    }
                    None => Ok(DataValue::Bool(false)),
                }
            }
            _ => Err(wrong_args("remove")),
        }),
        "contains" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::List(l), Some(target)) => {
                Ok(DataValue::Bool(l.borrow().iter().any(|v| v == target)))
            }
            _ => Err(wrong_args("contains")),
        }),
        "indexOf" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::List(l), Some(target)) => Ok(DataValue::Int(
                l.borrow()
                    .iter()
                    .position(|v| v == target)
                    .map(|p| p as i32)
                    .unwrap_or(-1),
            )),
            _ => Err(wrong_args("indexOf")),
        }),
        "addAll" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::List(l), Some(DataValue::List(other))) => {
                let items = other.borrow().to_vec();
                l.borrow_mut().extend(items);
                Ok(DataValue::Bool(true))
            }
            _ => Err(wrong_args("addAll")),
        }),
        "clear" => Rc::new(|bean, _| match bean {
            DataValue::List(l) => {
                l.borrow_mut().clear();
                Ok(DataValue::Null)
            }
            _ => Err(wrong_args("clear")),
        }),
        "subList" => Rc::new(
            |bean, args| match (bean, int_arg(args, 0), int_arg(args, 1)) {
                (DataValue::List(l), Some(a), Some(b)) => {
                    let list = l.borrow();
                    let (a, b) = (a.max(0) as usize, (b.max(0) as usize).min(list.len()));
                    if a > b {
                        return Err(wrong_args("subList"));
                    }
                    Ok(DataValue::list(list[a..b].to_vec()))
                }
                _ => Err(wrong_args("subList")),
            },
        ),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.java_string_value_of()))),
        _ => return None,
    };
    Some(f)
}

/// `java.util.Map` 的脚本可用方法子集。
fn map_method(name: &str) -> Option<NativeMethod> {
    let f: NativeMethod = match name {
        "entrySet" => Rc::new(|bean, args| match bean {
            DataValue::Map(map) if args.is_empty() => Ok(DataValue::list(
                map.borrow()
                    .entries()
                    .iter()
                    .map(|(key, value)| {
                        JavaMapEntry::new(key.clone(), value.clone()).into_data_value()
                    })
                    .collect(),
            )),
            _ => Err(wrong_args("entrySet")),
        }),
        "size" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => Ok(DataValue::Int(m.borrow().len() as i32)),
            _ => Err(wrong_args("size")),
        }),
        "isEmpty" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => Ok(DataValue::Bool(m.borrow().is_empty())),
            _ => Err(wrong_args("isEmpty")),
        }),
        "get" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Map(m), Some(k)) => {
                Ok(m.borrow().get(k).cloned().unwrap_or(DataValue::Null))
            }
            _ => Err(wrong_args("get")),
        }),
        "put" => Rc::new(|bean, args| match (bean, args.first(), args.get(1)) {
            (DataValue::Map(m), Some(k), Some(v)) => Ok(m
                .borrow_mut()
                .insert(k.clone(), v.clone())
                .unwrap_or(DataValue::Null)),
            _ => Err(wrong_args("put")),
        }),
        "containsKey" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Map(m), Some(k)) => Ok(DataValue::Bool(m.borrow().contains_key(k))),
            _ => Err(wrong_args("containsKey")),
        }),
        "containsValue" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Map(m), Some(v)) => {
                Ok(DataValue::Bool(m.borrow().values().any(|value| value == v)))
            }
            _ => Err(wrong_args("containsValue")),
        }),
        "remove" => Rc::new(|bean, args| match (bean, args.first()) {
            (DataValue::Map(m), Some(k)) => Ok(m.borrow_mut().remove(k).unwrap_or(DataValue::Null)),
            _ => Err(wrong_args("remove")),
        }),
        "keySet" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => Ok(DataValue::list(m.borrow().keys().cloned().collect())),
            _ => Err(wrong_args("keySet")),
        }),
        "values" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => Ok(DataValue::list(m.borrow().values().cloned().collect())),
            _ => Err(wrong_args("values")),
        }),
        "clear" => Rc::new(|bean, _| match bean {
            DataValue::Map(m) => {
                m.borrow_mut().clear();
                Ok(DataValue::Null)
            }
            _ => Err(wrong_args("clear")),
        }),
        "toString" => Rc::new(|bean, _| Ok(DataValue::Str(bean.java_string_value_of()))),
        _ => return None,
    };
    Some(f)
}
