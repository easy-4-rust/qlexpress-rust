//! Stage 6 对齐测试:移植 Java `TestSuiteRunner.suiteTest` 的 testsuite 脚本用例。
//!
//! 每个 `#[test]` 对应 `src/test/resources/testsuite/independent/` 下的一个
//! `.ql` 脚本(脚本字符串与 Java 版保持一致,含选项注释头),通过
//! `alignment_util` 中复刻的 assert/assertFalse/assertErrorCode/println 执行。
//!
//! 对应 Java: com.alibaba.qlexpress4.TestSuiteRunner#suiteTest

// 与 lib 一致的架构性豁免:QLException 对齐 Java 单一异常类(见 lib.rs)。
#![allow(clippy::result_large_err)]

mod alignment_util;

use alignment_util::{
    expect_err_code, expect_err_code_with, expect_null, expect_ok, expect_ok_with,
};
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::QLOptions;
use qlexpress::runtime::value::DataValue;
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;

include!("alignment_suite/basics_to_docs.rs");
include!("alignment_suite/loops_and_functions.rs");
include!("alignment_suite/control_and_maps.rs");
include!("alignment_suite/numeric_operators.rs");
include!("alignment_suite/operators_and_scope.rs");
include!("alignment_suite/collections_and_control.rs");
