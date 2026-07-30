//! 对齐 Java `com.alibaba.qlexpress4.runtime.ValueInitializationTest`。

use std::sync::{mpsc, Arc, Barrier};
use std::time::Duration;

use qlexpress::runtime::nothing::NOTHING_TYPE_NAME;
use qlexpress::runtime::value::{DataValue, QValue};

// Java source: ValueInitializationTest#nullValueDoesNotDependOnDataValue
// ADAPTED: Java 用非 DataValue 的匿名 Value 避免 JVM 类初始化循环；Rust
// 的 DataValue 是无类初始化过程的枚举，Null 变体本身就是无依赖常量。
#[test]
fn null_value_is_dependency_free_enum_constant() {
    const NULL_VALUE: DataValue = DataValue::NULL_VALUE;

    assert_eq!(NULL_VALUE, DataValue::Null);
    assert!(NULL_VALUE.is_null());
    assert_eq!(NULL_VALUE.data_type_name(), NOTHING_TYPE_NAME);
    assert_eq!(QValue::Data(NULL_VALUE).get(), DataValue::Null);
}

// Java source: ValueInitializationTest#valueAndDataValueCanInitializeConcurrently
// ADAPTED: Rust 没有 Class.forName/JVM <clinit>；两个线程同时首次构造
// QValue 与 DataValue，并在 Java 测试相同的两秒期限内完成，锁定无初始化死锁。
#[test]
fn value_and_data_value_construct_concurrently() {
    let barrier = Arc::new(Barrier::new(2));
    let (sender, receiver) = mpsc::channel();

    let value_barrier = Arc::clone(&barrier);
    let value_sender = sender.clone();
    let value_thread = std::thread::spawn(move || {
        value_barrier.wait();
        let value = QValue::Data(DataValue::NULL_VALUE);
        value_sender
            .send(value.get().is_null())
            .expect("send value");
    });

    let data_barrier = Arc::clone(&barrier);
    let data_thread = std::thread::spawn(move || {
        data_barrier.wait();
        let value = DataValue::Int(1);
        sender
            .send(value.data_type_name() == "java.lang.Integer")
            .expect("send data value");
    });

    assert!(receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("first initialization must finish"));
    assert!(receiver
        .recv_timeout(Duration::from_secs(2))
        .expect("second initialization must finish"));
    value_thread.join().expect("value thread");
    data_thread.join().expect("data value thread");
}
