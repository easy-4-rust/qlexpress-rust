//! Stage 4a 操作符合约测试:数值/位/比较/逻辑/instanceof 操作符经
//! `BinaryOperator::execute` 完整路径的语义对齐(对齐 Java QLExpress4)。
//!
//! 覆盖任务要求的关键语义:int/long 溢出回绕、浮点除零得 Infinity、
//! BigDecimal 除法精度与舍入、== 跨数值类型、位运算掩码、&&/|| 真值规则。

use std::cell::RefCell;
use std::rc::Rc;

use qlexpress_rust::aparser::operator_factory::OperatorFactory as _;
use qlexpress_rust::exception::error_codes;
use qlexpress_rust::exception::pure_err_reporter::PureErrReporter;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::runtime::data::assignable_data_value::AssignableDataValue;
use qlexpress_rust::runtime::delegate_qcontext::DelegateQContext;
use qlexpress_rust::runtime::member::{ClassRef, MetaClass, NativeRegistry};
use qlexpress_rust::runtime::operator::arithmetic::*;
use qlexpress_rust::runtime::operator::base::{BinaryOperator, UnaryOperator};
use qlexpress_rust::runtime::operator::bit::*;
use qlexpress_rust::runtime::operator::compare::*;
use qlexpress_rust::runtime::operator::instance_of_operator::InstanceOfOperator;
use qlexpress_rust::runtime::operator::logic::*;
use qlexpress_rust::runtime::operator::operator_manager::OperatorManager;
use qlexpress_rust::runtime::qvm_global_scope::QvmGlobalScope;
use qlexpress_rust::runtime::qvm_runtime::QvmRuntime;
use qlexpress_rust::runtime::scope::QScope;
use qlexpress_rust::runtime::value::{DataValue, QValue};

fn ctx() -> DelegateQContext {
    let global_scope = QScope::global(QvmGlobalScope::empty());
    let instruction_scope = QScope::block_fresh_stack(&global_scope, Default::default(), 16);
    DelegateQContext::new(
        Rc::new(QvmRuntime::for_test(Rc::new(
            NativeRegistry::with_builtins(),
        ))),
        instruction_scope,
    )
}

fn opts() -> QLOptions {
    QLOptions::builder().build()
}

fn precise_opts() -> QLOptions {
    QLOptions::builder().precise(true).build()
}

fn v(d: DataValue) -> QValue {
    QValue::Data(d)
}

fn bin(op: &dyn BinaryOperator, l: DataValue, r: DataValue) -> DataValue {
    op.execute(
        &v(l),
        &v(r),
        &mut ctx(),
        &opts(),
        &PureErrReporter::INSTANCE,
    )
    .unwrap()
}

fn bin_err(
    op: &dyn BinaryOperator,
    l: DataValue,
    r: DataValue,
) -> qlexpress_rust::exception::QLException {
    op.execute(
        &v(l),
        &v(r),
        &mut ctx(),
        &opts(),
        &PureErrReporter::INSTANCE,
    )
    .unwrap_err()
}

fn left(d: DataValue) -> QValue {
    QValue::Left(Rc::new(RefCell::new(AssignableDataValue::new("x", d))))
}

// ---------- arithmetic ----------

#[test]
fn plus_adds_numbers_and_concats_strings() {
    let op = PlusOperator::get_instance();
    assert_eq!(
        bin(&op, DataValue::Int(1), DataValue::Int(2)),
        DataValue::Int(3)
    );
    // int 溢出回绕(Java int 语义,不提升)。
    assert_eq!(
        bin(&op, DataValue::Int(i32::MAX), DataValue::Int(1)),
        DataValue::Int(i32::MIN)
    );
    // int + long → long 提升。
    assert_eq!(
        bin(&op, DataValue::Int(1), DataValue::Long(1)),
        DataValue::Long(2)
    );
    // 字符串拼接。
    assert_eq!(
        bin(&op, DataValue::Str("a".into()), DataValue::Int(1)),
        DataValue::Str("a1".into())
    );
    assert_eq!(
        bin(&op, DataValue::Null, DataValue::Str("x".into())),
        DataValue::Str("nullx".into())
    );
    // 非法操作数。
    assert_eq!(
        bin_err(&op, DataValue::Bool(true), DataValue::Int(1)).error_code(),
        error_codes::INVALID_BINARY_OPERAND
    );
}

#[test]
fn minus_multiply_basic() {
    assert_eq!(
        bin(
            &MinusOperator::get_instance(),
            DataValue::Int(5),
            DataValue::Int(8)
        ),
        DataValue::Int(-3)
    );
    // 'a' - 1 = 96(字符按码点)。
    assert_eq!(
        bin(
            &MinusOperator::get_instance(),
            DataValue::Char('a'),
            DataValue::Int(1)
        ),
        DataValue::Int(96)
    );
    assert_eq!(
        bin(
            &MultiplyOperator::get_instance(),
            DataValue::Int(3),
            DataValue::Int(4)
        ),
        DataValue::Int(12)
    );
    // long 溢出回绕。
    assert_eq!(
        bin(
            &MultiplyOperator::get_instance(),
            DataValue::Long(i64::MAX),
            DataValue::Long(2)
        ),
        DataValue::Long(-2)
    );
}

#[test]
fn divide_semantics_aligned_with_java() {
    let op = DivideOperator::get_instance();
    // 整型相除 → BigDecimal(7/2 = 3.5)。
    assert_eq!(
        bin(&op, DataValue::Int(7), DataValue::Int(2)),
        DataValue::BigDec("3.5".into())
    );
    // 非终止小数 → scale 10,HALF_UP。
    assert_eq!(
        bin(&op, DataValue::Int(1), DataValue::Int(3)),
        DataValue::BigDec("0.3333333333".into())
    );
    assert_eq!(
        bin(&op, DataValue::Int(2), DataValue::Int(3)),
        DataValue::BigDec("0.6666666667".into())
    );
    // 浮点除零 → Infinity(不 panic)。
    assert_eq!(
        bin(&op, DataValue::Double(1.0), DataValue::Int(0)),
        DataValue::Double(f64::INFINITY)
    );
    // 整型除零 → INVALID_ARITHMETIC。
    let err = bin_err(&op, DataValue::Int(1), DataValue::Int(0));
    assert_eq!(err.error_code(), error_codes::INVALID_ARITHMETIC);
}

#[test]
fn remainder_sign_and_float() {
    let op = RemainderOperator::get_instance("%").unwrap();
    // 符号跟被除数(Java % 语义)。
    assert_eq!(
        bin(&op, DataValue::Int(-7), DataValue::Int(3)),
        DataValue::Int(-1)
    );
    assert_eq!(
        bin(&op, DataValue::Long(7), DataValue::Int(-3)),
        DataValue::Long(1)
    );
    // 浮点取余。
    assert_eq!(
        bin(&op, DataValue::Double(5.5), DataValue::Int(2)),
        DataValue::Double(1.5)
    );
    // 未注册词素。
    assert!(RemainderOperator::get_instance("mod").is_none());
}

#[test]
fn assign_operators_write_back() {
    let l = left(DataValue::Int(1));
    let r = PlusAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(2)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(r, DataValue::Int(3));
    assert_eq!(l.get(), DataValue::Int(3)); // 写回左值

    let l = left(DataValue::Int(10));
    let r = MinusAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(4)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(r, DataValue::Int(6));
    assert_eq!(l.get(), DataValue::Int(6));

    let l = left(DataValue::Int(3));
    MultiplyAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(4)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(l.get(), DataValue::Int(12));

    let l = left(DataValue::Int(7));
    DivideAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(2)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(l.get(), DataValue::BigDec("3.5".into()));

    let l = left(DataValue::Int(7));
    RemainderAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(3)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(l.get(), DataValue::Int(1));

    // 非左值 → INVALID_ASSIGNMENT。
    let err = PlusAssignOperator::get_instance()
        .execute(
            &v(DataValue::Int(1)),
            &v(DataValue::Int(2)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap_err();
    assert_eq!(err.error_code(), error_codes::INVALID_ASSIGNMENT);
}

#[test]
fn precise_mode_uses_big_decimal() {
    // precise:0.1 + 0.2 = 0.3(精确十进制)。
    let r = PlusOperator::get_instance()
        .execute(
            &v(DataValue::Double(0.1)),
            &v(DataValue::Double(0.2)),
            &mut ctx(),
            &precise_opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(r, DataValue::BigDec("0.3".into()));
}

// ---------- bit ----------

#[test]
fn bitwise_ops_on_numbers_and_booleans() {
    assert_eq!(
        bin(
            &BitwiseAndOperator::get_instance(),
            DataValue::Int(0b110),
            DataValue::Int(0b101)
        ),
        DataValue::Int(0b100)
    );
    assert_eq!(
        bin(
            &BitwiseOrOperator::get_instance(),
            DataValue::Int(0b110),
            DataValue::Long(0b101)
        ),
        DataValue::Long(0b111)
    );
    assert_eq!(
        bin(
            &BitwiseXorOperator::get_instance(),
            DataValue::Int(0b110),
            DataValue::Int(0b101)
        ),
        DataValue::Int(0b011)
    );
    // Boolean 操作数按逻辑位运算;null 视为 false。
    assert_eq!(
        bin(
            &BitwiseAndOperator::get_instance(),
            DataValue::Bool(true),
            DataValue::Null
        ),
        DataValue::Bool(false)
    );
    assert_eq!(
        bin(
            &BitwiseOrOperator::get_instance(),
            DataValue::Bool(false),
            DataValue::Bool(true)
        ),
        DataValue::Bool(true)
    );
    assert_eq!(
        bin(
            &BitwiseXorOperator::get_instance(),
            DataValue::Bool(true),
            DataValue::Bool(true)
        ),
        DataValue::Bool(false)
    );
}

#[test]
fn shift_mask_and_sign_extension() {
    // int 移位距离掩 31:1 << 32 == 1。
    assert_eq!(
        bin(
            &BitwiseLeftShiftOperator::get_instance(),
            DataValue::Int(1),
            DataValue::Int(32)
        ),
        DataValue::Int(1)
    );
    // 算术右移符号扩展:-8 >> 1 == -4。
    assert_eq!(
        bin(
            &BitwiseRightShiftOperator::get_instance(),
            DataValue::Int(-8),
            DataValue::Int(1)
        ),
        DataValue::Int(-4)
    );
    // 逻辑右移补零:-1 >>> 1 == 0x7FFFFFFF。
    assert_eq!(
        bin(
            &BitwiseRightShiftUnsignedOperator::get_instance(),
            DataValue::Int(-1),
            DataValue::Int(1)
        ),
        DataValue::Int(0x7FFF_FFFF)
    );
    // long 域:-1L >>> 1 == i64::MAX。
    assert_eq!(
        bin(
            &BitwiseRightShiftUnsignedOperator::get_instance(),
            DataValue::Long(-1),
            DataValue::Int(1)
        ),
        DataValue::Long(i64::MAX)
    );
    // 移位距离为浮点 → 报错(Java UnsupportedOperationException)。
    assert!(bin_err(
        &BitwiseLeftShiftOperator::get_instance(),
        DataValue::Int(1),
        DataValue::Double(1.0)
    )
    .reason()
    .contains("Shift distance must be an integral type"));
}

#[test]
fn bitwise_assign_variants() {
    let l = left(DataValue::Int(0b110));
    BitwiseAndAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(0b101)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(l.get(), DataValue::Int(0b100));

    let l = left(DataValue::Int(1));
    BitwiseOrAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(2)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(l.get(), DataValue::Int(3));

    let l = left(DataValue::Int(3));
    BitwiseXorAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(1)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(l.get(), DataValue::Int(2));

    let l = left(DataValue::Int(1));
    BitwiseLeftShiftAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(3)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(l.get(), DataValue::Int(8));

    let l = left(DataValue::Int(-8));
    BitwiseRightShiftAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(1)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(l.get(), DataValue::Int(-4));

    let l = left(DataValue::Int(-1));
    BitwiseRightShiftUnsignedAssignOperator::get_instance()
        .execute(
            &l,
            &v(DataValue::Int(1)),
            &mut ctx(),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(l.get(), DataValue::Int(0x7FFF_FFFF));
}

#[test]
fn bitwise_invert_unary() {
    let op = BitwiseInvertOperator::get_instance();
    assert_eq!(
        op.execute(&v(DataValue::Int(0)), &PureErrReporter::INSTANCE)
            .unwrap(),
        DataValue::Int(-1)
    );
    assert_eq!(op.operator(), "~");
}

// ---------- compare ----------

#[test]
fn equal_unequal_across_numeric_types() {
    let eq = EqualOperator::get_instance();
    // 跨数值类型:1 == 1L == 1.0 == 1.00(BigDecimal)。
    assert_eq!(
        bin(&eq, DataValue::Int(1), DataValue::Long(1)),
        DataValue::Bool(true)
    );
    assert_eq!(
        bin(&eq, DataValue::Long(1), DataValue::Double(1.0)),
        DataValue::Bool(true)
    );
    assert_eq!(
        bin(&eq, DataValue::BigDec("1.00".into()), DataValue::Int(1)),
        DataValue::Bool(true)
    );
    // 'a' == 97(字符按码点)。
    assert_eq!(
        bin(&eq, DataValue::Char('a'), DataValue::Int(97)),
        DataValue::Bool(true)
    );
    assert_eq!(
        bin(&eq, DataValue::Null, DataValue::Null),
        DataValue::Bool(true)
    );
    assert_eq!(
        bin(&eq, DataValue::Null, DataValue::Int(1)),
        DataValue::Bool(false)
    );

    let ne = UnequalOperator::get_instance("!=").unwrap();
    assert_eq!(
        bin(&ne, DataValue::Int(1), DataValue::Double(1.5)),
        DataValue::Bool(true)
    );
    let ne2 = UnequalOperator::get_instance("<>").unwrap();
    assert_eq!(
        bin(&ne2, DataValue::Int(1), DataValue::Long(1)),
        DataValue::Bool(false)
    );
    assert!(UnequalOperator::get_instance("=?").is_none());
}

#[test]
fn ordering_compares_and_avoid_null_pointer() {
    assert_eq!(
        bin(
            &GreaterOperator::get_instance(),
            DataValue::Int(2),
            DataValue::Double(1.5)
        ),
        DataValue::Bool(true)
    );
    assert_eq!(
        bin(
            &GreaterEqualOperator::get_instance(),
            DataValue::Int(1),
            DataValue::Long(1)
        ),
        DataValue::Bool(true)
    );
    assert_eq!(
        bin(
            &LessOperator::get_instance(),
            DataValue::Str("a".into()),
            DataValue::Str("b".into())
        ),
        DataValue::Bool(true)
    );
    assert_eq!(
        bin(
            &LessEqualOperator::get_instance(),
            DataValue::Int(2),
            DataValue::Int(1)
        ),
        DataValue::Bool(false)
    );

    // avoidNullPointer:遇 null 返回 false 而非报错。
    let opts = QLOptions::builder().avoid_null_pointer(true).build();
    let r = GreaterOperator::get_instance()
        .execute(
            &v(DataValue::Null),
            &v(DataValue::Int(1)),
            &mut ctx(),
            &opts,
            &PureErrReporter::INSTANCE,
        )
        .unwrap();
    assert_eq!(r, DataValue::Bool(false));
    // 默认模式:null 比较报 INVALID_BINARY_OPERAND。
    assert_eq!(
        bin_err(
            &GreaterOperator::get_instance(),
            DataValue::Null,
            DataValue::Int(1)
        )
        .error_code(),
        error_codes::INVALID_BINARY_OPERAND
    );
}

// ---------- logic ----------

#[test]
fn logic_and_or_truth_rules() {
    let and = LogicAndOperator::get_instance("&&").unwrap();
    let and_word = LogicAndOperator::get_instance("and").unwrap();
    // Java 真值规则:仅 Boolean 操作数;null 视为 false。
    assert_eq!(
        bin(&and, DataValue::Bool(true), DataValue::Bool(true)),
        DataValue::Bool(true)
    );
    assert_eq!(
        bin(&and, DataValue::Bool(true), DataValue::Null),
        DataValue::Bool(false)
    );
    assert_eq!(
        bin(&and_word, DataValue::Null, DataValue::Null),
        DataValue::Bool(false)
    );
    // 非 Boolean → INVALID_BINARY_OPERAND(数字不作为真值)。
    assert_eq!(
        bin_err(&and, DataValue::Int(1), DataValue::Bool(true)).error_code(),
        error_codes::INVALID_BINARY_OPERAND
    );

    let or = LogicOrOperator::get_instance("||").unwrap();
    let or_word = LogicOrOperator::get_instance("or").unwrap();
    assert_eq!(
        bin(&or, DataValue::Bool(false), DataValue::Null),
        DataValue::Bool(false)
    );
    assert_eq!(
        bin(&or_word, DataValue::Null, DataValue::Bool(true)),
        DataValue::Bool(true)
    );
    assert_eq!(or.priority(), qlexpress_rust::ql_precedences::OR);
    assert_eq!(and.priority(), qlexpress_rust::ql_precedences::AND);
}

#[test]
fn logic_not_unary() {
    let op = LogicNotOperator::get_instance();
    assert_eq!(
        op.execute(&v(DataValue::Bool(false)), &PureErrReporter::INSTANCE)
            .unwrap(),
        DataValue::Bool(true)
    );
    // !null == true。
    assert_eq!(
        op.execute(&v(DataValue::Null), &PureErrReporter::INSTANCE)
            .unwrap(),
        DataValue::Bool(true)
    );
}

// ---------- instanceof ----------

#[test]
fn instanceof_operator() {
    let op = InstanceOfOperator::get_instance();
    let int_class = MetaClass::new(ClassRef::from_name("java.lang.Integer")).into_data_value();
    assert_eq!(
        bin(&op, DataValue::Int(1), int_class.clone()),
        DataValue::Bool(true)
    );
    assert_eq!(bin(&op, DataValue::Null, int_class), DataValue::Bool(false));
    assert_eq!(op.operator(), "instanceof");
}

// ---------- OperatorManager ----------

#[test]
fn operator_manager_resolves_all_stage4a_operators() {
    let manager = OperatorManager::new();
    for lexeme in [
        "+",
        "-",
        "*",
        "/",
        "%",
        "+=",
        "-=",
        "*=",
        "/=",
        "%=",
        "&",
        "|",
        "^",
        "<<",
        ">>",
        ">>>",
        "&=",
        "|=",
        "^=",
        "<<=",
        ">>=",
        ">>>=",
        "&&",
        "and",
        "||",
        "or",
        "==",
        "!=",
        "<>",
        ">",
        ">=",
        "<",
        "<=",
        "instanceof",
    ] {
        assert!(
            manager.get_binary_operator(lexeme).is_some(),
            "缺少 {lexeme}"
        );
    }
    assert!(manager.get_prefix_unary_operator("~").is_some());
    assert!(manager.get_prefix_unary_operator("!").is_some());
}
