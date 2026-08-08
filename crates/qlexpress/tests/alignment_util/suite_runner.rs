/// 对应 Java `TestSuiteRunner.prepareRunner`:注册四个测试函数,
/// 并使用开放安全策略 + JDK 类型供应器(同 `handleFile`)。
pub fn suite_runner() -> Express4Runner {
    suite_runner_with_init_options(jdk_init_options())
}

/// 使用指定初始化选项创建套件 Runner，同时注册 Java 套件的四个函数。
///
/// 对应 Java `TestSuiteRunner#prepareRunner(InitOptions)`；供 debug 与
/// 自定义初始化策略测试复用相同宿主装配逻辑。
pub fn suite_runner_with_init_options(init_options: InitOptions) -> Express4Runner {
    let allow_private_access = init_options.is_allow_private_access();
    let mut runner = Express4Runner::with_init_options(init_options);
    let mut person = NativeType::named("com.alibaba.qlexpress4.inport.Person");
    person.constructor = Some(Rc::new(|args| {
        let [age] = args else {
            return Err(biz_error("Person constructor expects one age"));
        };
        if !age.is_number() {
            return Err(biz_error("Person constructor expects numeric age"));
        }
        let object: Rc<RefCell<dyn NativeObject>> = Rc::new(RefCell::new(TestPerson {
            age: qlexpress::runtime::data::convert::to_i64(age),
        }));
        Ok(DataValue::Object(object))
    }));
    runner.register_native_type(person);
    // 对应 Java testsuite import fixture 中的 `Sample.value == 1` 与
    // `Sample1.value == 10`。Java 通过反射读取 public static 字段；Rust
    // 必须显式注册同名 capability，避免把测试夹具变成隐式反射白名单。
    let mut sample = NativeType::named("com.alibaba.qlexpress4.inport.Sample");
    sample
        .static_fields
        .insert("value".to_string(), DataValue::Int(1));
    runner.register_native_type(sample);
    let mut sample1 = NativeType::named("com.alibaba.qlexpress4.inport.Sample1");
    sample1
        .static_fields
        .insert("value".to_string(), DataValue::Int(10));
    runner.register_native_type(sample1);
    // Java Map 字面量的 `@class` 分类实例化：未知字段由
    // NewFilledInstanceInstruction 忽略，已登记字段经显式 NativeObject 写入。
    for (type_name, methods) in [
        (
            "com.alibaba.qlexpress4.inport.MyHome",
            [
                "getSofa",
                "setSofa",
                "getChair",
                "setChair",
                "getMyDesk",
                "setMyDesk",
                "getBed",
                "",
            ],
        ),
        (
            "com.alibaba.qlexpress4.inport.MyDesk",
            [
                "getBook1", "setBook1", "getBook2", "setBook2", "", "", "", "",
            ],
        ),
    ] {
        let mut classified = NativeType::named(type_name);
        classified.constructor = Some(Rc::new(move |args| match args {
            [] => Ok(DataValue::Object(Rc::new(RefCell::new(ClassifiedObject {
                type_name,
                fields: HashMap::new(),
            })))),
            _ => Err(biz_error(format!("{type_name} constructor arguments"))),
        }));
        for method in methods.into_iter().filter(|method| !method.is_empty()) {
            classified.methods.insert(
                method.to_string(),
                Rc::new(move |bean, args| match bean {
                    DataValue::Object(object) => object.borrow_mut().call_method(method, args),
                    _ => Err(biz_error(format!("{method} receiver"))),
                }),
            );
        }
        if type_name == "com.alibaba.qlexpress4.inport.MyHome" {
            // Java MyHome.bed 有 getter 但没有 setter；分类构造命中它时不是
            // “未知字段可忽略”，而是必须报 INVALID_ASSIGNMENT。
            classified.fields.insert(
                "bed".to_string(),
                Rc::new(|bean| match bean {
                    DataValue::Object(object) => object.borrow().get_field("bed"),
                    _ => None,
                }),
            );
        }
        runner.register_native_type(classified);
    }
    // 对应 Java `HelloParent`/`HelloChild`/`HelloConstructor` 测试夹具的
    // 全部公开构造器。候选签名交给 NativeRegistry 按 Java 可赋值性与
    // 可变参数优先级选择，不能以 if/else 伪造重载结果。
    let parent_name = "com.alibaba.qlexpress4.test.constructor.HelloParent";
    let child_name = "com.alibaba.qlexpress4.test.constructor.HelloChild";
    let hello_name = "com.alibaba.qlexpress4.test.constructor.HelloConstructor";
    let mut parent = NativeType::named(parent_name);
    parent.constructor = Some(Rc::new(move |args| match args {
        [] => Ok(OpaqueNativeObject::new(parent_name).into_data_value()),
        _ => Err(biz_error("HelloParent constructor arguments")),
    }));
    runner.register_native_type(parent);
    let mut child = NativeType::named(child_name);
    child.supertypes.push(parent_name.to_string());
    child.constructor = Some(Rc::new(move |args| match args {
        [] => Ok(OpaqueNativeObject::new(child_name).into_data_value()),
        _ => Err(biz_error("HelloChild constructor arguments")),
    }));
    runner.register_native_type(child);
    runner.register_native_type(NativeType::interface("java.lang.Runnable", ["run"]));
    runner.register_native_type(NativeType::interface(
        "com.alibaba.qlexpress4.test.lambda.UserFunctionalInterface",
        ["lala"],
    ));
    let mut hello = NativeType::named(hello_name);
    hello.add_constructor_candidate(constructor_candidate(
        vec![named_class(parent_name)],
        false,
        0,
    ));
    hello.add_constructor_candidate(constructor_candidate(
        vec![named_class(child_name)],
        false,
        1,
    ));
    hello.add_constructor_candidate(constructor_candidate(
        vec![ClassRef::array_of(named_class("java.lang.String"))],
        true,
        2,
    ));
    hello.add_constructor_candidate(constructor_candidate(
        vec![named_class("java.lang.String")],
        false,
        3,
    ));
    hello.add_constructor_candidate(constructor_candidate(
        vec![named_class(child_name), named_class("java.lang.Runnable")],
        false,
        4,
    ));
    hello.add_constructor_candidate(constructor_candidate(
        vec![
            named_class(parent_name),
            named_class("com.alibaba.qlexpress4.runtime.QLambda"),
        ],
        false,
        5,
    ));
    hello.fields.insert(
        "flag".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("flag"),
            _ => None,
        }),
    );
    runner.register_native_type(hello);
    let sample_property_name = "com.alibaba.qlexpress4.test.property.Sample";
    let mut property_sample = NativeType::named(sample_property_name);
    property_sample.constructor = Some(Rc::new(|args| match args {
        [value] if value.is_number() => Ok(DataValue::Object(Rc::new(RefCell::new(
            PropertySampleObject {
                count: qlexpress::runtime::data::convert::to_i32(value),
            },
        )))),
        _ => Err(biz_error("Sample constructor arguments")),
    }));
    property_sample.methods.insert(
        "getCount".to_string(),
        Rc::new(|bean, args| match bean {
            DataValue::Object(object) => object.borrow_mut().call_method("getCount", args),
            _ => Err(biz_error("Sample.getCount receiver")),
        }),
    );
    property_sample.methods.insert(
        "setCount".to_string(),
        Rc::new(|bean, args| match bean {
            DataValue::Object(object) => object.borrow_mut().call_method("setCount", args),
            _ => Err(biz_error("Sample.setCount receiver")),
        }),
    );
    // Java `Sample.count` 是 private，但 ReflectLoader 通过 getCount/setCount
    // 暴露同名属性；Rust 将该 JavaBean 属性显式登记为可读写字段能力。
    property_sample.fields.insert(
        "count".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("count"),
            _ => None,
        }),
    );
    property_sample.field_setters.insert(
        "count".to_string(),
        Rc::new(|bean, value| match bean {
            DataValue::Object(object) => object.borrow_mut().set_field("count", value),
            _ => false,
        }),
    );
    runner.register_native_type(property_sample);
    let sample_enum_name = "com.alibaba.qlexpress4.test.property.SampleEnum";
    let normal_enum = DataValue::Object(Rc::new(RefCell::new(SampleEnumObject)));
    let unsupported_enum = DataValue::Object(Rc::new(RefCell::new(SampleEnumObject)));
    let mut sample_enum = NativeType::named(sample_enum_name);
    sample_enum
        .static_fields
        .insert("NORMAL".to_string(), normal_enum);
    sample_enum
        .static_fields
        .insert("UN_SUPPORT".to_string(), unsupported_enum);
    sample_enum
        .static_fields
        .insert("testStaticField".to_string(), DataValue::Int(1000));
    sample_enum.fields.insert(
        "testField".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("testField"),
            _ => None,
        }),
    );
    sample_enum.methods.insert(
        "equals".to_string(),
        Rc::new(|bean, args| match (bean, args) {
            (DataValue::Object(left), [DataValue::Object(right)]) => {
                Ok(DataValue::Bool(Rc::ptr_eq(left, right)))
            }
            (DataValue::Object(_), [_]) => Ok(DataValue::Bool(false)),
            _ => Err(biz_error("SampleEnum.equals receiver")),
        }),
    );
    runner.register_native_type(sample_enum);
    let mut some_inter = NativeType::named("com.alibaba.qlexpress4.test.property.SomeInter");
    some_inter
        .static_fields
        .insert("INTER_CONST_1".to_string(), DataValue::string("test1"));
    runner.register_native_type(some_inter);
    // Java `Parent.birth` 同时可经 public field 与 JavaBean getter/setter
    // 访问；这里只登记原脚本需要的同一能力面。
    let parent_property_name = "com.alibaba.qlexpress4.test.property.Parent";
    let mut parent_property = NativeType::named(parent_property_name);
    parent_property.constructor = Some(Rc::new(|args| match args {
        [] => Ok(DataValue::Object(Rc::new(RefCell::new(ParentObject {
            birth: DataValue::string("2022-01-01"),
            lock_status: 0,
            lock_status2: DataValue::Null,
        })))),
        _ => Err(biz_error("Parent constructor arguments")),
    }));
    parent_property.fields.insert(
        "birth".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("birth"),
            _ => None,
        }),
    );
    parent_property.field_setters.insert(
        "birth".to_string(),
        Rc::new(|bean, value| match bean {
            DataValue::Object(object) => object.borrow_mut().set_field("birth", value),
            _ => false,
        }),
    );
    for field_name in ["lockStatus", "lockStatus2"] {
        parent_property.fields.insert(
            field_name.to_string(),
            Rc::new(move |bean| match bean {
                DataValue::Object(object) => object.borrow().get_field(field_name),
                _ => None,
            }),
        );
        parent_property.field_setters.insert(
            field_name.to_string(),
            Rc::new(move |bean, value| match bean {
                DataValue::Object(object) => object.borrow_mut().set_field(field_name, value),
                _ => false,
            }),
        );
    }
    for method in ["getBirth", "setBirth"] {
        parent_property.methods.insert(
            method.to_string(),
            Rc::new(move |bean, args| match bean {
                DataValue::Object(object) => object.borrow_mut().call_method(method, args),
                _ => Err(biz_error("Parent method receiver")),
            }),
        );
    }
    runner.register_native_type(parent_property);
    // fastjson2 JSONObject 保留自身运行时类型，同时显式暴露 Java Map 风格
    // 的 `put/get`，避免将声明类型静默降级为 LinkedHashMap。
    let mut json_object = NativeType::named("com.alibaba.fastjson2.JSONObject");
    json_object.constructor = Some(Rc::new(|args| match args {
        [] => Ok(DataValue::Object(Rc::new(RefCell::new(JsonObject {
            entries: qlexpress::runtime::data::index_map::IndexMap::new(),
        })))),
        _ => Err(biz_error("JSONObject constructor arguments")),
    }));
    for method in ["put", "get"] {
        json_object.methods.insert(
            method.to_string(),
            Rc::new(move |bean, args| match bean {
                DataValue::Object(object) => object.borrow_mut().call_method(method, args),
                _ => Err(biz_error("JSONObject receiver")),
            }),
        );
    }
    runner.register_native_type(json_object);
    let mut json = NativeType::named("com.alibaba.fastjson2.JSON");
    json.static_methods.insert(
        "toJSONString".to_string(),
        Rc::new(|_bean, args| match args {
            [value] => Ok(DataValue::string(value.string_value_of())),
            _ => Err(biz_error("JSON.toJSONString arguments")),
        }),
    );
    runner.register_native_type(json);
    let mut sample_set = NativeType::named("com.alibaba.qlexpress4.test.property.SampleSet");
    sample_set.constructor = Some(Rc::new(|args| match args {
        [] => Ok(DataValue::Object(Rc::new(RefCell::new(CountObject {
            type_name: "com.alibaba.qlexpress4.test.property.SampleSet",
            count: 0,
        })))),
        _ => Err(biz_error("SampleSet constructor arguments")),
    }));
    sample_set.fields.insert(
        "count".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("count"),
            _ => None,
        }),
    );
    sample_set.field_setters.insert(
        "count".to_string(),
        Rc::new(|bean, value| match bean {
            DataValue::Object(object) => object.borrow_mut().set_field("count", value),
            _ => false,
        }),
    );
    runner.register_native_type(sample_set);
    // 私有成员不是 Rust 反射能力；仅当 Java 对应选项打开时显式登记，未打开
    // 的 runner 不能看见该字段，保持 `FIELD_NOT_FOUND`。
    let mut private_sample =
        NativeType::named("com.alibaba.qlexpress4.test.property.SampleForPrivate");
    private_sample.constructor = Some(Rc::new(|args| match args {
        [value] if value.is_number() => Ok(DataValue::Object(Rc::new(RefCell::new(CountObject {
            type_name: "com.alibaba.qlexpress4.test.property.SampleForPrivate",
            count: qlexpress::runtime::data::convert::to_i32(value),
        })))),
        _ => Err(biz_error("SampleForPrivate constructor arguments")),
    }));
    if allow_private_access {
        private_sample.fields.insert(
            "count".to_string(),
            Rc::new(|bean| match bean {
                DataValue::Object(object) => object.borrow().get_field("count"),
                _ => None,
            }),
        );
        private_sample.field_setters.insert(
            "count".to_string(),
            Rc::new(|bean, value| match bean {
                DataValue::Object(object) => object.borrow_mut().set_field("count", value),
                _ => false,
            }),
        );
    }
    runner.register_native_type(private_sample);
    let mut test_enum = NativeType::named("com.alibaba.qlexpress4.test.property.TestEnum");
    test_enum.static_fields.insert(
        "SKT".to_string(),
        DataValue::Object(Rc::new(RefCell::new(CountObject {
            type_name: "com.alibaba.qlexpress4.test.property.TestEnum",
            count: -1,
        }))),
    );
    test_enum.fields.insert(
        "value".to_string(),
        Rc::new(|bean| match bean {
            DataValue::Object(object) => object.borrow().get_field("count"),
            _ => None,
        }),
    );
    runner.register_native_type(test_enum);
    let child_type_name = "com.alibaba.qlexpress4.test.method.TestChild";
    let mut test_child = NativeType::named(child_type_name);
    test_child.supertypes.extend([
        "com.alibaba.qlexpress4.test.method.TestParent".to_string(),
        "com.alibaba.qlexpress4.test.method.InterWithDefault".to_string(),
    ]);
    test_child.constructor = Some(Rc::new(|args| match args {
        [] => Ok(DataValue::Object(Rc::new(RefCell::new(TestChildObject)))),
        _ => Err(biz_error("TestChild constructor arguments")),
    }));
    for method in ["get10", "get1", "get100"] {
        test_child.methods.insert(
            method.to_string(),
            Rc::new(move |bean, args| match bean {
                DataValue::Object(object) => object.borrow_mut().call_method(method, args),
                _ => Err(biz_error("TestChild method receiver")),
            }),
        );
    }
    runner.register_native_type(test_child);
    let mut stream_test_type = NativeType::named("com.alibaba.qlexpress4.test.stream.STObject");
    stream_test_type.constructor = Some(Rc::new(|args| match args {
        [DataValue::Str(payload)] => {
            Ok(DataValue::Object(Rc::new(RefCell::new(StreamTestObject {
                payload: DataValue::Str(payload.clone()),
            }))))
        }
        _ => Err(biz_error("STObject constructor arguments")),
    }));
    stream_test_type.methods.insert(
        "getPayload".to_string(),
        Rc::new(|bean, args| match bean {
            DataValue::Object(object) => object.borrow_mut().call_method("getPayload", args),
            _ => Err(biz_error("STObject.getPayload receiver")),
        }),
    );
    runner.register_native_type(stream_test_type);
    runner.add_function("assert", assert_function);
    runner.add_function("assertFalse", assert_false_function);
    runner.add_function("assertErrorCode", assert_error_code_function);
    runner.add_function("println", print_function);
    runner
}
