/// Java 测试夹具 `com.alibaba.qlexpress4.inport.Person` 的 Rust 对等对象。
struct TestPerson {
    age: i64,
}

/// Java 测试夹具 `HelloConstructor` 的 Rust 对等对象，仅暴露 Java 中的
/// `public int flag`，用于验证构造器重载选择结果。
struct FlagObject {
    flag: i32,
}

/// Java 测试夹具 `test.property.Sample` 的 Rust 等价对象。
struct PropertySampleObject {
    count: i32,
}

/// Java `SampleEnum.NORMAL` / `UN_SUPPORT` 的 Rust 测试夹具实例。
struct SampleEnumObject;

/// Java 测试夹具 `property.Parent`：只承载本套件实际使用的出生日期属性。
struct ParentObject {
    birth: DataValue,
    lock_status: i32,
    lock_status2: DataValue,
}

/// Java 测试夹具 `property.SampleSet` 的公开 `count` 字段。
struct CountObject {
    type_name: &'static str,
    count: i32,
}

/// Java 测试夹具 `method.TestChild` 的方法分派对象。
struct TestChildObject;

/// Java 测试夹具 `stream.STObject` 的不可变 payload。
struct StreamTestObject {
    payload: DataValue,
}

/// Java 分类 Map 目标 `MyHome` / `MyDesk` 的字段容器。
struct ClassifiedObject {
    type_name: &'static str,
    fields: HashMap<String, DataValue>,
}

/// Java fastjson2 `JSONObject`：测试所需的字符串键 `put/get` 有序对象。
struct JsonObject {
    entries: qlexpress::runtime::data::index_map::IndexMap,
}

impl NativeObject for SampleEnumObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "testField").then_some(DataValue::Int(10))
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(biz_error(format!("SampleEnum method not found: {name}")))
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.property.SampleEnum"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for ParentObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        match name {
            "birth" => Some(self.birth.clone()),
            "lockStatus" => Some(DataValue::Int(self.lock_status)),
            "lockStatus2" => Some(self.lock_status2.clone()),
            _ => None,
        }
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        match (name, value) {
            ("birth", value) if value.is_null() || matches!(value, DataValue::Str(_)) => {
                self.birth = value.clone();
                true
            }
            ("lockStatus", value) if value.is_number() => {
                self.lock_status = qlexpress::runtime::data::convert::to_i32(value);
                true
            }
            ("lockStatus2", value) if value.is_null() || value.is_number() => {
                self.lock_status2 = value.clone();
                true
            }
            _ => false,
        }
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("getBirth", []) => Ok(self.birth.clone()),
            ("setBirth", [value]) if value.is_null() || matches!(value, DataValue::Str(_)) => {
                self.birth = value.clone();
                Ok(DataValue::Null)
            }
            _ => Err(biz_error(format!("Parent method not found: {name}"))),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.property.Parent"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for CountObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "count").then_some(DataValue::Int(self.count))
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        if name == "count" && value.is_number() {
            self.count = qlexpress::runtime::data::convert::to_i32(value);
            true
        } else {
            false
        }
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(biz_error(format!(
            "{} method not found: {name}",
            self.type_name
        )))
    }

    fn native_type_name(&self) -> &str {
        self.type_name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for TestChildObject {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("get10", []) => Ok(DataValue::Int(10)),
            ("get10", [DataValue::Str(_)]) => Ok(DataValue::Int(11)),
            ("get1", []) => Ok(DataValue::Int(1)),
            ("get100", []) => Ok(DataValue::Int(100)),
            _ => Err(biz_error(format!("TestChild method not found: {name}"))),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.method.TestChild"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for StreamTestObject {
    fn get_field(&self, _name: &str) -> Option<DataValue> {
        None
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("getPayload", []) => Ok(self.payload.clone()),
            _ => Err(biz_error(format!("STObject method not found: {name}"))),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.stream.STObject"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for ClassifiedObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        if matches!(
            name,
            "sofa" | "chair" | "myDesk" | "bed" | "book1" | "book2"
        ) {
            Some(self.fields.get(name).cloned().unwrap_or(DataValue::Null))
        } else {
            None
        }
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        if matches!(name, "sofa" | "chair" | "myDesk" | "book1" | "book2") {
            self.fields.insert(name.to_string(), value.clone());
            true
        } else {
            false
        }
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        let field = match (name, args) {
            ("getSofa", []) => "sofa",
            ("getChair", []) => "chair",
            ("getMyDesk", []) => "myDesk",
            ("getBed", []) => "bed",
            ("getBook1", []) => "book1",
            ("getBook2", []) => "book2",
            ("setSofa", [value]) => {
                self.fields.insert("sofa".to_string(), value.clone());
                return Ok(DataValue::Null);
            }
            ("setChair", [value]) => {
                self.fields.insert("chair".to_string(), value.clone());
                return Ok(DataValue::Null);
            }
            ("setMyDesk", [value]) => {
                self.fields.insert("myDesk".to_string(), value.clone());
                return Ok(DataValue::Null);
            }
            ("setBook1", [value]) => {
                self.fields.insert("book1".to_string(), value.clone());
                return Ok(DataValue::Null);
            }
            ("setBook2", [value]) => {
                self.fields.insert("book2".to_string(), value.clone());
                return Ok(DataValue::Null);
            }
            _ => {
                return Err(biz_error(format!(
                    "{} method not found: {name}",
                    self.type_name
                )))
            }
        };
        Ok(self.fields.get(field).cloned().unwrap_or(DataValue::Null))
    }

    fn native_type_name(&self) -> &str {
        self.type_name
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for JsonObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        self.entries.get(&DataValue::string(name)).cloned()
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        self.entries.insert(DataValue::string(name), value.clone());
        true
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("put", [DataValue::Str(key), value]) => Ok(self
                .entries
                .insert(DataValue::Str(key.clone()), value.clone())
                .unwrap_or(DataValue::Null)),
            ("get", [DataValue::Str(key)]) => Ok(self
                .entries
                .get(&DataValue::Str(key.clone()))
                .cloned()
                .unwrap_or(DataValue::Null)),
            _ => Err(biz_error(format!("JSONObject method not found: {name}"))),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.fastjson2.JSONObject"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for PropertySampleObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "count").then_some(DataValue::Int(self.count))
    }

    fn set_field(&mut self, name: &str, value: &DataValue) -> bool {
        if name == "count" && value.is_number() {
            self.count = qlexpress::runtime::data::convert::to_i32(value);
            return true;
        }
        false
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        match (name, args) {
            ("getCount", []) => Ok(DataValue::Int(self.count)),
            ("setCount", [value]) if value.is_number() => {
                self.count = qlexpress::runtime::data::convert::to_i32(value);
                Ok(DataValue::Null)
            }
            _ => Err(biz_error(format!("Sample method not found: {name}"))),
        }
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.property.Sample"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

impl NativeObject for FlagObject {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "flag").then_some(DataValue::Int(self.flag))
    }

    fn call_method(&mut self, name: &str, _args: &[DataValue]) -> Result<DataValue, QLException> {
        Err(biz_error(format!(
            "HelloConstructor method not found: {name}"
        )))
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.test.constructor.HelloConstructor"
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

fn flag_object(flag: i32) -> DataValue {
    DataValue::Object(Rc::new(RefCell::new(FlagObject { flag })))
}

fn named_class(name: &str) -> ClassRef {
    ClassRef::Named(name.to_string())
}

fn constructor_candidate(
    parameter_types: Vec<ClassRef>,
    var_args: bool,
    flag: i32,
) -> NativeConstructorCandidate {
    NativeConstructorCandidate::new(
        parameter_types,
        var_args,
        Rc::new(move |_args| Ok(flag_object(flag))),
    )
}

impl NativeObject for TestPerson {
    fn get_field(&self, name: &str) -> Option<DataValue> {
        (name == "age").then_some(DataValue::Long(self.age))
    }

    fn call_method(&mut self, name: &str, args: &[DataValue]) -> Result<DataValue, QLException> {
        if name == "compareTo" {
            let Some(DataValue::Object(other)) = args.first() else {
                return Err(biz_error("Person.compareTo expects Person"));
            };
            let borrowed = other.borrow();
            let Some(other) = borrowed.as_any().downcast_ref::<TestPerson>() else {
                return Err(biz_error("Person.compareTo expects Person"));
            };
            return Ok(DataValue::Int(match self.age.cmp(&other.age) {
                Ordering::Less => -1,
                Ordering::Equal => 0,
                Ordering::Greater => 1,
            }));
        }
        Err(biz_error(format!("Person method not found: {name}")))
    }

    fn native_type_name(&self) -> &str {
        "com.alibaba.qlexpress4.inport.Person"
    }

    fn is_comparable(&self) -> bool {
        true
    }

    fn compare_to(&self, other: &dyn NativeObject) -> Option<Ordering> {
        other
            .as_any()
            .downcast_ref::<TestPerson>()
            .map(|other| self.age.cmp(&other.age))
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
