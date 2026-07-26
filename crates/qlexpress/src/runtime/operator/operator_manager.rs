//! 操作符管理器:内建操作符表 + 自定义操作符注册。
//!
//! 对应 Java: com.alibaba.qlexpress4.runtime.operator.OperatorManager
//! (implements OperatorFactory, ParserOperatorManager;静态块中注册全部
//! 内建操作符,实例字段持有自定义操作符与别名表)。
//!
//! Rust 说明:Java 用 static 块初始化 DEFAULT_* 三张表;Rust 在
//! [`OperatorManager::new`] 中构建(对齐 Java 构造即可用的语义)。
//! 本 agent 交付 arithmetic/bit/compare/logic/instanceof 共 26 个二元
//! 操作符与前缀一元 `~`、`!`;assign(=)、collection(in/not in)、
//! string(like/not like)、unary(++/‑‑/单目+-)由其他 Stage 4 agent
//! 交付,本文件统一注册(Java static 块全量清单)。

use std::collections::HashMap;
use std::rc::Rc;

use crate::aparser::operator_factory::{
    OperatorFactory, OperatorManager as AParserOperatorManager,
};
use crate::aparser::parser_operator_manager::{OpType, ParserOperatorManager};
use crate::aparser::token;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};

use super::base::{BinaryOperator, UnaryOperator};
use super::custom_binary_operator::CustomBinaryOperator;

use super::arithmetic::{
    DivideAssignOperator, DivideOperator, MinusAssignOperator, MinusOperator,
    MultiplyAssignOperator, MultiplyOperator, PlusAssignOperator, PlusOperator,
    RemainderAssignOperator, RemainderOperator,
};
use super::assign::AssignOperator;
use super::bit::{
    BitwiseAndAssignOperator, BitwiseAndOperator, BitwiseInvertOperator,
    BitwiseLeftShiftAssignOperator, BitwiseLeftShiftOperator, BitwiseOrAssignOperator,
    BitwiseOrOperator, BitwiseRightShiftAssignOperator, BitwiseRightShiftOperator,
    BitwiseRightShiftUnsignedAssignOperator, BitwiseRightShiftUnsignedOperator,
    BitwiseXorAssignOperator, BitwiseXorOperator,
};
use super::collection::{InOperator, NotInOperator};
use super::compare::{
    EqualOperator, GreaterEqualOperator, GreaterOperator, LessEqualOperator, LessOperator,
    UnequalOperator,
};
use super::instance_of_operator::InstanceOfOperator;
use super::logic::{LogicAndOperator, LogicNotOperator, LogicOrOperator};
use super::string::{LikeOperator, NotLikeOperator};
use super::unary::{
    MinusMinusPrefixUnaryOperator, MinusMinusSuffixUnaryOperator, MinusUnaryOperator,
    PlusPlusPrefixUnaryOperator, PlusPlusSuffixUnaryOperator, PlusUnaryOperator,
};

/// Java `OperatorManager.adapt2BinOp`:把 CustomBinaryOperator 适配成
/// BinaryOperator,用户异常经 `ThrowUtils` 包装为
/// OPERATOR_INNER_EXCEPTION(用户消息保留在 reason 中)。
struct CustomBinaryOperatorAdapter {
    operator_name: String,
    custom_binary_operator: Rc<dyn CustomBinaryOperator>,
    priority: i32,
}

impl BinaryOperator for CustomBinaryOperatorAdapter {
    /// 对应 Java 方法: 匿名类 `execute(...)`(catch UserDefineException /
    /// Throwable 两条路径在 Rust 合并为 QLException 包装)。
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
                error_reporter.report(
                    "OPERATOR_INNER_EXCEPTION",
                    &format!(
                        "custom operator '{}' inner exception: {}",
                        self.operator_name,
                        err.reason()
                    ),
                )
            })
    }

    fn operator(&self) -> &str {
        &self.operator_name
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

/// Java `OperatorManager.adaptOriginOperator`:为既有操作符起新词素
/// (`addOperatorAlias`),执行逻辑委托原操作符。
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

/// 对应 Java: OperatorManager(内建表 + 自定义表 + 关键字别名表)。
#[derive(Default)]
pub struct OperatorManager {
    /// Java `DEFAULT_BINARY_OPERATOR_MAP`。
    default_binary_operator_map: HashMap<String, Rc<dyn BinaryOperator>>,
    /// Java `DEFAULT_PREFIX_UNARY_OPERATOR_MAP`。
    default_prefix_unary_operator_map: HashMap<String, Rc<dyn UnaryOperator>>,
    /// Java `DEFAULT_SUFFIX_UNARY_OPERATOR_MAP`。
    default_suffix_unary_operator_map: HashMap<String, Rc<dyn UnaryOperator>>,
    /// Java `customBinaryOperatorMap`。
    custom_binary_operator_map: HashMap<String, Rc<dyn BinaryOperator>>,
    /// Java `keyWordAliases`。
    key_word_aliases: HashMap<String, i32>,
}

impl OperatorManager {
    /// 对应 Java `OperatorManager` 构造 + static 初始化块:构建并注册
    /// 全部内建操作符(Stage 4a 部分,见模块头注释)。
    pub fn new() -> Self {
        let mut manager = OperatorManager::default();
        for operator in default_binary_operators() {
            manager
                .default_binary_operator_map
                .insert(operator.operator().to_string(), operator);
        }
        for operator in default_prefix_unary_operators() {
            manager
                .default_prefix_unary_operator_map
                .insert(operator.operator().to_string(), operator);
        }
        for operator in default_suffix_unary_operators() {
            manager
                .default_suffix_unary_operator_map
                .insert(operator.operator().to_string(), operator);
        }
        manager
    }

    /// Java `addBinaryOperator`:词素未被内建/已注册自定义占用时注册
    /// 成功,返回 true。
    pub fn add_binary_operator(
        &mut self,
        operator_name: impl Into<String>,
        custom_binary_operator: Rc<dyn CustomBinaryOperator>,
        priority: i32,
    ) -> bool {
        let operator_name = operator_name.into();
        if self
            .default_binary_operator_map
            .contains_key(&operator_name)
        {
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

    /// Java `replaceDefaultOperator`:仅当词素是内建操作符时替换成功,
    /// 沿用原操作符优先级。
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

    /// Java `addOperatorAlias`:为既有(内建或自定义)操作符注册新词素。
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

    /// Java `addKeyWordAlias`:`lexeme` 映射到关键字 token 类型。
    pub fn add_key_word_alias(&mut self, lexeme: impl Into<String>, key_word: &str) -> bool {
        match aliasable_keyword_id(key_word) {
            Some(id) => {
                self.key_word_aliases.insert(lexeme.into(), id);
                true
            }
            None => false,
        }
    }

    /// 供其他 Stage 4 agent 补注册内建二元操作符(assign/collection/
    /// string 等,对齐 Java static 块的完整清单)。
    pub fn register_default_binary_operator(&mut self, operator: Rc<dyn BinaryOperator>) {
        self.default_binary_operator_map
            .insert(operator.operator().to_string(), operator);
    }

    /// 供其他 Stage 4 agent 补注册前缀一元操作符。
    pub fn register_default_prefix_unary_operator(&mut self, operator: Rc<dyn UnaryOperator>) {
        self.default_prefix_unary_operator_map
            .insert(operator.operator().to_string(), operator);
    }

    /// 供其他 Stage 4 agent 补注册后缀一元操作符。
    pub fn register_default_suffix_unary_operator(&mut self, operator: Rc<dyn UnaryOperator>) {
        self.default_suffix_unary_operator_map
            .insert(operator.operator().to_string(), operator);
    }

    /// 把本管理器的内建操作符灌入 aparser 的 OperatorManager
    /// (aparser 侧表默认空,Stage 4 负责填充;本函数即 Java static
    /// 块语义向编译期组件的投影)。
    pub fn populate_aparser_operator_manager(&self, target: &mut AParserOperatorManager) {
        for operator in self.default_binary_operator_map.values() {
            target.register_default_binary_operator(Rc::clone(operator));
        }
        for operator in self.default_prefix_unary_operator_map.values() {
            target.register_default_prefix_unary_operator(Rc::clone(operator));
        }
        for operator in self.default_suffix_unary_operator_map.values() {
            target.register_default_suffix_unary_operator(Rc::clone(operator));
        }
    }
}

impl OperatorFactory for OperatorManager {
    /// Java `getBinaryOperator`:自定义操作符优先于内建。
    fn get_binary_operator(&self, operator_lexeme: &str) -> Option<Rc<dyn BinaryOperator>> {
        if let Some(custom) = self.custom_binary_operator_map.get(operator_lexeme) {
            return Some(Rc::clone(custom));
        }
        self.default_binary_operator_map
            .get(operator_lexeme)
            .map(Rc::clone)
    }

    /// Java `getPrefixUnaryOperator`(`--1 ++1 !true ~1 ^1`)。
    fn get_prefix_unary_operator(&self, operator_lexeme: &str) -> Option<Rc<dyn UnaryOperator>> {
        self.default_prefix_unary_operator_map
            .get(operator_lexeme)
            .map(Rc::clone)
    }

    /// Java `getSuffixUnaryOperator`(`1-- 1++`)。
    fn get_suffix_unary_operator(&self, operator_lexeme: &str) -> Option<Rc<dyn UnaryOperator>> {
        self.default_suffix_unary_operator_map
            .get(operator_lexeme)
            .map(Rc::clone)
    }
}

impl ParserOperatorManager for OperatorManager {
    /// Java `isOpType`。
    fn is_op_type(&self, lexeme: &str, op_type: OpType) -> bool {
        match op_type {
            OpType::Middle => self.get_binary_operator(lexeme).is_some(),
            OpType::Prefix => self.default_prefix_unary_operator_map.contains_key(lexeme),
            OpType::Suffix => self.default_suffix_unary_operator_map.contains_key(lexeme),
        }
    }

    /// Java `precedence`(`getBinaryOperator(lexeme).getPriority()`)。
    fn precedence(&self, lexeme: &str) -> Option<i32> {
        self.get_binary_operator(lexeme).map(|op| op.priority())
    }

    /// Java `getAlias`。
    fn get_alias(&self, lexeme: &str) -> Option<i32> {
        self.key_word_aliases.get(lexeme).copied()
    }
}

/// Java `OperatorManager.ALIASABLE_KEYWORDS` 静态表。
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

/// Java static 块 `binaryOperatorList` 全量(注册顺序与 Java 一致;
/// assign/collection/string 三项来自其他 Stage 4 agent 的目录)。
pub fn default_binary_operators() -> Vec<Rc<dyn BinaryOperator>> {
    vec![
        Rc::new(AssignOperator::get_instance()),
        Rc::new(PlusOperator::get_instance()),
        Rc::new(PlusAssignOperator::get_instance()),
        Rc::new(MinusOperator::get_instance()),
        Rc::new(MinusAssignOperator::get_instance()),
        Rc::new(MultiplyOperator::get_instance()),
        Rc::new(MultiplyAssignOperator::get_instance()),
        Rc::new(DivideOperator::get_instance()),
        Rc::new(DivideAssignOperator::get_instance()),
        // Java: RemainderOperator.getInstance("%")
        Rc::new(RemainderOperator::get_instance("%").expect("'%' 已预注册")),
        Rc::new(RemainderAssignOperator::get_instance()),
        // Java 注释掉的:RemainderOperator.getInstance("mod")
        Rc::new(BitwiseAndOperator::get_instance()),
        Rc::new(BitwiseAndAssignOperator::get_instance()),
        Rc::new(BitwiseOrOperator::get_instance()),
        Rc::new(BitwiseOrAssignOperator::get_instance()),
        Rc::new(BitwiseXorOperator::get_instance()),
        Rc::new(BitwiseXorAssignOperator::get_instance()),
        Rc::new(BitwiseLeftShiftOperator::get_instance()),
        Rc::new(BitwiseLeftShiftAssignOperator::get_instance()),
        Rc::new(BitwiseRightShiftOperator::get_instance()),
        Rc::new(BitwiseRightShiftAssignOperator::get_instance()),
        Rc::new(BitwiseRightShiftUnsignedOperator::get_instance()),
        Rc::new(BitwiseRightShiftUnsignedAssignOperator::get_instance()),
        Rc::new(LogicAndOperator::get_instance("&&").expect("'&&' 已预注册")),
        Rc::new(LogicAndOperator::get_instance("and").expect("'and' 已预注册")),
        Rc::new(LogicOrOperator::get_instance("||").expect("'||' 已预注册")),
        Rc::new(LogicOrOperator::get_instance("or").expect("'or' 已预注册")),
        Rc::new(EqualOperator::get_instance()),
        Rc::new(UnequalOperator::get_instance("!=").expect("'!=' 已预注册")),
        Rc::new(UnequalOperator::get_instance("<>").expect("'<>' 已预注册")),
        // Java 注释掉的:PrismaticUnequalOperator.getInstance()
        Rc::new(GreaterOperator::get_instance()),
        Rc::new(GreaterEqualOperator::get_instance()),
        Rc::new(LessOperator::get_instance()),
        Rc::new(LessEqualOperator::get_instance()),
        Rc::new(InOperator::get_instance()),
        Rc::new(NotInOperator::get_instance()),
        Rc::new(LikeOperator::get_instance()),
        Rc::new(NotLikeOperator::get_instance()),
        Rc::new(InstanceOfOperator::get_instance()),
    ]
}

/// Java static 块 `prefixUnaryOperatorList` 全量(顺序与 Java 一致)。
pub fn default_prefix_unary_operators() -> Vec<Rc<dyn UnaryOperator>> {
    vec![
        Rc::new(PlusUnaryOperator::get_instance()),
        Rc::new(MinusUnaryOperator::get_instance()),
        Rc::new(PlusPlusPrefixUnaryOperator::get_instance()),
        Rc::new(MinusMinusPrefixUnaryOperator::get_instance()),
        Rc::new(BitwiseInvertOperator::get_instance()),
        Rc::new(LogicNotOperator::get_instance()),
    ]
}

/// Java static 块 `suffixUnaryOperatorList` 全量。
pub fn default_suffix_unary_operators() -> Vec<Rc<dyn UnaryOperator>> {
    vec![
        Rc::new(PlusPlusSuffixUnaryOperator::get_instance()),
        Rc::new(MinusMinusSuffixUnaryOperator::get_instance()),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ql_precedences;

    struct Pow;

    impl CustomBinaryOperator for Pow {
        fn execute(&self, left: &QValue, right: &QValue) -> Result<DataValue, QLException> {
            match (left.get(), right.get()) {
                (DataValue::Int(l), DataValue::Int(r)) => Ok(DataValue::Int(l.pow(r as u32))),
                _ => Ok(DataValue::Null),
            }
        }
    }

    #[test]
    fn default_table_aligns_java_static_block() {
        let manager = OperatorManager::new();
        // Java static 块的 Stage 4a 词素全部可查,优先级正确。
        let cases: &[(&str, i32)] = &[
            ("+", ql_precedences::ADD),
            ("-", ql_precedences::ADD),
            ("*", ql_precedences::MULTI),
            ("/", ql_precedences::MULTI),
            ("%", ql_precedences::MULTI),
            ("+=", ql_precedences::ASSIGN),
            ("&", ql_precedences::BIT_AND),
            ("|", ql_precedences::BIT_OR),
            ("^", ql_precedences::XOR),
            ("<<", ql_precedences::BIT_MOVE),
            (">>", ql_precedences::BIT_MOVE),
            (">>>", ql_precedences::BIT_MOVE),
            (">>>=", ql_precedences::ASSIGN),
            ("&&", ql_precedences::AND),
            ("and", ql_precedences::AND),
            ("||", ql_precedences::OR),
            ("or", ql_precedences::OR),
            ("==", ql_precedences::EQUAL),
            ("!=", ql_precedences::EQUAL),
            ("<>", ql_precedences::EQUAL),
            (">", ql_precedences::COMPARE),
            (">=", ql_precedences::COMPARE),
            ("<", ql_precedences::COMPARE),
            ("<=", ql_precedences::COMPARE),
            ("instanceof", ql_precedences::COMPARE),
        ];
        for (lexeme, priority) in cases {
            let op = manager
                .get_binary_operator(lexeme)
                .unwrap_or_else(|| panic!("缺少 {lexeme}"));
            assert_eq!(op.priority(), *priority, "{lexeme} 优先级");
            assert!(manager.is_op_type(lexeme, OpType::Middle));
            assert_eq!(manager.precedence(lexeme), Some(*priority));
        }
        // 前缀一元:~ 与 !(其余由 unary/ 包提供)。
        assert!(manager.is_op_type("~", OpType::Prefix));
        assert!(manager.is_op_type("!", OpType::Prefix));
        // unary/ 包注册了单目 +/- 与 ++/--(Java 同样在册)。
        assert!(manager.is_op_type("+", OpType::Prefix));
        assert!(manager.is_op_type("++", OpType::Prefix));
        assert!(!manager.is_op_type("*", OpType::Prefix));
        // assign/collection/string 包的词素(其他 agent 交付)同样在册。
        for lexeme in ["=", "in", "not_in", "like", "not_like"] {
            assert!(
                manager.get_binary_operator(lexeme).is_some(),
                "缺少 {lexeme}"
            );
        }
        // 后缀一元:++/--。
        assert!(manager.is_op_type("++", OpType::Suffix));
        assert!(manager.is_op_type("--", OpType::Suffix));
    }

    #[test]
    fn custom_add_replace_alias_semantics() {
        let mut manager = OperatorManager::new();
        // 与内建冲突 → 拒绝。
        assert!(!manager.add_binary_operator("+", Rc::new(Pow), 100));
        // 新词素 → 接受;重复 → 拒绝。
        assert!(manager.add_binary_operator("**", Rc::new(Pow), 100));
        assert!(!manager.add_binary_operator("**", Rc::new(Pow), 100));
        assert_eq!(manager.get_binary_operator("**").unwrap().priority(), 100);
        // replace:仅内建可替换,沿用原优先级。
        assert!(manager.replace_default_operator("+", Rc::new(Pow)));
        assert_eq!(
            manager.get_binary_operator("+").unwrap().priority(),
            ql_precedences::ADD
        );
        assert!(!manager.replace_default_operator("??", Rc::new(Pow)));
        // alias:新词素委托原操作符。
        assert!(manager.add_operator_alias("plus", "-"));
        let alias = manager.get_binary_operator("plus").unwrap();
        assert_eq!(alias.operator(), "plus");
        assert_eq!(alias.priority(), ql_precedences::ADD);
        assert!(!manager.add_operator_alias("x", "missing"));
        // 关键字别名。
        assert!(manager.add_key_word_alias("ruo", "if"));
        assert_eq!(manager.get_alias("ruo"), Some(token::IF as i32));
        assert!(!manager.add_key_word_alias("x", "not-a-keyword"));
    }
}
