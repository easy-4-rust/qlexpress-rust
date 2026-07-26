//! Operator lookup for the instruction-generating visitor, mirroring Java
//! `OperatorFactory` (interface) and `OperatorManager` (implementation).
//!
//! Java keeps the built-in operator singletons in static maps
//! (`DEFAULT_BINARY_OPERATOR_MAP`, ...). The Rust port keeps them in the
//! [`OperatorManager`] instance; Stage 4 populates them through
//! [`OperatorManager::register_default_binary_operator`] and friends. The
//! lookup/custom-operator/alias semantics are fully ported here.

use std::collections::HashMap;
use std::rc::Rc;

use super::parser_operator_manager::{OpType, ParserOperatorManager};
use super::token;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::operator::base::{BinaryOperator, UnaryOperator};
use crate::runtime::operator::custom_binary_operator::CustomBinaryOperator;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

/// Java `OperatorFactory`: the compile-time operator lookup used by
/// `QvmInstructionVisitor`.
pub trait OperatorFactory {
    /// Java `getBinaryOperator(String)`; `None` plays Java's `null`.
    fn get_binary_operator(&self, operator_lexeme: &str) -> Option<Rc<dyn BinaryOperator>>;

    /// Java `getPrefixUnaryOperator(String)`.
    fn get_prefix_unary_operator(&self, operator_lexeme: &str) -> Option<Rc<dyn UnaryOperator>>;

    /// Java `getSuffixUnaryOperator(String)`.
    fn get_suffix_unary_operator(&self, operator_lexeme: &str) -> Option<Rc<dyn UnaryOperator>>;
}

/// Java `OperatorManager.adapt2BinOp`: wraps a [`CustomBinaryOperator]`
/// into a [`BinaryOperator`], converting user errors into reported
/// `OPERATOR_INNER_EXCEPTION` failures (Java `ThrowUtils.wrapThrowable`).
struct CustomBinaryOperatorAdapter {
    operator_name: String,
    custom_binary_operator: Rc<dyn CustomBinaryOperator>,
    priority: i32,
}

impl BinaryOperator for CustomBinaryOperatorAdapter {
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        self.custom_binary_operator
            .execute(left, right)
            .map_err(|err| {
                // Java: UserDefineException -> reportUserDefinedException;
                // other Throwable -> OPERATOR_INNER_EXCEPTION. The Rust
                // port carries the user message in the QLException reason.
                error_reporter.report("OPERATOR_INNER_EXCEPTION", &format!(
                    "custom operator '{}' inner exception: {}",
                    self.operator_name,
                    err.reason()
                ))
            })
    }

    fn operator(&self) -> &str {
        &self.operator_name
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// Java `OperatorManager.adaptOriginOperator`: re-lexemes an existing
/// operator under an alias (`addOperatorAlias`).
struct AliasedBinaryOperator {
    lexeme: String,
    origin: Rc<dyn BinaryOperator>,
}

impl BinaryOperator for AliasedBinaryOperator {
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
        error_reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        self.origin
            .execute(left, right, q_context, ql_options, error_reporter)
    }

    fn operator(&self) -> &str {
        &self.lexeme
    }

    fn priority(&self) -> i32 {
        self.origin.priority()
    }
}

/// Java `OperatorManager`: built-in operator tables plus user-registered
/// custom operators, operator aliases and keyword aliases.
///
/// Unlike Java (static default maps), the default tables live in the
/// instance and start empty; Stage 4 registers the built-in operators.
#[derive(Default)]
pub struct OperatorManager {
    /// Java `DEFAULT_BINARY_OPERATOR_MAP`.
    default_binary_operator_map: HashMap<String, Rc<dyn BinaryOperator>>,
    /// Java `DEFAULT_PREFIX_UNARY_OPERATOR_MAP`.
    default_prefix_unary_operator_map: HashMap<String, Rc<dyn UnaryOperator>>,
    /// Java `DEFAULT_SUFFIX_UNARY_OPERATOR_MAP`.
    default_suffix_unary_operator_map: HashMap<String, Rc<dyn UnaryOperator>>,
    /// Java `customBinaryOperatorMap`.
    custom_binary_operator_map: HashMap<String, Rc<dyn BinaryOperator>>,
    /// Java `keyWordAliases`.
    key_word_aliases: HashMap<String, i32>,
}

impl OperatorManager {
    /// Java `new OperatorManager()` (with as-yet unpopulated default
    /// tables; see the module docs).
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a built-in binary operator (Stage 4 fills the table that
    /// Java initializes in the static block).
    pub fn register_default_binary_operator(&mut self, operator: Rc<dyn BinaryOperator>) {
        self.default_binary_operator_map
            .insert(operator.operator().to_string(), operator);
    }

    /// Register a built-in prefix unary operator (Stage 4).
    pub fn register_default_prefix_unary_operator(&mut self, operator: Rc<dyn UnaryOperator>) {
        self.default_prefix_unary_operator_map
            .insert(operator.operator().to_string(), operator);
    }

    /// Register a built-in suffix unary operator (Stage 4).
    pub fn register_default_suffix_unary_operator(&mut self, operator: Rc<dyn UnaryOperator>) {
        self.default_suffix_unary_operator_map
            .insert(operator.operator().to_string(), operator);
    }

    /// Java `addBinaryOperator`: register a custom binary operator unless
    /// the name clashes with a built-in operator.
    pub fn add_binary_operator(
        &mut self,
        operator_name: impl Into<String>,
        custom_binary_operator: Rc<dyn CustomBinaryOperator>,
        priority: i32,
    ) -> bool {
        let operator_name = operator_name.into();
        if self.default_binary_operator_map.contains_key(&operator_name) {
            return false;
        }
        if self.custom_binary_operator_map.contains_key(&operator_name) {
            return false;
        }
        self.custom_binary_operator_map.insert(
            operator_name.clone(),
            Rc::new(CustomBinaryOperatorAdapter {
                operator_name,
                custom_binary_operator,
                priority,
            }),
        );
        true
    }

    /// Java `replaceDefaultOperator`.
    pub fn replace_default_operator(
        &mut self,
        operator_name: &str,
        custom_binary_operator: Rc<dyn CustomBinaryOperator>,
    ) -> bool {
        let Some(default_operator) = self.default_binary_operator_map.get(operator_name) else {
            return false;
        };
        let priority = default_operator.priority();
        if self.custom_binary_operator_map.contains_key(operator_name) {
            return false;
        }
        self.custom_binary_operator_map.insert(
            operator_name.to_string(),
            Rc::new(CustomBinaryOperatorAdapter {
                operator_name: operator_name.to_string(),
                custom_binary_operator,
                priority,
            }),
        );
        true
    }

    /// Java `addOperatorAlias`: give an existing operator a new lexeme.
    pub fn add_operator_alias(&mut self, lexeme: impl Into<String>, operator: &str) -> bool {
        let lexeme = lexeme.into();
        let origin = self
            .default_binary_operator_map
            .get(operator)
            .or_else(|| self.custom_binary_operator_map.get(operator));
        let Some(origin) = origin else {
            return false;
        };
        if self.custom_binary_operator_map.contains_key(&lexeme) {
            return false;
        }
        self.custom_binary_operator_map.insert(
            lexeme.clone(),
            Rc::new(AliasedBinaryOperator {
                lexeme,
                origin: Rc::clone(origin),
            }),
        );
        true
    }

    /// Java `addKeyWordAlias`: alias a keyword (`if`, `while`, ...) so the
    /// lexer maps `lexeme` to the keyword token type.
    pub fn add_key_word_alias(&mut self, lexeme: impl Into<String>, key_word: &str) -> bool {
        let key_word_id = aliasable_keyword_id(key_word);
        match key_word_id {
            Some(id) => {
                self.key_word_aliases.insert(lexeme.into(), id);
                true
            }
            None => false,
        }
    }
}

/// Java `OperatorManager.ALIASABLE_KEYWORDS`.
fn aliasable_keyword_id(key_word: &str) -> Option<i32> {
    let id = match key_word {
        "if" => token::IF,
        "then" => token::THEN,
        "else" => token::ELSE,
        "for" => token::FOR,
        "while" => token::WHILE,
        "break" => token::BREAK,
        "continue" => token::CONTINUE,
        "return" => token::RETURN,
        "function" => token::FUNCTION,
        "macro" => token::MACRO,
        "new" => token::NEW,
        "null" => token::NULL,
        "true" => token::TRUE,
        "false" => token::FALSE,
        _ => return None,
    };
    Some(id as i32)
}

impl OperatorFactory for OperatorManager {
    /// Java `getBinaryOperator`: custom operators shadow built-ins.
    fn get_binary_operator(&self, operator_lexeme: &str) -> Option<Rc<dyn BinaryOperator>> {
        if let Some(custom) = self.custom_binary_operator_map.get(operator_lexeme) {
            return Some(Rc::clone(custom));
        }
        self.default_binary_operator_map
            .get(operator_lexeme)
            .map(Rc::clone)
    }

    fn get_prefix_unary_operator(&self, operator_lexeme: &str) -> Option<Rc<dyn UnaryOperator>> {
        self.default_prefix_unary_operator_map
            .get(operator_lexeme)
            .map(Rc::clone)
    }

    fn get_suffix_unary_operator(&self, operator_lexeme: &str) -> Option<Rc<dyn UnaryOperator>> {
        self.default_suffix_unary_operator_map
            .get(operator_lexeme)
            .map(Rc::clone)
    }
}

impl ParserOperatorManager for OperatorManager {
    /// Java `isOpType`.
    fn is_op_type(&self, lexeme: &str, op_type: OpType) -> bool {
        match op_type {
            OpType::Middle => self.get_binary_operator(lexeme).is_some(),
            OpType::Prefix => self
                .default_prefix_unary_operator_map
                .contains_key(lexeme),
            OpType::Suffix => self
                .default_suffix_unary_operator_map
                .contains_key(lexeme),
        }
    }

    /// Java `precedence` (`getBinaryOperator(lexeme).getPriority()`).
    fn precedence(&self, lexeme: &str) -> Option<i32> {
        self.get_binary_operator(lexeme).map(|op| op.priority())
    }

    /// Java `getAlias`.
    fn get_alias(&self, lexeme: &str) -> Option<i32> {
        self.key_word_aliases.get(lexeme).copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exception::pure_err_reporter::PureErrReporter;
    use crate::runtime::qvm_runtime::QvmRuntime;
    use crate::runtime::member::NativeRegistry;

    struct Add;

    impl BinaryOperator for Add {
        fn execute(
            &self,
            left: &QValue,
            right: &QValue,
            _ctx: &mut dyn QContext,
            _opts: &QLOptions,
            _reporter: &dyn ErrorReporter,
        ) -> Result<DataValue, QLException> {
            match (left.get(), right.get()) {
                (DataValue::Int(l), DataValue::Int(r)) => Ok(DataValue::Int(l + r)),
                _ => Ok(DataValue::Null),
            }
        }

        fn operator(&self) -> &str {
            "+"
        }

        fn priority(&self) -> i32 {
            200
        }
    }

    struct Pow;

    impl CustomBinaryOperator for Pow {
        fn execute(&self, left: &QValue, right: &QValue) -> Result<DataValue, QLException> {
            match (left.get(), right.get()) {
                (DataValue::Int(l), DataValue::Int(r)) => Ok(DataValue::Int(l.pow(r as u32))),
                _ => Ok(DataValue::Null),
            }
        }
    }

    fn test_ctx() -> (QvmRuntime, QLOptions) {
        (
            QvmRuntime::for_test(Rc::new(NativeRegistry::with_builtins())),
            QLOptions::builder().build(),
        )
    }

    #[test]
    fn lookup_prefers_custom_over_default() {
        let mut manager = OperatorManager::new();
        manager.register_default_binary_operator(Rc::new(Add));
        assert_eq!(manager.get_binary_operator("+").unwrap().priority(), 200);
        assert!(manager.precedence("+") == Some(200));
        assert!(manager.is_op_type("+", OpType::Middle));
        assert!(!manager.is_op_type("+", OpType::Prefix));

        // clash with built-in rejected
        assert!(!manager.add_binary_operator("+", Rc::new(Pow), 300));
        // new lexeme accepted
        assert!(manager.add_binary_operator("**", Rc::new(Pow), 300));
        assert_eq!(manager.get_binary_operator("**").unwrap().priority(), 300);
        // duplicate rejected
        assert!(!manager.add_binary_operator("**", Rc::new(Pow), 300));

        let (runtime, opts) = test_ctx();
        let mut ctx = crate::runtime::delegate_qcontext::DelegateQContext::new(
            Rc::new(runtime),
            crate::runtime::scope::QScope::global(crate::runtime::qvm_global_scope::QvmGlobalScope::empty()),
        );
        let result = manager
            .get_binary_operator("**")
            .unwrap()
            .execute(
                &QValue::Data(DataValue::Int(2)),
                &QValue::Data(DataValue::Int(3)),
                &mut ctx,
                &opts,
                &PureErrReporter::INSTANCE,
            )
            .unwrap();
        assert_eq!(result, DataValue::Int(8));
    }

    #[test]
    fn operator_alias_delegates_to_origin() {
        let mut manager = OperatorManager::new();
        manager.register_default_binary_operator(Rc::new(Add));
        assert!(manager.add_operator_alias("plus", "+"));
        let alias = manager.get_binary_operator("plus").unwrap();
        assert_eq!(alias.operator(), "plus");
        assert_eq!(alias.priority(), 200);
        assert!(!manager.add_operator_alias("plus", "+"));
        assert!(!manager.add_operator_alias("x", "missing"));
    }

    #[test]
    fn replace_default_keeps_priority() {
        let mut manager = OperatorManager::new();
        manager.register_default_binary_operator(Rc::new(Add));
        assert!(manager.replace_default_operator("+", Rc::new(Pow)));
        assert_eq!(manager.get_binary_operator("+").unwrap().operator(), "+");
        assert_eq!(manager.get_binary_operator("+").unwrap().priority(), 200);
        assert!(!manager.replace_default_operator("?", Rc::new(Pow)));
    }

    #[test]
    fn keyword_aliases() {
        let mut manager = OperatorManager::new();
        assert!(manager.add_key_word_alias("ruo", "if"));
        assert_eq!(manager.get_alias("ruo"), Some(token::IF as i32));
        assert!(!manager.add_key_word_alias("x", "not-a-keyword"));
        assert_eq!(manager.get_alias("x"), None);
    }
}
