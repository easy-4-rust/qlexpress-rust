//! QLFunction name helpers, mirroring Java `QLFunctionUtil`.
//!
//! Java inspects the `@QLFunction` annotation reflectively; in Rust the
//! function names declared for a native method are supplied explicitly at
//! registration time (SPEC §4), so this helper operates on an optional
//! name list.

pub struct QLFunctionUtil;

impl QLFunctionUtil {
    /// Java `getQLFunctionValue`: the names declared for the method.
    /// Returns `None` when the method carries no `QLFunction` names.
    pub fn get_ql_function_value(ql_function_names: Option<&[String]>) -> Option<&[String]> {
        ql_function_names
    }

    /// Java `containsQLFunctionForMethod`: whether any `QLFunction` names
    /// were declared for the method.
    pub fn contains_ql_function_for_method(ql_function_names: Option<&[String]>) -> bool {
        ql_function_names.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_and_values() {
        let names = vec!["add".to_string(), "sum".to_string()];
        assert!(QLFunctionUtil::contains_ql_function_for_method(Some(
            &names
        )));
        assert_eq!(
            QLFunctionUtil::get_ql_function_value(Some(&names)),
            Some(&names[..])
        );
        assert!(!QLFunctionUtil::contains_ql_function_for_method(None));
        assert_eq!(QLFunctionUtil::get_ql_function_value(None), None);
    }
}
