//! Stage 7: 对齐 Java `docs/CustomItemsDocTest` (7 个 @Test)。
//!
//! addFunction / addOperator / addVarargsFunction 的端到端注册测试。

#![allow(clippy::result_large_err)]

use std::collections::HashMap;
use std::rc::Rc;

use qlexpress::exception::QLException;
use qlexpress::ql_options::QLOptions;
use qlexpress::ql_precedences;
use qlexpress::runtime::class_ref::ClassRef;
use qlexpress::runtime::function::extension_function::ExtensionFunction;
use qlexpress::runtime::parameters::Parameters;
use qlexpress::runtime::qcontext::QContext;
use qlexpress::runtime::value::{DataValue, QValue};
use qlexpress::Express4Runner;

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn run_int(runner: &Express4Runner, script: &str) -> i64 {
    let r = runner
        .execute(script, HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    match r {
        DataValue::Long(n) => n,
        DataValue::Int(n) => n as i64,
        other => panic!("expected int/long, got {other:?}"),
    }
}

// ---------- addFunction variants ----------

// Java source: CustomItemsDocTest#addFunctionWithJavaFunctionalTest
#[test]
fn add_function_with_java_functional_test() {
    let runner = Express4Runner::new();
    runner.add_function_unary("inc", |value: DataValue| {
        DataValue::Long(qlexpress::runtime::data::convert::to_i64(&value) + 1)
    });
    runner.add_function_unary("isPos", |value: DataValue| {
        DataValue::Bool(qlexpress::runtime::data::convert::to_i64(&value) > 0)
    });
    runner.add_function(
        "notify",
        |_ctx: &mut dyn QContext, _params: &Parameters| Ok(DataValue::Null),
    );
    runner.add_function_unary("print", |_value: DataValue| DataValue::Null);

    assert_eq!(run_int(&runner, "inc(1)"), 2);
    assert_eq!(
        runner
            .execute("isPos(1)", HashMap::new(), &opts())
            .expect("predicate")
            .into_result(),
        DataValue::Bool(true)
    );
}

#[test]
fn add_function_with_function_signature() {
    // Java addFunction(String, Function<T, R>)
    let runner = Express4Runner::new();
    runner.add_function(
        "inc",
        |_ctx: &mut dyn QContext,
         params: &Parameters|
         -> Result<DataValue, qlexpress::exception::QLException> {
            let n = qlexpress::runtime::data::convert::to_i64(&params.get_value(0));
            Ok(DataValue::Long(n + 1))
        },
    );
    assert_eq!(run_int(&runner, "inc(1)"), 2);
}

#[test]
fn add_function_with_predicate() {
    // Java addFunction(String, Predicate<T>)
    let runner = Express4Runner::new();
    runner.add_function_unary("is_pos", |v: DataValue| -> DataValue {
        // 接受任意 numeric,通过 to_i64 统一处理
        let n = qlexpress::runtime::data::convert::to_i64(&v);
        DataValue::Bool(n > 0)
    });
    let r = runner
        .execute("is_pos(5)", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Bool(true));
}

#[test]
fn add_function_with_runnable_returns_null() {
    // Java Runnable.run() returns void → null
    let runner = Express4Runner::new();
    runner.add_function(
        "do_nothing",
        |_ctx: &mut dyn QContext,
         _params: &Parameters|
         -> Result<DataValue, qlexpress::exception::QLException> { Ok(DataValue::Null) },
    );
    let r = runner
        .execute("do_nothing()", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Null);
}

#[test]
fn add_function_with_consumer() {
    let runner = Express4Runner::new();
    runner.add_function_unary("double_str", |v: DataValue| -> DataValue {
        let s = v.string_value_of();
        DataValue::Str(format!("{s}{s}"))
    });
    let r = runner
        .execute("double_str(\"ab\")", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("abab".to_string()));
}

// Java source: CustomItemsDocTest#addFunctionByVarargsTest
#[test]
fn add_varargs_function() {
    // Java QLFunctionalVarargs
    let runner = Express4Runner::new();
    runner.add_varargs_function(
        "join",
        |params: &[DataValue]| -> Result<DataValue, qlexpress::exception::QLException> {
            Ok(DataValue::Str(
                params
                    .iter()
                    .map(DataValue::string_value_of)
                    .collect::<Vec<_>>()
                    .join(","),
            ))
        },
    );
    let r = runner
        .execute("join(1,2,3)", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("1,2,3".to_string()));
}

// ---------- addOperator variants ----------

// Java source: CustomItemsDocTest#addOperatorBiFunctionTest
#[test]
fn add_operator_bifunction() {
    // Java addOperatorBiFunction
    let mut runner = Express4Runner::new();
    runner.add_operator_bi("join", |left: DataValue, right: DataValue| {
        DataValue::Str(format!(
            "{},{}",
            left.string_value_of(),
            right.string_value_of()
        ))
    });
    let r = runner
        .execute("1 join 2 join 3", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Str("1,2,3".to_string()));
}

// Java source: CustomItemsDocTest#replaceDefaultOperatorTest
#[test]
fn replace_default_operator() {
    let mut runner = Express4Runner::new();
    let replaced = runner.replace_operator(
        "+",
        Rc::new(|left: &QValue, right: &QValue| {
            let left = left.get().string_value_of().parse::<f64>().unwrap();
            let right = right.get().string_value_of().parse::<f64>().unwrap();
            Ok(DataValue::Double(left + right))
        }),
    );
    assert!(replaced);
    let r = runner
        .execute("'1.2' + '2.3'", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(r, DataValue::Double(3.5));
}

// Java source: CustomItemsDocTest#addOperatorWithPrecedenceTest
#[test]
fn add_operator_with_add_precedence() {
    let mut runner = Express4Runner::new();
    assert!(runner.add_operator_with_precedence(
        "?><",
        Rc::new(|left: &QValue, right: &QValue| {
            Ok(DataValue::Str(format!(
                "{}{}",
                left.get().string_value_of(),
                right.get().string_value_of()
            )))
        }),
        ql_precedences::ADD,
    ));
    let result = runner
        .execute("1 ?>< 2 * 3", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(result, DataValue::Str("16".to_string()));
}

// Java source: CustomItemsDocTest#addOperatorByVarargsTest
#[test]
fn add_operator_by_varargs() {
    let mut runner = Express4Runner::new();
    assert!(runner.add_operator_varargs(
        "join",
        |params: &[DataValue]| -> Result<DataValue, QLException> {
            Ok(DataValue::Str(format!(
                "{},{}",
                params[0].string_value_of(),
                params[1].string_value_of()
            )))
        },
    ));
    let result = runner
        .execute("1 join 2", HashMap::new(), &opts())
        .expect("ok")
        .into_result();
    assert_eq!(result, DataValue::Str("1,2".to_string()));
}

fn sum_values(values: impl IntoIterator<Item = DataValue>) -> f64 {
    values.into_iter().fold(0.0, |sum, value| {
        sum + match value {
            DataValue::Byte(value) => f64::from(value),
            DataValue::Short(value) => f64::from(value),
            DataValue::Int(value) => f64::from(value),
            DataValue::Long(value) => value as f64,
            DataValue::Float(value) => f64::from(value),
            DataValue::Double(value) => value,
            _ => 0.0,
        }
    })
}

struct PlusAll;

impl ExtensionFunction for PlusAll {
    fn parameter_types(&self) -> Vec<ClassRef> {
        vec![ClassRef::Named("java.lang.Object".to_string())]
    }

    fn name(&self) -> &str {
        "plusAll"
    }

    fn declaring_class(&self) -> ClassRef {
        ClassRef::Named("java.lang.Integer".to_string())
    }

    fn invoke(&self, obj: &DataValue, args: &[DataValue]) -> Result<DataValue, QLException> {
        Ok(DataValue::Double(sum_values(
            std::iter::once(obj.clone()).chain(args.iter().cloned()),
        )))
    }
}

// Java source: CustomItemsDocTest#qlfunctionalvarargsAllInOneTest
#[test]
fn qlfunctional_varargs_all_in_one() {
    let mut runner = Express4Runner::new();
    runner.add_varargs_function(
        "sumAll",
        |params: &[DataValue]| -> Result<DataValue, QLException> {
            Ok(DataValue::Double(sum_values(params.iter().cloned())))
        },
    );
    assert!(runner.add_operator_varargs(
        "+&",
        |params: &[DataValue]| -> Result<DataValue, QLException> {
            Ok(DataValue::Double(sum_values(params.iter().cloned())))
        },
    ));
    runner.add_extend_function(PlusAll);

    assert_eq!(
        runner
            .execute("sumAll(1,2,3)", HashMap::new(), &opts())
            .expect("function")
            .into_result(),
        DataValue::Double(6.0)
    );
    assert_eq!(
        runner
            .execute("1 +& 4", HashMap::new(), &opts())
            .expect("operator")
            .into_result(),
        DataValue::Double(5.0)
    );
    assert_eq!(
        runner
            .execute("1.plusAll(5)", HashMap::new(), &opts())
            .expect("extension")
            .into_result(),
        DataValue::Double(6.0)
    );
}
