//! 订单风控业务宿主集成验收。

use std::collections::HashMap;

use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::member::QLExpressNativeType;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::{Express4Runner, QLExpressType};

#[derive(QLExpressType)]
#[qlexpress(name = "com.example.risk.Order")]
struct Order {
    order_id: String,
    amount: f64,
    customer_level: i64,
    device_risk_score: i64,
    overseas: bool,
}

/// 对应 Java: 无（Rust 原生适配）。
pub fn run() -> Result<(), String> {
    let mut runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::open())
            .build(),
    );
    runner.register_qlexpress_type::<Order>();
    let rule = r#"
        base = order.amount >= 1000.0 ? 30 : 0;
        device = order.device_risk_score >= 80 ? 50 : 0;
        region = order.overseas ? 25 : 0;
        trusted = order.customer_level >= 4 ? -20 : 0;
        score = base + device + region + trusted;
        {'orderId': order.order_id, 'score': score, 'decision': score >= 60 ? 'REJECT' : (score >= 30 ? 'REVIEW' : 'PASS')}
    "#;
    let cases = [
        (
            Order {
                order_id: "O-1001".to_string(),
                amount: 199.0,
                customer_level: 5,
                device_risk_score: 10,
                overseas: false,
            },
            "PASS",
        ),
        (
            Order {
                order_id: "O-1002".to_string(),
                amount: 1500.0,
                customer_level: 2,
                device_risk_score: 40,
                overseas: false,
            },
            "REVIEW",
        ),
        (
            Order {
                order_id: "O-1003".to_string(),
                amount: 1800.0,
                customer_level: 1,
                device_risk_score: 95,
                overseas: true,
            },
            "REJECT",
        ),
    ];
    for (order, expected) in cases {
        let order_id = order.order_id.clone();
        let mut context = HashMap::new();
        context.insert("order".to_string(), order.into_data_value());
        let result = runner
            .execute(rule, context, &QLOptions::builder().cache(true).build())
            .map_err(|error| format!("business order {order_id}: {error}"))?
            .into_result();
        let DataValue::Map(result) = result else {
            return Err(format!("business order {order_id}: expected map result"));
        };
        let decision = result
            .borrow()
            .get(&DataValue::Str("decision".to_string()))
            .cloned();
        if decision != Some(DataValue::Str(expected.to_string())) {
            return Err(format!(
                "business order {order_id}: expected {expected}, got {decision:?}"
            ));
        }
    }
    println!(
        "{{\"host\":\"order-risk\",\"cases\":3,\"passed\":3,\"registry_type\":\"com.example.risk.Order\"}}"
    );
    Ok(())
}
