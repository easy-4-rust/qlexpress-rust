//! Error codes, mirroring Java `QLErrorCodes` enum (SPEC §3.4).
//!
//! Every constant keeps the Java name and the original (English) message
//! template verbatim. Templates use Java `String.format` style placeholders
//! (`%s`, `%d`); substitute them with [`format_msg`].

// ---- syntax error ----
/// `SYNTAX_ERROR` 错误码或错误消息模板。
pub const SYNTAX_ERROR: &str = "SYNTAX_ERROR";
/// `MISSING_INDEX` 错误码或错误消息模板。
pub const MISSING_INDEX: &str = "MISSING_INDEX";
/// `INVALID_NUMBER` 错误码或错误消息模板。
pub const INVALID_NUMBER: &str = "INVALID_NUMBER";
/// `CLASS_NOT_FOUND` 错误码或错误消息模板。
pub const CLASS_NOT_FOUND: &str = "CLASS_NOT_FOUND";
/// Stage 6 / FixedSizeStack: emitted when an operand-stack push would
/// exceed the declared capacity (mirrors Java's StackOverflowError).
pub const STACK_OVERFLOW: &str = "STACK_OVERFLOW";
/// `try_push` on a full operand stack returns this code.
pub const OPERAND_STACK_OVERFLOW: &str = "OPERAND_STACK_OVERFLOW";
/// `try_pop` / `try_peak` on an empty operand stack returns this code.
pub const OPERAND_STACK_UNDERFLOW: &str = "OPERAND_STACK_UNDERFLOW";

// ---- runtime error ----
/// `INVALID_INDEX` 错误码或错误消息模板。
pub const INVALID_INDEX: &str = "INVALID_INDEX";
/// `INDEX_OUT_BOUND` 错误码或错误消息模板。
pub const INDEX_OUT_BOUND: &str = "INDEX_OUT_BOUND";
/// `NONINDEXABLE_OBJECT` 错误码或错误消息模板。
pub const NONINDEXABLE_OBJECT: &str = "NONINDEXABLE_OBJECT";
/// `NONTRAVERSABLE_OBJECT` 错误码或错误消息模板。
pub const NONTRAVERSABLE_OBJECT: &str = "NONTRAVERSABLE_OBJECT";
/// `NULL_FIELD_ACCESS` 错误码或错误消息模板。
pub const NULL_FIELD_ACCESS: &str = "NULL_FIELD_ACCESS";
/// `NULL_METHOD_ACCESS` 错误码或错误消息模板。
pub const NULL_METHOD_ACCESS: &str = "NULL_METHOD_ACCESS";
/// `FIELD_NOT_FOUND` 错误码或错误消息模板。
pub const FIELD_NOT_FOUND: &str = "FIELD_NOT_FOUND";
/// `SET_FIELD_UNKNOWN_ERROR` 错误码或错误消息模板。
pub const SET_FIELD_UNKNOWN_ERROR: &str = "SET_FIELD_UNKNOWN_ERROR";
/// `GET_FIELD_UNKNOWN_ERROR` 错误码或错误消息模板。
pub const GET_FIELD_UNKNOWN_ERROR: &str = "GET_FIELD_UNKNOWN_ERROR";
/// `INVOKE_METHOD_WITH_WRONG_ARGUMENTS` 错误码或错误消息模板。
pub const INVOKE_METHOD_WITH_WRONG_ARGUMENTS: &str = "INVOKE_METHOD_WITH_WRONG_ARGUMENTS";
/// `INVOKE_METHOD_INNER_ERROR` 错误码或错误消息模板。
pub const INVOKE_METHOD_INNER_ERROR: &str = "INVOKE_METHOD_INNER_ERROR";
/// `INVOKE_METHOD_UNKNOWN_ERROR` 错误码或错误消息模板。
pub const INVOKE_METHOD_UNKNOWN_ERROR: &str = "INVOKE_METHOD_UNKNOWN_ERROR";
/// `INVOKE_FUNCTION_INNER_ERROR` 错误码或错误消息模板。
pub const INVOKE_FUNCTION_INNER_ERROR: &str = "INVOKE_FUNCTION_INNER_ERROR";
/// `FUNCTION_NOT_FOUND` 错误码或错误消息模板。
pub const FUNCTION_NOT_FOUND: &str = "FUNCTION_NOT_FOUND";
/// `FUNCTION_TYPE_MISMATCH` 错误码或错误消息模板。
pub const FUNCTION_TYPE_MISMATCH: &str = "FUNCTION_TYPE_MISMATCH";
/// `INVOKE_LAMBDA_ERROR` 错误码或错误消息模板。
pub const INVOKE_LAMBDA_ERROR: &str = "INVOKE_LAMBDA_ERROR";
/// `NULL_CALL` 错误码或错误消息模板。
pub const NULL_CALL: &str = "NULL_CALL";
/// `OBJECT_NOT_CALLABLE` 错误码或错误消息模板。
pub const OBJECT_NOT_CALLABLE: &str = "OBJECT_NOT_CALLABLE";
/// `METHOD_NOT_FOUND` 错误码或错误消息模板。
pub const METHOD_NOT_FOUND: &str = "METHOD_NOT_FOUND";
/// `INVOKE_CONSTRUCTOR_UNKNOWN_ERROR` 错误码或错误消息模板。
pub const INVOKE_CONSTRUCTOR_UNKNOWN_ERROR: &str = "INVOKE_CONSTRUCTOR_UNKNOWN_ERROR";
/// `INVOKE_CONSTRUCTOR_INNER_ERROR` 错误码或错误消息模板。
pub const INVOKE_CONSTRUCTOR_INNER_ERROR: &str = "INVOKE_CONSTRUCTOR_INNER_ERROR";
/// `NO_SUITABLE_CONSTRUCTOR` 错误码或错误消息模板。
pub const NO_SUITABLE_CONSTRUCTOR: &str = "NO_SUITABLE_CONSTRUCTOR";
/// `EXECUTE_BLOCK_ERROR` 错误码或错误消息模板。
pub const EXECUTE_BLOCK_ERROR: &str = "EXECUTE_BLOCK_ERROR";
/// `INCOMPATIBLE_TYPE_CAST` 错误码或错误消息模板。
pub const INCOMPATIBLE_TYPE_CAST: &str = "INCOMPATIBLE_TYPE_CAST";
/// `INVALID_CAST_TARGET` 错误码或错误消息模板。
pub const INVALID_CAST_TARGET: &str = "INVALID_CAST_TARGET";
/// `SCRIPT_TIME_OUT` 错误码或错误消息模板。
pub const SCRIPT_TIME_OUT: &str = "SCRIPT_TIME_OUT";
/// `INCOMPATIBLE_ASSIGNMENT_TYPE` 错误码或错误消息模板。
pub const INCOMPATIBLE_ASSIGNMENT_TYPE: &str = "INCOMPATIBLE_ASSIGNMENT_TYPE";
/// `FOR_EACH_ITERABLE_REQUIRED` 错误码或错误消息模板。
pub const FOR_EACH_ITERABLE_REQUIRED: &str = "FOR_EACH_ITERABLE_REQUIRED";
/// `FOR_EACH_TYPE_MISMATCH` 错误码或错误消息模板。
pub const FOR_EACH_TYPE_MISMATCH: &str = "FOR_EACH_TYPE_MISMATCH";
/// `FOR_EACH_UNKNOWN_ERROR` 错误码或错误消息模板。
pub const FOR_EACH_UNKNOWN_ERROR: &str = "FOR_EACH_UNKNOWN_ERROR";
/// `FOR_INIT_ERROR` 错误码或错误消息模板。
pub const FOR_INIT_ERROR: &str = "FOR_INIT_ERROR";
/// `FOR_BODY_ERROR` 错误码或错误消息模板。
pub const FOR_BODY_ERROR: &str = "FOR_BODY_ERROR";
/// `FOR_UPDATE_ERROR` 错误码或错误消息模板。
pub const FOR_UPDATE_ERROR: &str = "FOR_UPDATE_ERROR";
/// `FOR_CONDITION_ERROR` 错误码或错误消息模板。
pub const FOR_CONDITION_ERROR: &str = "FOR_CONDITION_ERROR";
/// `FOR_CONDITION_BOOL_REQUIRED` 错误码或错误消息模板。
pub const FOR_CONDITION_BOOL_REQUIRED: &str = "FOR_CONDITION_BOOL_REQUIRED";
/// `WHILE_CONDITION_BOOL_REQUIRED` 错误码或错误消息模板。
pub const WHILE_CONDITION_BOOL_REQUIRED: &str = "WHILE_CONDITION_BOOL_REQUIRED";
/// `WHILE_CONDITION_ERROR` 错误码或错误消息模板。
pub const WHILE_CONDITION_ERROR: &str = "WHILE_CONDITION_ERROR";
/// `CONDITION_BOOL_REQUIRED` 错误码或错误消息模板。
pub const CONDITION_BOOL_REQUIRED: &str = "CONDITION_BOOL_REQUIRED";
/// `ARRAY_SIZE_NUM_REQUIRED` 错误码或错误消息模板。
pub const ARRAY_SIZE_NUM_REQUIRED: &str = "ARRAY_SIZE_NUM_REQUIRED";
/// `EXCEED_MAX_ARR_LENGTH` 错误码或错误消息模板。
pub const EXCEED_MAX_ARR_LENGTH: &str = "EXCEED_MAX_ARR_LENGTH";
/// `INCOMPATIBLE_ARRAY_ITEM_TYPE` 错误码或错误消息模板。
pub const INCOMPATIBLE_ARRAY_ITEM_TYPE: &str = "INCOMPATIBLE_ARRAY_ITEM_TYPE";
/// `INVALID_ASSIGNMENT` 错误码或错误消息模板。
pub const INVALID_ASSIGNMENT: &str = "INVALID_ASSIGNMENT";
/// `EXECUTE_OPERATOR_EXCEPTION` 错误码或错误消息模板。
pub const EXECUTE_OPERATOR_EXCEPTION: &str = "EXECUTE_OPERATOR_EXCEPTION";
/// `INVALID_ARITHMETIC` 错误码或错误消息模板。
pub const INVALID_ARITHMETIC: &str = "INVALID_ARITHMETIC";
/// `INVALID_BINARY_OPERAND` 错误码或错误消息模板。
pub const INVALID_BINARY_OPERAND: &str = "INVALID_BINARY_OPERAND";
/// `INVALID_UNARY_OPERAND` 错误码或错误消息模板。
pub const INVALID_UNARY_OPERAND: &str = "INVALID_UNARY_OPERAND";
/// `EXECUTE_FINAL_BLOCK_ERROR` 错误码或错误消息模板。
pub const EXECUTE_FINAL_BLOCK_ERROR: &str = "EXECUTE_FINAL_BLOCK_ERROR";
/// `EXECUTE_TRY_BLOCK_ERROR` 错误码或错误消息模板。
pub const EXECUTE_TRY_BLOCK_ERROR: &str = "EXECUTE_TRY_BLOCK_ERROR";
/// `EXECUTE_CATCH_HANDLER_ERROR` 错误码或错误消息模板。
pub const EXECUTE_CATCH_HANDLER_ERROR: &str = "EXECUTE_CATCH_HANDLER_ERROR";

// ---- operator restriction error ----
/// `OPERATOR_NOT_ALLOWED` 错误码或错误消息模板。
pub const OPERATOR_NOT_ALLOWED: &str = "OPERATOR_NOT_ALLOWED";

// ---- serializable parse cache error ----
/// `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION` 错误码或错误消息模板。
pub const SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION: &str =
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION";
/// `SERIALIZABLE_PARSE_CACHE_INVALID_MODEL` 错误码或错误消息模板。
pub const SERIALIZABLE_PARSE_CACHE_INVALID_MODEL: &str = "SERIALIZABLE_PARSE_CACHE_INVALID_MODEL";
/// `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION` 错误码或错误消息模板。
pub const SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION: &str =
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION";
/// `SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT` 错误码或错误消息模板。
pub const SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT: &str =
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT";
/// `SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND` 错误码或错误消息模板。
pub const SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND: &str =
    "SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND";
/// `SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND` 错误码或错误消息模板。
pub const SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND: &str =
    "SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND";

// ---- user defined exception ----
/// `INVALID_ARGUMENT` 错误码或错误消息模板。
pub const INVALID_ARGUMENT: &str = "INVALID_ARGUMENT";
/// `BIZ_EXCEPTION` 错误码或错误消息模板。
pub const BIZ_EXCEPTION: &str = "BIZ_EXCEPTION";
/// `QL_THROW` 错误码或错误消息模板。
pub const QL_THROW: &str = "QL_THROW";

/// 按错误码查询稳定消息模板。
/// 参数：`code`；返回：`&'static str`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLErrorCodes.java`，方法 `errorMsg`；Rust 侧按所有权与 `Result` 语义适配。
/// Returns the Java message template for an error code (verbatim from
/// `QLErrorCodes`). Unknown codes yield an empty string, mirroring the
/// empty-message codes in Java.
/// 对应 Java: 无（Rust 原生适配）。
pub fn error_msg(code: &str) -> &'static str {
    match code {
        SYNTAX_ERROR => "",
        MISSING_INDEX => "missing index expression",
        INVALID_NUMBER => "invalid number",
        CLASS_NOT_FOUND => "can not find class: %s",
        INVALID_INDEX => "index can only be number",
        INDEX_OUT_BOUND => "index out of bound",
        NONINDEXABLE_OBJECT => "object of class %s is not indexable",
        NONTRAVERSABLE_OBJECT => "object of class %s is not traversable",
        NULL_FIELD_ACCESS => "can not access field from null",
        NULL_METHOD_ACCESS => "can not access method from null",
        FIELD_NOT_FOUND => "'%s' field not found",
        SET_FIELD_UNKNOWN_ERROR => "unknown error when setting field '%s' value",
        GET_FIELD_UNKNOWN_ERROR => "unknown error when getting field '%s' value",
        INVOKE_METHOD_WITH_WRONG_ARGUMENTS => "invoke method '%s' with wrong arguments",
        INVOKE_METHOD_INNER_ERROR => "exception from inner when invoking method '%s'",
        INVOKE_METHOD_UNKNOWN_ERROR => "unknown error when invoking method '%s'",
        INVOKE_FUNCTION_INNER_ERROR => {
            "exception from inner when invoking function '%s', error message: %s"
        }
        FUNCTION_NOT_FOUND => "function '%s' not found",
        FUNCTION_TYPE_MISMATCH => "symbol '%s' is not a function type",
        INVOKE_LAMBDA_ERROR => "error when invoking lambda",
        NULL_CALL => "can not call null",
        OBJECT_NOT_CALLABLE => "type '%s' is not callable",
        METHOD_NOT_FOUND => "no suitable method '%s' found for args %s",
        INVOKE_CONSTRUCTOR_UNKNOWN_ERROR => "unknown error when invoking constructor",
        INVOKE_CONSTRUCTOR_INNER_ERROR => "exception from inner when invoking constructor",
        NO_SUITABLE_CONSTRUCTOR => "no suitable constructor for types %s",
        EXECUTE_BLOCK_ERROR => "error when executing block",
        INCOMPATIBLE_TYPE_CAST => "incompatible cast from type: %s to type: %s",
        INVALID_CAST_TARGET => "target for type cast must be a class, but accept %s",
        SCRIPT_TIME_OUT => "script exceeds timeout milliseconds, which is %d ms",
        INCOMPATIBLE_ASSIGNMENT_TYPE => {
            "variable declared type %s, assigned with incompatible value type %s"
        }
        FOR_EACH_ITERABLE_REQUIRED => "for-each can only be applied to iterable",
        FOR_EACH_TYPE_MISMATCH => "for-each type mismatch, required %s, but %s provided",
        FOR_EACH_UNKNOWN_ERROR => "unknown error when executing for-each",
        FOR_INIT_ERROR => "error when executing for init",
        FOR_BODY_ERROR => "error when executing for body",
        FOR_UPDATE_ERROR => "error when executing for update",
        FOR_CONDITION_ERROR => "error when executing for condition",
        FOR_CONDITION_BOOL_REQUIRED => "result of for condition must be bool",
        WHILE_CONDITION_BOOL_REQUIRED => "result of while condition must be bool",
        WHILE_CONDITION_ERROR => "error when executing while condition",
        CONDITION_BOOL_REQUIRED => "result of condition expression must be bool",
        ARRAY_SIZE_NUM_REQUIRED => "size of array must be number",
        EXCEED_MAX_ARR_LENGTH => "array length %d, exceed max allowed length %d",
        INCOMPATIBLE_ARRAY_ITEM_TYPE => {
            "item %d with type %s incompatible with array type %s"
        }
        INVALID_ASSIGNMENT => "value %s is not assignable",
        EXECUTE_OPERATOR_EXCEPTION => "exception when executing '%s %s %s'",
        INVALID_ARITHMETIC => "",
        INVALID_BINARY_OPERAND => {
            "the '%s' operator can not be applied to leftType:%s with leftValue:%s and rightType:%s with rightValue:%s"
        }
        INVALID_UNARY_OPERAND => "the '%s' operator can not be applied to type %s with value %s",
        EXECUTE_FINAL_BLOCK_ERROR => "error when executing final block in try...catch...final...",
        EXECUTE_TRY_BLOCK_ERROR => "error when executing try... block",
        EXECUTE_CATCH_HANDLER_ERROR => "error when executing handler of '%s'",
        OPERATOR_NOT_ALLOWED => "Script uses disallowed operator: %s",
        SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION => {
            "unsupported serializable parse cache model version: %s"
        }
        SERIALIZABLE_PARSE_CACHE_INVALID_MODEL => "invalid serializable parse cache model: %s",
        SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION => {
            "unsupported serializable parse cache instruction: %s"
        }
        SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT => {
            "unsupported serializable parse cache constant: %s"
        }
        SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND => {
            "class not found when loading serializable parse cache: %s"
        }
        SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND => {
            "operator not found when loading serializable parse cache: %s"
        }
        INVALID_ARGUMENT => "",
        BIZ_EXCEPTION => "",
        QL_THROW => "qlexpress throw statement",
        OPERAND_STACK_OVERFLOW => "operand stack overflow",
        OPERAND_STACK_UNDERFLOW => "operand stack underflow",
        _ => "",
    }
}

/// 按占位符顺序格式化稳定错误消息。
/// 参数：`format`、`args`；返回：`String`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLErrorCodes.java`，方法 `formatMsg`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `String.format` subset used by QLExpress error templates: replaces
/// `%s` / `%d` placeholders in order with the given pre-rendered arguments.
/// Missing placeholders and extra arguments are tolerated (as in Java usage).
/// 对应 Java: 无（Rust 原生适配）。
pub fn format_msg(format: &str, args: &[String]) -> String {
    let mut result = String::with_capacity(format.len() + args.len() * 8);
    let mut arg_iter = args.iter();
    let bytes = format.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 1 < bytes.len() && (bytes[i + 1] == b's' || bytes[i + 1] == b'd')
        {
            match arg_iter.next() {
                Some(arg) => result.push_str(arg),
                None => result.push_str(&format[i..i + 2]),
            }
            i += 2;
        } else {
            // Copy the UTF-8 char starting at i.
            let ch_len = utf8_len(bytes[i]);
            result.push_str(&format[i..i + ch_len]);
            i += ch_len;
        }
    }
    result
}

fn utf8_len(first_byte: u8) -> usize {
    if first_byte < 0x80 {
        1
    } else if first_byte < 0xE0 {
        2
    } else if first_byte < 0xF0 {
        3
    } else {
        4
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOURCE_PARITY: Java `QLErrorCodes#getErrorMsg` 的 63 个枚举项与消息模板
    /// 必须逐项一致；这张表有意作为 Java 枚举的独立测试 oracle 保留。
    #[test]
    fn all_java_error_codes_and_templates_match() {
        let expected = [
            (SYNTAX_ERROR, ""),
            (MISSING_INDEX, "missing index expression"),
            (INVALID_NUMBER, "invalid number"),
            (CLASS_NOT_FOUND, "can not find class: %s"),
            (INVALID_INDEX, "index can only be number"),
            (INDEX_OUT_BOUND, "index out of bound"),
            (NONINDEXABLE_OBJECT, "object of class %s is not indexable"),
            (
                NONTRAVERSABLE_OBJECT,
                "object of class %s is not traversable",
            ),
            (NULL_FIELD_ACCESS, "can not access field from null"),
            (NULL_METHOD_ACCESS, "can not access method from null"),
            (FIELD_NOT_FOUND, "'%s' field not found"),
            (
                SET_FIELD_UNKNOWN_ERROR,
                "unknown error when setting field '%s' value",
            ),
            (
                GET_FIELD_UNKNOWN_ERROR,
                "unknown error when getting field '%s' value",
            ),
            (
                INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
                "invoke method '%s' with wrong arguments",
            ),
            (
                INVOKE_METHOD_INNER_ERROR,
                "exception from inner when invoking method '%s'",
            ),
            (
                INVOKE_METHOD_UNKNOWN_ERROR,
                "unknown error when invoking method '%s'",
            ),
            (
                INVOKE_FUNCTION_INNER_ERROR,
                "exception from inner when invoking function '%s', error message: %s",
            ),
            (FUNCTION_NOT_FOUND, "function '%s' not found"),
            (
                FUNCTION_TYPE_MISMATCH,
                "symbol '%s' is not a function type",
            ),
            (INVOKE_LAMBDA_ERROR, "error when invoking lambda"),
            (NULL_CALL, "can not call null"),
            (OBJECT_NOT_CALLABLE, "type '%s' is not callable"),
            (
                METHOD_NOT_FOUND,
                "no suitable method '%s' found for args %s",
            ),
            (
                INVOKE_CONSTRUCTOR_UNKNOWN_ERROR,
                "unknown error when invoking constructor",
            ),
            (
                INVOKE_CONSTRUCTOR_INNER_ERROR,
                "exception from inner when invoking constructor",
            ),
            (
                NO_SUITABLE_CONSTRUCTOR,
                "no suitable constructor for types %s",
            ),
            (EXECUTE_BLOCK_ERROR, "error when executing block"),
            (
                INCOMPATIBLE_TYPE_CAST,
                "incompatible cast from type: %s to type: %s",
            ),
            (
                INVALID_CAST_TARGET,
                "target for type cast must be a class, but accept %s",
            ),
            (
                SCRIPT_TIME_OUT,
                "script exceeds timeout milliseconds, which is %d ms",
            ),
            (
                INCOMPATIBLE_ASSIGNMENT_TYPE,
                "variable declared type %s, assigned with incompatible value type %s",
            ),
            (
                FOR_EACH_ITERABLE_REQUIRED,
                "for-each can only be applied to iterable",
            ),
            (
                FOR_EACH_TYPE_MISMATCH,
                "for-each type mismatch, required %s, but %s provided",
            ),
            (
                FOR_EACH_UNKNOWN_ERROR,
                "unknown error when executing for-each",
            ),
            (FOR_INIT_ERROR, "error when executing for init"),
            (FOR_BODY_ERROR, "error when executing for body"),
            (FOR_UPDATE_ERROR, "error when executing for update"),
            (
                FOR_CONDITION_ERROR,
                "error when executing for condition",
            ),
            (
                FOR_CONDITION_BOOL_REQUIRED,
                "result of for condition must be bool",
            ),
            (
                WHILE_CONDITION_BOOL_REQUIRED,
                "result of while condition must be bool",
            ),
            (
                WHILE_CONDITION_ERROR,
                "error when executing while condition",
            ),
            (
                CONDITION_BOOL_REQUIRED,
                "result of condition expression must be bool",
            ),
            (ARRAY_SIZE_NUM_REQUIRED, "size of array must be number"),
            (
                EXCEED_MAX_ARR_LENGTH,
                "array length %d, exceed max allowed length %d",
            ),
            (
                INCOMPATIBLE_ARRAY_ITEM_TYPE,
                "item %d with type %s incompatible with array type %s",
            ),
            (INVALID_ASSIGNMENT, "value %s is not assignable"),
            (
                EXECUTE_OPERATOR_EXCEPTION,
                "exception when executing '%s %s %s'",
            ),
            (INVALID_ARITHMETIC, ""),
            (
                INVALID_BINARY_OPERAND,
                "the '%s' operator can not be applied to leftType:%s with leftValue:%s and rightType:%s with rightValue:%s",
            ),
            (
                INVALID_UNARY_OPERAND,
                "the '%s' operator can not be applied to type %s with value %s",
            ),
            (
                EXECUTE_FINAL_BLOCK_ERROR,
                "error when executing final block in try...catch...final...",
            ),
            (
                EXECUTE_TRY_BLOCK_ERROR,
                "error when executing try... block",
            ),
            (
                EXECUTE_CATCH_HANDLER_ERROR,
                "error when executing handler of '%s'",
            ),
            (
                OPERATOR_NOT_ALLOWED,
                "Script uses disallowed operator: %s",
            ),
            (
                SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION,
                "unsupported serializable parse cache model version: %s",
            ),
            (
                SERIALIZABLE_PARSE_CACHE_INVALID_MODEL,
                "invalid serializable parse cache model: %s",
            ),
            (
                SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION,
                "unsupported serializable parse cache instruction: %s",
            ),
            (
                SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT,
                "unsupported serializable parse cache constant: %s",
            ),
            (
                SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND,
                "class not found when loading serializable parse cache: %s",
            ),
            (
                SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND,
                "operator not found when loading serializable parse cache: %s",
            ),
            (INVALID_ARGUMENT, ""),
            (BIZ_EXCEPTION, ""),
            (QL_THROW, "qlexpress throw statement"),
            (OPERAND_STACK_OVERFLOW, "operand stack overflow"),
            (OPERAND_STACK_UNDERFLOW, "operand stack underflow"),
        ];
        assert_eq!(expected.len(), 65);
        for (code, template) in expected {
            assert_eq!(error_msg(code), template, "error code {code}");
        }
        assert_eq!(error_msg("NO_SUCH_CODE"), "");
    }

    #[test]
    fn format_msg_substitutes_in_order() {
        assert_eq!(
            format_msg(error_msg(FUNCTION_NOT_FOUND), &["foo".to_string()]),
            "function 'foo' not found"
        );
        assert_eq!(
            format_msg(error_msg(SCRIPT_TIME_OUT), &["100".to_string()]),
            "script exceeds timeout milliseconds, which is 100 ms"
        );
        assert_eq!(
            format_msg(
                error_msg(INCOMPATIBLE_TYPE_CAST),
                &["Integer".to_string(), "String".to_string()]
            ),
            "incompatible cast from type: Integer to type: String"
        );
        // Missing args leave placeholders intact.
        assert_eq!(format_msg("a %s b %s", &["x".to_string()]), "a x b %s");
        // No placeholders: template unchanged even with args.
        assert_eq!(format_msg("plain", &["x".to_string()]), "plain");
    }
}
