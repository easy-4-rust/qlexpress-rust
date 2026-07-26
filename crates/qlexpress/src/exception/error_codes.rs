//! Error codes, mirroring Java `QLErrorCodes` enum (SPEC §3.4).
//!
//! Every constant keeps the Java name and the original (English) message
//! template verbatim. Templates use Java `String.format` style placeholders
//! (`%s`, `%d`); substitute them with [`format_msg`].

// ---- syntax error ----
pub const SYNTAX_ERROR: &str = "SYNTAX_ERROR";
pub const MISSING_INDEX: &str = "MISSING_INDEX";
pub const INVALID_NUMBER: &str = "INVALID_NUMBER";
pub const CLASS_NOT_FOUND: &str = "CLASS_NOT_FOUND";
/// Stage 6 / FixedSizeStack: emitted when an operand-stack push would
/// exceed the declared capacity (mirrors Java's StackOverflowError).
pub const STACK_OVERFLOW: &str = "STACK_OVERFLOW";

// ---- runtime error ----
pub const INVALID_INDEX: &str = "INVALID_INDEX";
pub const INDEX_OUT_BOUND: &str = "INDEX_OUT_BOUND";
pub const NONINDEXABLE_OBJECT: &str = "NONINDEXABLE_OBJECT";
pub const NONTRAVERSABLE_OBJECT: &str = "NONTRAVERSABLE_OBJECT";
pub const NULL_FIELD_ACCESS: &str = "NULL_FIELD_ACCESS";
pub const NULL_METHOD_ACCESS: &str = "NULL_METHOD_ACCESS";
pub const FIELD_NOT_FOUND: &str = "FIELD_NOT_FOUND";
pub const SET_FIELD_UNKNOWN_ERROR: &str = "SET_FIELD_UNKNOWN_ERROR";
pub const GET_FIELD_UNKNOWN_ERROR: &str = "GET_FIELD_UNKNOWN_ERROR";
pub const INVOKE_METHOD_WITH_WRONG_ARGUMENTS: &str = "INVOKE_METHOD_WITH_WRONG_ARGUMENTS";
pub const INVOKE_METHOD_INNER_ERROR: &str = "INVOKE_METHOD_INNER_ERROR";
pub const INVOKE_METHOD_UNKNOWN_ERROR: &str = "INVOKE_METHOD_UNKNOWN_ERROR";
pub const INVOKE_FUNCTION_INNER_ERROR: &str = "INVOKE_FUNCTION_INNER_ERROR";
pub const FUNCTION_NOT_FOUND: &str = "FUNCTION_NOT_FOUND";
pub const FUNCTION_TYPE_MISMATCH: &str = "FUNCTION_TYPE_MISMATCH";
pub const INVOKE_LAMBDA_ERROR: &str = "INVOKE_LAMBDA_ERROR";
pub const NULL_CALL: &str = "NULL_CALL";
pub const OBJECT_NOT_CALLABLE: &str = "OBJECT_NOT_CALLABLE";
pub const METHOD_NOT_FOUND: &str = "METHOD_NOT_FOUND";
pub const INVOKE_CONSTRUCTOR_UNKNOWN_ERROR: &str = "INVOKE_CONSTRUCTOR_UNKNOWN_ERROR";
pub const INVOKE_CONSTRUCTOR_INNER_ERROR: &str = "INVOKE_CONSTRUCTOR_INNER_ERROR";
pub const NO_SUITABLE_CONSTRUCTOR: &str = "NO_SUITABLE_CONSTRUCTOR";
pub const EXECUTE_BLOCK_ERROR: &str = "EXECUTE_BLOCK_ERROR";
pub const INCOMPATIBLE_TYPE_CAST: &str = "INCOMPATIBLE_TYPE_CAST";
pub const INVALID_CAST_TARGET: &str = "INVALID_CAST_TARGET";
pub const SCRIPT_TIME_OUT: &str = "SCRIPT_TIME_OUT";
pub const INCOMPATIBLE_ASSIGNMENT_TYPE: &str = "INCOMPATIBLE_ASSIGNMENT_TYPE";
pub const FOR_EACH_ITERABLE_REQUIRED: &str = "FOR_EACH_ITERABLE_REQUIRED";
pub const FOR_EACH_TYPE_MISMATCH: &str = "FOR_EACH_TYPE_MISMATCH";
pub const FOR_EACH_UNKNOWN_ERROR: &str = "FOR_EACH_UNKNOWN_ERROR";
pub const FOR_INIT_ERROR: &str = "FOR_INIT_ERROR";
pub const FOR_BODY_ERROR: &str = "FOR_BODY_ERROR";
pub const FOR_UPDATE_ERROR: &str = "FOR_UPDATE_ERROR";
pub const FOR_CONDITION_ERROR: &str = "FOR_CONDITION_ERROR";
pub const FOR_CONDITION_BOOL_REQUIRED: &str = "FOR_CONDITION_BOOL_REQUIRED";
pub const WHILE_CONDITION_BOOL_REQUIRED: &str = "WHILE_CONDITION_BOOL_REQUIRED";
pub const WHILE_CONDITION_ERROR: &str = "WHILE_CONDITION_ERROR";
pub const CONDITION_BOOL_REQUIRED: &str = "CONDITION_BOOL_REQUIRED";
pub const ARRAY_SIZE_NUM_REQUIRED: &str = "ARRAY_SIZE_NUM_REQUIRED";
pub const EXCEED_MAX_ARR_LENGTH: &str = "EXCEED_MAX_ARR_LENGTH";
pub const INCOMPATIBLE_ARRAY_ITEM_TYPE: &str = "INCOMPATIBLE_ARRAY_ITEM_TYPE";
pub const INVALID_ASSIGNMENT: &str = "INVALID_ASSIGNMENT";
pub const EXECUTE_OPERATOR_EXCEPTION: &str = "EXECUTE_OPERATOR_EXCEPTION";
pub const INVALID_ARITHMETIC: &str = "INVALID_ARITHMETIC";
pub const INVALID_BINARY_OPERAND: &str = "INVALID_BINARY_OPERAND";
pub const INVALID_UNARY_OPERAND: &str = "INVALID_UNARY_OPERAND";
pub const EXECUTE_FINAL_BLOCK_ERROR: &str = "EXECUTE_FINAL_BLOCK_ERROR";
pub const EXECUTE_TRY_BLOCK_ERROR: &str = "EXECUTE_TRY_BLOCK_ERROR";
pub const EXECUTE_CATCH_HANDLER_ERROR: &str = "EXECUTE_CATCH_HANDLER_ERROR";

// ---- operator restriction error ----
pub const OPERATOR_NOT_ALLOWED: &str = "OPERATOR_NOT_ALLOWED";

// ---- serializable parse cache error ----
pub const SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION: &str =
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_VERSION";
pub const SERIALIZABLE_PARSE_CACHE_INVALID_MODEL: &str = "SERIALIZABLE_PARSE_CACHE_INVALID_MODEL";
pub const SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION: &str =
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_INSTRUCTION";
pub const SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT: &str =
    "SERIALIZABLE_PARSE_CACHE_UNSUPPORTED_CONSTANT";
pub const SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND: &str =
    "SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND";
pub const SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND: &str =
    "SERIALIZABLE_PARSE_CACHE_OPERATOR_NOT_FOUND";

// ---- user defined exception ----
pub const INVALID_ARGUMENT: &str = "INVALID_ARGUMENT";
pub const BIZ_EXCEPTION: &str = "BIZ_EXCEPTION";
pub const QL_THROW: &str = "QL_THROW";

/// 处理 error msg 对应的领域职责。
/// 参数：`code`；返回：`&'static str`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLErrorCodes.java`，方法 `errorMsg`；Rust 侧按所有权与 `Result` 语义适配。
/// Returns the Java message template for an error code (verbatim from
/// `QLErrorCodes`). Unknown codes yield an empty string, mirroring the
/// empty-message codes in Java.
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
        _ => "",
    }
}

/// 处理 format msg 对应的领域职责。
/// 参数：`format`、`args`；返回：`String`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/QLErrorCodes.java`，方法 `formatMsg`；Rust 侧按所有权与 `Result` 语义适配。
/// Java `String.format` subset used by QLExpress error templates: replaces
/// `%s` / `%d` placeholders in order with the given pre-rendered arguments.
/// Missing placeholders and extra arguments are tolerated (as in Java usage).
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

    #[test]
    fn all_templates_match_codes() {
        // Spot-check verbatim Java templates.
        assert_eq!(error_msg(FUNCTION_NOT_FOUND), "function '%s' not found");
        assert_eq!(
            error_msg(SCRIPT_TIME_OUT),
            "script exceeds timeout milliseconds, which is %d ms"
        );
        assert_eq!(error_msg(SYNTAX_ERROR), "");
        assert_eq!(error_msg(QL_THROW), "qlexpress throw statement");
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
