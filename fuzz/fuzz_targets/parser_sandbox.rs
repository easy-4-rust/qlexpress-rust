#![no_main]

use libfuzzer_sys::fuzz_target;
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::Express4Runner;

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
