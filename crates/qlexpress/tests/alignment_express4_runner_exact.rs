//! Java `Express4RunnerTest` 的逐方法精确迁移测试。
//!
//! 本文件只登记已经按 Java 方法输入与断言逐条复刻的用例；不能用一个
//! 宽泛 smoke test 代替多个 Java 方法。

#![allow(clippy::result_large_err)]

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;
use std::thread;
use std::time::{Duration, Instant};

use qlexpress::aparser::import_manager::QLImport;
use qlexpress::api::parsecache::ConcurrentParseCache;
use qlexpress::check_options::CheckOptions;
use qlexpress::default_class_supplier::DefaultClassSupplier;
use qlexpress::exception::{error_codes, QLException};
use qlexpress::init_options::InitOptions;
use qlexpress::ql_options::{Attachments, QLOptions, SharedAttachments};
use qlexpress::runtime::class_ref::ClassRef;
use qlexpress::runtime::context::{DynamicVariableContext, EmptyContext, ExpressContext};
use qlexpress::runtime::data::index_map::IndexMap;
use qlexpress::runtime::function::{CustomFunction, ExtensionFunction};
use qlexpress::runtime::jvm_i_method::NativeIMethod;
use qlexpress::runtime::meta_class::as_meta_class;
use qlexpress::runtime::native_object::NativeObject;
use qlexpress::runtime::native_type::NativeType;
use qlexpress::runtime::parameters::Parameters;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::value::{DataValue, QValue};
use qlexpress::security::ql_security_strategy::QLSecurityStrategy;
use qlexpress::{Express4Runner, QLFunctionMethod, QLFunctionProvider};

fn assert_integer(value: &DataValue, expected: i64) {
    match value {
        DataValue::Int(actual) => assert_eq!(i64::from(*actual), expected),
        DataValue::Long(actual) => assert_eq!(*actual, expected),
        other => panic!("expected integer {expected}, got {other:?}"),
    }
}

include!("alignment_express4_runner_exact/cache_and_context.rs");
include!("alignment_express4_runner_exact/custom_context_and_types.rs");
include!("alignment_express4_runner_exact/aliases_and_concurrency.rs");
include!("alignment_express4_runner_exact/extensions_and_annotations.rs");
