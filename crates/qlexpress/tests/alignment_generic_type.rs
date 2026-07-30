//! 对齐 Java `com.alibaba.qlexpress4.generic.GenericTypeTest`。

/// Java 内部类 `GenericTypeTest.Class4GenericType` 的 Rust 静态类型映射。
struct Class4GenericType {
    field1: Vec<String>,
    field2: String,
}

impl Class4GenericType {
    fn method1(&self, _long_list: Vec<i64>) -> Vec<String> {
        Vec::new()
    }
}

fn accepts_string_list(_value: &Vec<String>) {}

fn accepts_string(_value: &String) {}

fn accepts_method_signature(_method: fn(&Class4GenericType, Vec<i64>) -> Vec<String>) {}

// Java source: GenericTypeTest#test
// ADAPTED: Java 通过反射读取擦除前的泛型签名；Rust 泛型由编译器静态校验，
// 不保留等价的 JVM ParameterizedType。以下断言在编译期锁定同一字段和方法签名。
#[test]
fn generic_signatures_are_checked_statically() {
    let fixture = Class4GenericType {
        field1: vec!["ql".to_string()],
        field2: "express".to_string(),
    };

    accepts_string_list(&fixture.field1);
    accepts_string(&fixture.field2);
    accepts_method_signature(Class4GenericType::method1);
    assert_eq!(fixture.method1(vec![1_i64]), Vec::<String>::new());
}
