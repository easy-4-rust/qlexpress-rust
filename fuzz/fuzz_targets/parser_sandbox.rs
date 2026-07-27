#![no_main]

use libfuzzer_sys::fuzz_target;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress_rust::Express4Runner;

fuzz_target!(|data: &[u8]| {
    if data.len() > 16 * 1024 {
        return;
    }
    let script = String::from_utf8_lossy(data);
    let runner = Express4Runner::with_init_options(
        InitOptions::builder()
            .security_strategy(QLSecurityStrategy::isolation())
            .build(),
    );
    let options = QLOptions::builder()
        .timeout_millis(5)
        .max_arr_length(1_024)
        .build();
    let _ = runner.execute(&script, Default::default(), &options);
});
