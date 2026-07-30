//! 二元操作符抽象基类:承载所有具体二元操作符共享的计算逻辑。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator
//! (abstract class implements BinaryOperator;`plus/minus/.../compare/
//! equals/in/like` 等 protected 方法供子类复用)。
//!
//! Rust 说明:Java 用继承共享 protected 方法;Rust 以零大小类型 +
//! 关联函数承载,具体操作符在 `execute` 中调用
//! `BaseBinaryOperator::plus(...)` 等,等价于 Java 的 `super` 方法。
//! 需要操作符词素拼错误消息的方法显式接收 `operator` 参数
//! (Java 里取 `getOperator()`)。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::value::{DataValue, QValue};

use crate::runtime::operator::number::number_math::{self, NumberMath};

/// 对应 Java: BaseBinaryOperator(abstract,@author bingo)。
pub struct BaseBinaryOperator;

impl BaseBinaryOperator {
    /// Java `isSameType(left, right)`:两边类型名相同。
    pub(crate) fn is_same_type(left: &QValue, right: &QValue) -> bool {
        let left_value = left.get();
        let right_value = right.get();
        if let (DataValue::Object(left_obj), DataValue::Object(right_obj)) =
            (&left_value, &right_value)
        {
            return left_obj.borrow().native_type_name() == right_obj.borrow().native_type_name();
        }
        left.type_name() == right.type_name()
    }

    /// Java `isInstanceofComparable(value)`:Rust 中内建可比较类型为
    /// 数值、字符串、布尔、字符(Java 的 String/Boolean/Character 均
    /// implements Comparable)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#isInstanceofComparable。
    pub(crate) fn is_instanceof_comparable(value: &QValue) -> bool {
        let data = value.get();
        matches!(
            &data,
            DataValue::Str(_) | DataValue::Bool(_) | DataValue::Char(_)
        ) || data.is_number()
            || matches!(&data, DataValue::Object(obj) if obj.borrow().is_comparable())
    }

    /// Java `isBothBoolean(left, right)`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#isBothBoolean。
    pub(crate) fn is_both_boolean(left: &QValue, right: &QValue) -> bool {
        matches!(left.get(), DataValue::Bool(_)) && matches!(right.get(), DataValue::Bool(_))
    }

    /// Java `isBooleanAndNull(left, right)`:一侧 Boolean 一侧 null。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#isBooleanAndNull。
    pub(crate) fn is_boolean_and_null(left: &QValue, right: &QValue) -> bool {
        let l = left.get();
        let r = right.get();
        (l.is_null() && matches!(r, DataValue::Bool(_)))
            || (matches!(l, DataValue::Bool(_)) && r.is_null())
    }

    /// Java `isBothNumber(left, right)`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#isBothNumber。
    pub(crate) fn is_both_number(left: &QValue, right: &QValue) -> bool {
        left.get().is_number() && right.get().is_number()
    }

    /// Java `isBothNumberOrChar(leftValue, rightValue)`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#isBothNumberOrChar。
    pub(crate) fn is_both_number_or_char(left_value: &DataValue, right_value: &DataValue) -> bool {
        (left_value.is_number() || matches!(left_value, DataValue::Char(_)))
            && (right_value.is_number() || matches!(right_value, DataValue::Char(_)))
    }

    /// Java `char2Number(charOrNumber)`:Character → int(码点)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#char2number。
    pub(crate) fn char2number(char_or_number: &DataValue) -> DataValue {
        match char_or_number {
            DataValue::Char(c) => DataValue::Int(*c as i32),
            number => number.clone(),
        }
    }

    /// Java `isNumberCharacter(left, right)`:一侧 Number 一侧 Character。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#isNumberCharacter。
    pub(crate) fn is_number_character(left: &QValue, right: &QValue) -> bool {
        let l = left.get();
        let r = right.get();
        (matches!(l, DataValue::Char(_)) && r.is_number())
            || (l.is_number() && matches!(r, DataValue::Char(_)))
    }

    /// Java `isNumber(value)`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#isNumber。
    pub(crate) fn is_number(value: &QValue) -> bool {
        value.get().is_number()
    }

    /// Java `assertLeftValue(left, errorReporter)`:赋值类操作符要求
    /// 左操作数为 LeftValue,否则报 INVALID_ASSIGNMENT("on the left side")。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#assertLeftValue。
    pub(crate) fn assert_left_value(
        left: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<(), QLException> {
        if !left.is_left() {
            return Err(error_reporter.report_format(
                error_codes::INVALID_ASSIGNMENT,
                error_codes::error_msg(error_codes::INVALID_ASSIGNMENT),
                &["on the left side".to_string()],
            ));
        }
        Ok(())
    }

    /// Java `plus(left, right, qlOptions, errorReporter)`。
    ///
    /// 语义要点:QLExpress 只支持 String 与 Number 的 `+`;任一操作数为
    /// String 时按 Java 字符串拼接(null 拼作 "null",double 带 `.0`);
    /// Character 按码点转 int 后相加;precise 模式强转 BigDecimal 计算。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#plus。
    pub fn plus(
        operator: &str,
        left: &QValue,
        right: &QValue,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let left_value = left.get();
        let right_value = right.get();

        // Java: if (leftValue instanceof String) return (String)leftValue + rightValue;
        if let DataValue::Str(l) = &left_value {
            return Ok(DataValue::Str(format!(
                "{l}{}",
                number_math::java_value_string(&right_value)
            )));
        }
        if let DataValue::Str(r) = &right_value {
            return Ok(DataValue::Str(format!(
                "{}{r}",
                number_math::java_value_string(&left_value)
            )));
        }

        if Self::is_both_number(left, right) {
            return Self::add(ql_options, &left_value, &right_value);
        }
        if Self::is_both_number_or_char(&left_value, &right_value) {
            return Self::add(
                ql_options,
                &Self::char2number(&left_value),
                &Self::char2number(&right_value),
            );
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// Java `private Number add(...)`:precise 模式转 BigDecimal 相加。
    fn add(
        ql_options: &QLOptions,
        left_value: &DataValue,
        right_value: &DataValue,
    ) -> Result<DataValue, QLException> {
        if ql_options.is_precise() {
            NumberMath::add(
                &NumberMath::to_big_decimal(left_value),
                &NumberMath::to_big_decimal(right_value),
            )
        } else {
            NumberMath::add(left_value, right_value)
        }
    }

    /// Java `minus(left, right, qlOptions, errorReporter)`(支持 Number/Char)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#minus。
    pub fn minus(
        operator: &str,
        left: &QValue,
        right: &QValue,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let left_value = left.get();
        let right_value = right.get();
        if Self::is_both_number(left, right) {
            return Self::subtract(ql_options, &left_value, &right_value);
        }
        if Self::is_both_number_or_char(&left_value, &right_value) {
            return Self::subtract(
                ql_options,
                &Self::char2number(&left_value),
                &Self::char2number(&right_value),
            );
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// Java `private Number subtract(...)`。
    fn subtract(
        ql_options: &QLOptions,
        left_value: &DataValue,
        right_value: &DataValue,
    ) -> Result<DataValue, QLException> {
        if ql_options.is_precise() {
            NumberMath::subtract(
                &NumberMath::to_big_decimal(left_value),
                &NumberMath::to_big_decimal(right_value),
            )
        } else {
            NumberMath::subtract(left_value, right_value)
        }
    }

    /// Java `multiply(...)`(仅 Number,不含 Character)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#multiply。
    pub fn multiply(
        operator: &str,
        left: &QValue,
        right: &QValue,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let left_value = left.get();
        let right_value = right.get();
        if Self::is_both_number(left, right) {
            let result = if ql_options.is_precise() {
                NumberMath::multiply(
                    &NumberMath::to_big_decimal(&left_value),
                    &NumberMath::to_big_decimal(&right_value),
                )
            } else {
                NumberMath::multiply(&left_value, &right_value)
            };
            return result;
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// Java `divide(...)`。
    ///
    /// 语义要点:Java 在此 catch ArithmeticException 并改报
    /// INVALID_ARITHMETIC(消息取原异常 message,如 "Division by zero");
    /// 浮点除零不抛异常(IEEE Infinity/NaN)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#divide。
    pub fn divide(
        operator: &str,
        left: &QValue,
        right: &QValue,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let left_value = left.get();
        let right_value = right.get();
        if Self::is_both_number(left, right) {
            let result = if ql_options.is_precise() {
                NumberMath::divide(
                    &NumberMath::to_big_decimal(&left_value),
                    &NumberMath::to_big_decimal(&right_value),
                )
            } else {
                NumberMath::divide(&left_value, &right_value)
            };
            return result.map_err(|e| Self::rethrow_arithmetic(e, error_reporter));
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// Java `remainder(...)`(Java 不 catch ArithmeticException,原样上抛)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#remainder。
    pub fn remainder(
        operator: &str,
        left: &QValue,
        right: &QValue,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let left_value = left.get();
        let right_value = right.get();
        if Self::is_both_number(left, right) {
            let result = if ql_options.is_precise() {
                NumberMath::remainder(
                    &NumberMath::to_big_decimal(&left_value),
                    &NumberMath::to_big_decimal(&right_value),
                )
            } else {
                NumberMath::remainder(&left_value, &right_value)
            };
            return result;
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// Java `bitwiseAnd(...)`:Boolean 操作数按逻辑与(null 视为 false),
    /// 数值按整型域位与。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#bitwiseAnd。
    pub fn bitwise_and(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        if Self::is_both_boolean(left, right) || Self::is_boolean_and_null(left, right) {
            // Java: Optional.ofNullable(x).orElse(Boolean.FALSE) & ...
            let l = matches!(left.get(), DataValue::Bool(true));
            let r = matches!(right.get(), DataValue::Bool(true));
            return Ok(DataValue::Bool(l & r));
        }
        if Self::is_both_number(left, right) {
            return NumberMath::and(&left.get(), &right.get());
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// 按 Java 数值提升规则执行按位或。
    /// 参数：`operator`、`left`、`right`、`error_reporter`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/base/BaseBinaryOperator.java`，方法 `bitwiseOr`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `bitwiseOr(...)`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#bitwiseOr。
    pub fn bitwise_or(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        if Self::is_both_boolean(left, right) || Self::is_boolean_and_null(left, right) {
            let l = matches!(left.get(), DataValue::Bool(true));
            let r = matches!(right.get(), DataValue::Bool(true));
            return Ok(DataValue::Bool(l | r));
        }
        if Self::is_both_number(left, right) {
            return NumberMath::or(&left.get(), &right.get());
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// 按 Java 数值提升规则执行按位异或。
    /// 参数：`operator`、`left`、`right`、`error_reporter`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/base/BaseBinaryOperator.java`，方法 `bitwiseXor`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `bitwiseXor(...)`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#bitwiseXor。
    pub fn bitwise_xor(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        if Self::is_both_boolean(left, right) || Self::is_boolean_and_null(left, right) {
            let l = matches!(left.get(), DataValue::Bool(true));
            let r = matches!(right.get(), DataValue::Bool(true));
            return Ok(DataValue::Bool(l ^ r));
        }
        if Self::is_both_number(left, right) {
            return NumberMath::xor(&left.get(), &right.get());
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// 执行遵循 Java 位宽掩码规则的左移运算。
    /// 参数：`operator`、`left`、`right`、`error_reporter`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/base/BaseBinaryOperator.java`，方法 `leftShift`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `leftShift(...)`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#leftShift。
    pub fn left_shift(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        if Self::is_both_number(left, right) {
            return NumberMath::left_shift(&left.get(), &right.get());
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// 执行保留符号位的 Java 右移运算。
    /// 参数：`operator`、`left`、`right`、`error_reporter`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/base/BaseBinaryOperator.java`，方法 `rightShift`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `rightShift(...)`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#rightShift。
    pub fn right_shift(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        if Self::is_both_number(left, right) {
            return NumberMath::right_shift(&left.get(), &right.get());
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// 执行零填充的 Java 无符号右移运算。
    /// 参数：`operator`、`left`、`right`、`error_reporter`；返回：`Result<DataValue, QLException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/operator/base/BaseBinaryOperator.java`，方法 `rightShiftUnsigned`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `rightShiftUnsigned(...)`。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#rightShiftUnsigned。
    pub fn right_shift_unsigned(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        if Self::is_both_number(left, right) {
            return NumberMath::right_shift_unsigned(&left.get(), &right.get());
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// Java `compare(left, right, errorReporter)`。
    ///
    /// 语义要点:先 `Objects.equals`(完全相等短路);数值跨类型比较走
    /// NumberMath.compareTo;Number/Character 混合把字符按码点转 int;
    /// 同类型 Comparable(String/Boolean/Character)按自然序;其余报
    /// INVALID_BINARY_OPERAND。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#compare。
    pub fn compare(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<i32, QLException> {
        let left_value = left.get();
        let right_value = right.get();
        if left_value == right_value {
            return Ok(0);
        }
        if Self::is_both_number(left, right) {
            return NumberMath::compare_to(&left_value, &right_value);
        }
        if Self::is_number_character(left, right) {
            return if Self::is_number(left) {
                NumberMath::compare_to(&left_value, &Self::char2number(&right_value))
            } else {
                NumberMath::compare_to(&Self::char2number(&left_value), &right_value)
            };
        }
        if Self::is_same_type(left, right) && Self::is_instanceof_comparable(left) {
            return Ok(match (&left_value, &right_value) {
                (DataValue::Str(a), DataValue::Str(b)) => compare_ord(a.cmp(b)),
                (DataValue::Bool(a), DataValue::Bool(b)) => compare_ord(a.cmp(b)),
                (DataValue::Char(a), DataValue::Char(b)) => compare_ord(a.cmp(b)),
                (DataValue::Object(a), DataValue::Object(b)) => {
                    let left_obj = a.borrow();
                    let right_obj = b.borrow();
                    left_obj
                        .compare_to(&*right_obj)
                        .map(compare_ord)
                        .ok_or_else(|| {
                            Self::build_invalid_operand_type_exception(
                                operator,
                                left,
                                right,
                                error_reporter,
                            )
                        })?
                }
                _ => {
                    return Err(Self::build_invalid_operand_type_exception(
                        operator,
                        left,
                        right,
                        error_reporter,
                    ))
                }
            });
        }
        Err(Self::build_invalid_operand_type_exception(
            operator,
            left,
            right,
            error_reporter,
        ))
    }

    /// Java `equals(left, right, errorReporter)`。
    ///
    /// 语义要点:数值/数值-字符/同类型 Comparable 走 `compare == 0`
    /// (即 `1 == 1L == 1.0`、`'a' == 97`);其余走 `Objects.equals`
    /// (Rust:DataValue 结构相等,跨数值类型变体不等)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#equals。
    pub fn equals(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<bool, QLException> {
        let left_value = left.get();
        let right_value = right.get();
        if Self::is_both_number(left, right)
            || Self::is_number_character(left, right)
            || (Self::is_same_type(left, right) && Self::is_instanceof_comparable(left))
        {
            Ok(Self::compare(operator, left, right, error_reporter)? == 0)
        } else {
            Ok(left_value == right_value)
        }
    }

    /// Java `in(left, right, errorReporter)`:右操作数为集合/数组时逐项
    /// equals,为 String 时 `contains(String.valueOf(left))`;两侧 null
    /// 相等规则:null in null == true,一侧 null == false。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#contains。
    pub fn contains(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<bool, QLException> {
        let right_operand = right.get();
        let left_operand = left.get();
        if left_operand.is_null() && right_operand.is_null() {
            return Ok(true);
        }
        if left_operand.is_null() || right_operand.is_null() {
            return Ok(false);
        }
        match &right_operand {
            DataValue::List(list) => {
                // Java: for (Object rightElement : rightCollection)
                //         if (equals(left, new DataValue(rightElement), ...))
                for element in list.borrow().iter() {
                    if Self::equals(
                        operator,
                        left,
                        &QValue::Data(element.clone()),
                        error_reporter,
                    )? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            DataValue::Array(array) => {
                for element in array.borrow().iter() {
                    if Self::equals(
                        operator,
                        left,
                        &QValue::Data(element.clone()),
                        error_reporter,
                    )? {
                        return Ok(true);
                    }
                }
                Ok(false)
            }
            DataValue::Str(s) => Ok(s.contains(&number_math::java_value_string(&left_operand))),
            _ => Err(Self::build_invalid_operand_type_exception(
                operator,
                left,
                right,
                error_reporter,
            )),
        }
    }

    /// Java `like(left, right, errorReporter)`:仅 String 操作数,
    /// 模式仅支持 `%` 通配(matchPattern 双指针回溯算法,逐行对齐 Java)。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#like。
    pub fn like(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<bool, QLException> {
        let target = left.get();
        let pattern = right.get();
        if target.is_null() && pattern.is_null() {
            return Ok(true);
        }
        if target.is_null() || pattern.is_null() {
            return Ok(false);
        }
        match (&target, &pattern) {
            (DataValue::Str(t), DataValue::Str(p)) => Ok(match_pattern(t, p)),
            _ => Err(Self::build_invalid_operand_type_exception(
                operator,
                left,
                right,
                error_reporter,
            )),
        }
    }

    /// Java `buildInvalidOperandTypeException(left, right, errorReporter)`
    /// —— INVALID_BINARY_OPERAND,参数顺序:
    /// 操作符、左类型名、左值、右类型名、右值。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.operator.base.BaseBinaryOperator#buildInvalidOperandTypeException。
    pub fn build_invalid_operand_type_exception(
        operator: &str,
        left: &QValue,
        right: &QValue,
        error_reporter: &dyn ErrorReporter,
    ) -> QLException {
        error_reporter.report_format(
            error_codes::INVALID_BINARY_OPERAND,
            error_codes::error_msg(error_codes::INVALID_BINARY_OPERAND),
            &[
                operator.to_string(),
                left.type_name().to_string(),
                number_math::java_value_string(&left.get()),
                right.type_name().to_string(),
                number_math::java_value_string(&right.get()),
            ],
        )
    }

    /// Java `BaseBinaryOperator.divide` 的 catch 语义:NumberMath 层抛出
    /// 的 ArithmeticException 改报 INVALID_ARITHMETIC;其余错误原样传播
    /// (Java 中 UnsupportedOperationException 等也是直接上抛)。
    fn rethrow_arithmetic(err: QLException, error_reporter: &dyn ErrorReporter) -> QLException {
        if err.error_code() == number_math::ARITHMETIC_EXCEPTION {
            error_reporter.report(error_codes::INVALID_ARITHMETIC, err.reason())
        } else {
            err
        }
    }
}

fn compare_ord(ord: std::cmp::Ordering) -> i32 {
    match ord {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

/// Java `BaseBinaryOperator.matchPattern`:`%` 通配符匹配(双指针 +
/// 回溯),逐语句对齐 Java 原版算法。
fn match_pattern(s: &str, pattern: &str) -> bool {
    let s: Vec<char> = s.chars().collect();
    let pattern: Vec<char> = pattern.chars().collect();
    let (mut s_pointer, mut p_pointer) = (0usize, 0usize);
    let (s_len, p_len) = (s.len(), pattern.len());
    let (mut s_recall, mut p_recall) = (-1i64, -1i64);
    while s_pointer < s_len {
        if p_pointer < p_len && s[s_pointer] == pattern[p_pointer] {
            s_pointer += 1;
            p_pointer += 1;
        } else if p_pointer < p_len && pattern[p_pointer] == '%' {
            s_recall = s_pointer as i64;
            p_recall = p_pointer as i64;
            p_pointer += 1;
        } else if s_recall >= 0 {
            s_recall += 1;
            s_pointer = s_recall as usize;
            p_pointer = (p_recall + 1) as usize;
        } else {
            return false;
        }
    }
    while p_pointer < p_len && pattern[p_pointer] == '%' {
        p_pointer += 1;
    }
    p_pointer == p_len
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;

    fn v(d: DataValue) -> QValue {
        QValue::Data(d)
    }

    fn opts() -> QLOptions {
        QLOptions::builder().build()
    }

    #[test]
    fn plus_supports_string_concat_and_char_code() {
        // "a" + 1 = "a1";null 拼作 "null"。
        assert_eq!(
            BaseBinaryOperator::plus(
                "+",
                &v(DataValue::Str("a".into())),
                &v(DataValue::Int(1)),
                &opts(),
                &PureErrReporter::INSTANCE
            )
            .unwrap(),
            DataValue::Str("a1".to_string())
        );
        assert_eq!(
            BaseBinaryOperator::plus(
                "+",
                &v(DataValue::Str("v=".into())),
                &v(DataValue::Null),
                &opts(),
                &PureErrReporter::INSTANCE
            )
            .unwrap(),
            DataValue::Str("v=null".to_string())
        );
        // 'a' + 1 = 98(字符按码点转 int)。
        assert_eq!(
            BaseBinaryOperator::plus(
                "+",
                &v(DataValue::Char('a' as u16)),
                &v(DataValue::Int(1)),
                &opts(),
                &PureErrReporter::INSTANCE
            )
            .unwrap(),
            DataValue::Int(98)
        );
    }

    #[test]
    fn equals_compares_across_numeric_types() {
        // Java 语义要点:== 跨数值类型(1 == 1L == 1.0 == "1.00"BigDecimal)。
        for (l, r) in [
            (DataValue::Int(1), DataValue::Long(1)),
            (DataValue::Int(1), DataValue::Double(1.0)),
            (DataValue::Long(1), DataValue::BigDec("1.00".into())),
            (DataValue::Char('a' as u16), DataValue::Int(97)),
        ] {
            assert!(
                BaseBinaryOperator::equals("==", &v(l), &v(r), &PureErrReporter::INSTANCE).unwrap()
            );
        }
        assert!(!BaseBinaryOperator::equals(
            "==",
            &v(DataValue::Int(1)),
            &v(DataValue::Double(1.5)),
            &PureErrReporter::INSTANCE
        )
        .unwrap());
        // 非 Comparable 走 Objects.equals:"a" == "a"。
        assert!(BaseBinaryOperator::equals(
            "==",
            &v(DataValue::Str("a".into())),
            &v(DataValue::Str("a".into())),
            &PureErrReporter::INSTANCE
        )
        .unwrap());
    }

    #[test]
    fn boolean_bitwise_treats_null_as_false() {
        // Java 语义要点:true & null == false,null | true == true。
        assert_eq!(
            BaseBinaryOperator::bitwise_and(
                "&",
                &v(DataValue::Bool(true)),
                &v(DataValue::Null),
                &PureErrReporter::INSTANCE
            )
            .unwrap(),
            DataValue::Bool(false)
        );
        assert_eq!(
            BaseBinaryOperator::bitwise_or(
                "|",
                &v(DataValue::Null),
                &v(DataValue::Bool(true)),
                &PureErrReporter::INSTANCE
            )
            .unwrap(),
            DataValue::Bool(true)
        );
    }

    #[test]
    fn divide_by_zero_reports_invalid_arithmetic() {
        let err = BaseBinaryOperator::divide(
            "/",
            &v(DataValue::Int(1)),
            &v(DataValue::Int(0)),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_ARITHMETIC);
        assert_eq!(err.reason(), "Division by zero");
    }

    #[test]
    fn invalid_operand_message_aligned() {
        let err = BaseBinaryOperator::plus(
            "+",
            &v(DataValue::Bool(true)),
            &v(DataValue::Bool(false)),
            &opts(),
            &PureErrReporter::INSTANCE,
        )
        .unwrap_err();
        assert_eq!(err.error_code(), error_codes::INVALID_BINARY_OPERAND);
        assert!(err.reason().contains("the '+' operator can not be applied"));
    }

    #[test]
    fn like_pattern_matching() {
        assert!(match_pattern("abc", "a%"));
        assert!(match_pattern("abc", "%b%"));
        assert!(!match_pattern("abc", "a%d"));
        assert!(match_pattern("abc", "abc"));
        assert!(match_pattern("abc", "%"));
    }
}
