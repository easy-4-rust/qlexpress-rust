//! Minimal executable example used by the project README files.

use std::collections::HashMap;

use qlexpress::{DataValue, Express4Runner, QLOptions};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let runner = Express4Runner::new();
    let options = QLOptions::builder().cache(true).build();

    let mut context = HashMap::new();
    context.insert("price".to_string(), DataValue::Double(125.0));
    context.insert("vip".to_string(), DataValue::Bool(true));

    let result = runner
        .execute("vip ? price * 0.8 : price", context, &options)?
        .into_result();
    assert_eq!(result, DataValue::Double(100.0));

    println!("{}", result.string_value_of());
    Ok(())
}
