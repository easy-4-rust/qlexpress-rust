//! 对齐 Java `com.alibaba.qlexpress4.runtime.MemberResolverTest`。

use std::collections::HashMap;

use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

// Java source: MemberResolverTest#resolveStreamTest
// ADAPTED: Rust 没有 JDK 的隐藏 Stream 实现类反射；Stream 接口候选显式
// 注册到 NativeType。除验证候选存在外，还通过真实 QVM 调用执行 filter。
#[test]
fn stream_interface_method_is_resolved_and_invoked_at_runtime() {
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    let stream_type = runner
        .registry()
        .get_type("java.util.stream.Stream")
        .expect("Stream interface registration");
    assert_eq!(
        stream_type
            .method_candidates
            .get("filter")
            .expect("filter candidates")
            .len(),
        1
    );

    let result = runner
        .execute(
            concat!(
                "list = new ArrayList(); ",
                "list.add(1); list.add(2); list.add(3); ",
                "list.stream().filter(x -> x > 1).collect(Collectors.toList())"
            ),
            HashMap::new(),
            &QLOptions::default(),
        )
        .expect("Stream.filter must resolve through the runtime")
        .into_result();
    assert_eq!(
        result,
        DataValue::list(vec![DataValue::Int(2), DataValue::Int(3)])
    );
}
