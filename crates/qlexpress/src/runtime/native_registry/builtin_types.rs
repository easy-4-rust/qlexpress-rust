impl NativeRegistry {
    /// 注册内建锚点类型,供 `ClassSupplier` 式查询与宿主覆盖挂接;
    /// 实际分派在 [`builtin_method`]。对应 Java 中这些 JDK 类天然可被反射。
    fn register_builtin_types(&self) {
        let mut system = NativeType::named("java.lang.System");
        system.static_methods.insert(
            "currentTimeMillis".to_string(),
            Rc::new(|_bean, args| {
                if args.is_empty() {
                    Ok(DataValue::Long(current_time_millis()))
                } else {
                    Err(wrong_args("System.currentTimeMillis"))
                }
            }),
        );
        self.register_type(system);

        let mut array_list = NativeType::named("java.util.ArrayList");
        array_list.constructor = Some(Rc::new(|args| match args {
            [] => Ok(DataValue::list(Vec::new())),
            [capacity] if capacity.is_number() => {
                let capacity = crate::runtime::data::convert::to_i64(capacity);
                if capacity < 0 {
                    Err(wrong_args("ArrayList"))
                } else {
                    Ok(DataValue::List(Rc::new(RefCell::new(
                        JavaArrayList::with_capacity(capacity as usize),
                    ))))
                }
            }
            _ => Err(wrong_args("ArrayList")),
        }));
        self.register_type(array_list);

        let map_constructor = Rc::new(|args: &[DataValue]| {
            if args.is_empty() {
                Ok(DataValue::Map(Rc::new(RefCell::new(IndexMap::new()))))
            } else {
                Err(wrong_args("HashMap"))
            }
        });
        let mut hash_map = NativeType::named("java.util.HashMap");
        hash_map.supertypes = vec!["java.util.Map".to_string(), "java.lang.Object".to_string()];
        hash_map.constructor = Some(map_constructor.clone());
        self.register_type(hash_map);

        let mut linked_hash_map = NativeType::named("java.util.LinkedHashMap");
        linked_hash_map.supertypes = vec!["java.util.HashMap".to_string()];
        linked_hash_map.constructor = Some(map_constructor);
        self.register_type(linked_hash_map);

        let mut collectors = NativeType::named("java.util.stream.Collectors");
        collectors.static_methods.insert(
            "toList".to_string(),
            Rc::new(|_bean, args| {
                if args.is_empty() {
                    Ok(JavaCollector.into_data_value())
                } else {
                    Err(wrong_args("Collectors.toList"))
                }
            }),
        );
        self.register_type(collectors);

        // Java 流的实际实现类通常不是公开的 Stream 接口本身；
        // Java MemberResolver 会沿接口查找 filter/map/collect。Rust 用
        // Stream 锚点类型登记同名候选，让真实调用现场经过
        // MemberResolver 的 Lambda/精确类型匹配后再进入宿主对象。
        let mut stream = NativeType::named("java.util.stream.Stream");
        stream.add_method_candidate(
            "filter",
            NativeMethodCandidate::new(
                vec![ClassRef::Named("java.util.function.Predicate".to_string())],
                false,
                native_object_method("filter"),
            ),
        );
        stream.add_method_candidate(
            "map",
            NativeMethodCandidate::new(
                vec![ClassRef::Named("java.util.function.Function".to_string())],
                false,
                native_object_method("map"),
            ),
        );
        stream.add_method_candidate(
            "collect",
            NativeMethodCandidate::new(
                vec![ClassRef::Named("java.util.stream.Collector".to_string())],
                false,
                native_object_method("collect"),
            ),
        );
        self.register_type(stream);

        let mut hash_set = NativeType::named("java.util.HashSet");
        hash_set.constructor = Some(Rc::new(|args| {
            if args.is_empty() {
                Ok(OpaqueNativeObject::new("java.util.HashSet").into_data_value())
            } else {
                Err(wrong_args("HashSet"))
            }
        }));
        self.register_type(hash_set);

        let mut integer = NativeType::named("java.lang.Integer");
        integer
            .static_fields
            .insert("MAX_VALUE".to_string(), DataValue::Int(i32::MAX));
        integer
            .static_fields
            .insert("MIN_VALUE".to_string(), DataValue::Int(i32::MIN));
        integer.constructor = Some(Rc::new(|args| match args {
            [value] if value.is_number() => Ok(DataValue::Int(
                crate::runtime::data::convert::to_i64(value) as i32,
            )),
            _ => Err(wrong_args("Integer")),
        }));
        // Java `Integer` 的常用静态数值 API。它们既可通过 `Integer.max(...)`
        // 直接调用，也必须可由 `Integer::max` 方法引用取得。
        integer.static_methods.insert(
            "max".to_string(),
            Rc::new(|_bean, args| match args {
                [left, right] if left.is_number() && right.is_number() => Ok(DataValue::Int(
                    crate::runtime::data::convert::to_i32(left)
                        .max(crate::runtime::data::convert::to_i32(right)),
                )),
                _ => Err(wrong_args("Integer.max")),
            }),
        );
        integer.static_methods.insert(
            "min".to_string(),
            Rc::new(|_bean, args| match args {
                [left, right] if left.is_number() && right.is_number() => Ok(DataValue::Int(
                    crate::runtime::data::convert::to_i32(left)
                        .min(crate::runtime::data::convert::to_i32(right)),
                )),
                _ => Err(wrong_args("Integer.min")),
            }),
        );
        integer.static_methods.insert(
            "compare".to_string(),
            Rc::new(|_bean, args| match args {
                [left, right] if left.is_number() && right.is_number() => {
                    let compared = crate::runtime::data::convert::to_i32(left)
                        .cmp(&crate::runtime::data::convert::to_i32(right));
                    Ok(DataValue::Int(match compared {
                        std::cmp::Ordering::Less => -1,
                        std::cmp::Ordering::Equal => 0,
                        std::cmp::Ordering::Greater => 1,
                    }))
                }
                _ => Err(wrong_args("Integer.compare")),
            }),
        );
        integer.static_methods.insert(
            "sum".to_string(),
            Rc::new(|_bean, args| match args {
                [left, right] if left.is_number() && right.is_number() => Ok(DataValue::Int(
                    crate::runtime::data::convert::to_i32(left)
                        .wrapping_add(crate::runtime::data::convert::to_i32(right)),
                )),
                _ => Err(wrong_args("Integer.sum")),
            }),
        );
        self.register_type(integer);

        let mut long = NativeType::named("java.lang.Long");
        long.static_fields
            .insert("MAX_VALUE".to_string(), DataValue::Long(i64::MAX));
        long.static_fields
            .insert("MIN_VALUE".to_string(), DataValue::Long(i64::MIN));
        long.constructor = Some(Rc::new(|args| match args {
            [value] if value.is_number() => Ok(DataValue::Long(
                crate::runtime::data::convert::to_i64(value),
            )),
            _ => Err(wrong_args("Long")),
        }));
        self.register_type(long);

        let mut big_integer = NativeType::named("java.math.BigInteger");
        big_integer.static_methods.insert(
            "valueOf".to_string(),
            Rc::new(|_bean, args| match args {
                [value] if value.is_number() => Ok(DataValue::BigInt(num_bigint::BigInt::from(
                    crate::runtime::data::convert::to_i64(value),
                ))),
                _ => Err(wrong_args("BigInteger.valueOf")),
            }),
        );
        self.register_type(big_integer);

        // `BigDecimal` 既是规则语言的数值域，也必须作为 Java 风格原生类型
        // 暴露构造器和实例 `divide`；后者保持 JDK 无舍入精确除法的异常语义。
        let mut big_decimal = NativeType::named("java.math.BigDecimal");
        big_decimal.constructor = Some(Rc::new(|args| match args {
            [value] if value.is_number() || matches!(value, DataValue::Str(_)) => Ok(
                DataValue::BigDec(crate::runtime::data::convert::to_big_dec_string(value)),
            ),
            _ => Err(wrong_args("BigDecimal")),
        }));
        big_decimal.methods.insert(
            "divide".to_string(),
            Rc::new(|bean, args| match args {
                [right] if right.is_number() => BigDecimalMath::divide_exact_method(bean, right)
                    .map_err(|error| {
                        // 对应 Java 反射调用目标抛出的 ArithmeticException：
                        // 错误码、cause 和 catchObj 都必须保留给 try/catch。
                        QLException::host_error(
                            QLExceptionKind::Runtime,
                            error.reason(),
                            error.error_code(),
                        )
                        .with_catch_obj(
                            OpaqueNativeObject::new("java.lang.ArithmeticException")
                                .into_data_value(),
                        )
                    }),
                _ => Err(wrong_args("BigDecimal.divide")),
            }),
        );
        self.register_type(big_decimal);

        for exception_name in [
            "java.lang.Exception",
            "java.lang.RuntimeException",
            "java.lang.NullPointerException",
            "java.lang.ArithmeticException",
        ] {
            let mut exception_type = NativeType::named(exception_name);
            exception_type.constructor = Some(Rc::new(move |args| {
                if args.is_empty() || matches!(args, [DataValue::Str(_)]) {
                    Ok(OpaqueNativeObject::new(exception_name).into_data_value())
                } else {
                    Err(wrong_args(exception_name))
                }
            }));
            self.register_type(exception_type);
        }

        for name in ["java.lang.String", "java.lang.Double", "java.lang.Boolean"] {
            self.register_type(NativeType::named(name));
        }
    }
}
