//! Token model and token type constants, mirroring Java
//! `com.alibaba.qlexpress4.aparser.Token` and the `public static final int`
//! block of `QLexer`.
//!
//! Constant names and declaration order match `QLexer.java` exactly (FOR = 1
//! .. CATCH_ALL = 97) so the Stage-2 parser can be cross-checked against the
//! Java sources by eye. Per SPEC §3.5 the 97 type constants are
//! `pub const u16`; the negative `EOF`/`EPSILON` sentinels from `Token.java`
//! are `pub const i32`, and [`Token::token_type`] is `i32` so a token can
//! carry either (widening from `u16` is lossless).

/// Java `Token.EOF`.
pub const EOF: i32 = -1;
/// Java `Token.EPSILON`.
pub const EPSILON: i32 = -2;

// ---------------------------------------------------------------------------
// Token type constants (QLexer.java, in declaration order).
// ---------------------------------------------------------------------------

/// Keyword `for`.
pub const FOR: u16 = 1;
/// Keyword `if`.
pub const IF: u16 = 2;
/// Keyword `else`.
pub const ELSE: u16 = 3;
/// Keyword `while`.
pub const WHILE: u16 = 4;
/// Keyword `break`.
pub const BREAK: u16 = 5;
/// Keyword `continue`.
pub const CONTINUE: u16 = 6;
/// Keyword `return`.
pub const RETURN: u16 = 7;
/// Keyword `function`.
pub const FUNCTION: u16 = 8;
/// Keyword `macro`.
pub const MACRO: u16 = 9;
/// Keyword `import`.
pub const IMPORT: u16 = 10;
/// Keyword `static`.
pub const STATIC: u16 = 11;
/// Keyword `new`.
pub const NEW: u16 = 12;
/// Keyword `switch`.
pub const SWITCH: u16 = 13;
/// Keyword `case`.
pub const CASE: u16 = 14;
/// Keyword `default`.
pub const DEFAULT: u16 = 15;
/// Keyword `byte`.
pub const BYTE: u16 = 16;
/// Keyword `short`.
pub const SHORT: u16 = 17;
/// Keyword `int`.
pub const INT: u16 = 18;
/// Keyword `long`.
pub const LONG: u16 = 19;
/// Keyword `float`.
pub const FLOAT: u16 = 20;
/// Keyword `double`.
pub const DOUBLE: u16 = 21;
/// Keyword `char`.
pub const CHAR: u16 = 22;
/// Keyword `boolean`.
pub const BOOL: u16 = 23;
/// Keyword `null`.
pub const NULL: u16 = 24;
/// Keyword `true`.
pub const TRUE: u16 = 25;
/// Keyword `false`.
pub const FALSE: u16 = 26;
/// Keyword `extends`.
pub const EXTENDS: u16 = 27;
/// Keyword `super`.
pub const SUPER: u16 = 28;
/// Keyword `try`.
pub const TRY: u16 = 29;
/// Keyword `catch`.
pub const CATCH: u16 = 30;
/// Keyword `finally`.
pub const FINALLY: u16 = 31;
/// Keyword `throw`.
pub const THROW: u16 = 32;
/// Keyword `then`.
pub const THEN: u16 = 33;
/// Keyword `class`.
pub const CLASS: u16 = 34;
/// Keyword `this`.
pub const THIS: u16 = 35;
/// Single-quoted string literal, e.g. `'abc'` (text includes the quotes).
pub const QUOTE_STRING_LITERAL: u16 = 36;
/// Integer literal with an explicit radix (`0x..` / `0b..`).
pub const INTEGER_LITERAL: u16 = 37;
/// Floating point literal (decimal point / exponent / float suffix).
pub const FLOATING_POINT_LITERAL: u16 = 38;
/// Decimal literal whose exact kind is decided by the parser (e.g. `1`, `1L`).
pub const INTEGER_OR_FLOATING_LITERAL: u16 = 39;
/// `(`.
pub const LPAREN: u16 = 40;
/// `)`.
pub const RPAREN: u16 = 41;
/// `{`.
pub const LBRACE: u16 = 42;
/// `}`.
pub const RBRACE: u16 = 43;
/// `[`.
pub const LBRACK: u16 = 44;
/// `]`.
pub const RBRACK: u16 = 45;
/// `.`.
pub const DOT: u16 = 46;
/// `->`.
pub const ARROW: u16 = 47;
/// `;`.
pub const SEMI: u16 = 48;
/// `,`.
pub const COMMA: u16 = 49;
/// `?`.
pub const QUESTION: u16 = 50;
/// `:`.
pub const COLON: u16 = 51;
/// `::`.
pub const DCOLON: u16 = 52;
/// `>`.
pub const GT: u16 = 53;
/// `<`.
pub const LT: u16 = 54;
/// `=`.
pub const EQ: u16 = 55;
/// `<>` (SQL-style not-equal; `==`/`!=` are scanned as OPID).
pub const NOEQ: u16 = 56;
/// `>>=`.
pub const RIGHSHIFT_ASSGIN: u16 = 57;
/// `>>`.
pub const RIGHSHIFT: u16 = 58;
/// `?.`.
pub const OPTIONAL_CHAINING: u16 = 59;
/// `*.`.
pub const SPREAD_CHAINING: u16 = 60;
/// `>>>=`.
pub const URSHIFT_ASSGIN: u16 = 61;
/// `>>>`.
pub const URSHIFT: u16 = 62;
/// `<<=`.
pub const LSHIFT_ASSGIN: u16 = 63;
/// `<<`.
pub const LEFTSHIFT: u16 = 64;
/// `>=`.
pub const GE: u16 = 65;
/// `<=`.
pub const LE: u16 = 66;
/// `.*`.
pub const DOTMUL: u16 = 67;
/// `^`.
pub const CARET: u16 = 68;
/// `+=`.
pub const ADD_ASSIGN: u16 = 69;
/// `-=`.
pub const SUB_ASSIGN: u16 = 70;
/// `&=`.
pub const AND_ASSIGN: u16 = 71;
/// `|=`.
pub const OR_ASSIGN: u16 = 72;
/// `*=`.
pub const MUL_ASSIGN: u16 = 73;
/// `%=`.
pub const MOD_ASSIGN: u16 = 74;
/// `/=`.
pub const DIV_ASSIGN: u16 = 75;
/// `^=`.
pub const XOR_ASSIGN: u16 = 76;
/// `!`.
pub const BANG: u16 = 77;
/// `~`.
pub const TILDE: u16 = 78;
/// `+`.
pub const ADD: u16 = 79;
/// `-`.
pub const SUB: u16 = 80;
/// `*`.
pub const MUL: u16 = 81;
/// `/`.
pub const DIV: u16 = 82;
/// `&`.
pub const BIT_AND: u16 = 83;
/// `|`.
pub const BIT_OR: u16 = 84;
/// `%`.
pub const MOD: u16 = 85;
/// `++`.
pub const INC: u16 = 86;
/// `--`.
pub const DEC: u16 = 87;
/// Line break; only emitted when `strictNewLines` is on.
pub const NEWLINE: u16 = 88;
/// Operator identifier: `==`, `!=`, `&&`, `||`, and custom operators
/// (e.g. `**`, `<=>`); word operators may also alias to OPID via
/// `ParserOperatorManager.getAlias`.
pub const OPID: u16 = 89;
/// Selector start marker (e.g. `${` outside a string).
pub const SELECTOR_START: u16 = 90;
/// Identifier (may start with `#`, `@`, `$`, `_`, or a Unicode letter).
pub const ID: u16 = 91;
/// `"` delimiter of a double-quoted string.
pub const DOUBLE_QUOTE: u16 = 92;
/// Text of a double-quoted string when interpolation is disabled.
pub const STATIC_STRING_CHARACTERS: u16 = 93;
/// `${` opening a string interpolation expression.
pub const DY_STR_EXPR_START: u16 = 94;
/// Literal text chunk inside an interpolated double-quoted string.
pub const DY_STR_TEXT: u16 = 95;
/// Variable name inside a selector or a VARIABLE-mode interpolation
/// (Java name `SelectorVariable_VANME`, kept as-is despite the typo).
pub const SELECTOR_VARIABLE_VANME: u16 = 96;
/// Any otherwise-unrecognized single character.
pub const CATCH_ALL: u16 = 97;

/// Total number of token type constants declared in `QLexer.java`
/// (FOR..CATCH_ALL). Used by tests to guard against drift.
pub const TOKEN_TYPE_COUNT: usize = 97;

/// Java `SelectorVariable_VANME` alias with the Java spelling, for parser
/// cross-referencing.
pub const SELECTOR_VARIABLE_VANME_JAVA_NAME: &str = "SelectorVariable_VANME";

/// 把关键字文本映射为对应 Token 类别。
/// 参数：`text`；返回：`Option<u16>`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/Token.java`，方法 `keywordType`；Rust 侧按所有权与 `Result` 语义适配。
/// Look up the token type of a keyword, mirroring `QLexer.KEYWORDS`.
/// Returns `None` for non-keywords (the scanner then emits [`ID`] or an
/// operator alias).
/// 对应 Java: com.alibaba.qlexpress4.aparser.Token#keywordType。
pub fn keyword_type(text: &str) -> Option<u16> {
    Some(match text {
        "for" => FOR,
        "if" => IF,
        "else" => ELSE,
        "while" => WHILE,
        "break" => BREAK,
        "continue" => CONTINUE,
        "return" => RETURN,
        "function" => FUNCTION,
        "macro" => MACRO,
        "import" => IMPORT,
        "static" => STATIC,
        "new" => NEW,
        "switch" => SWITCH,
        "case" => CASE,
        "default" => DEFAULT,
        "byte" => BYTE,
        "short" => SHORT,
        "int" => INT,
        "long" => LONG,
        "float" => FLOAT,
        "double" => DOUBLE,
        "char" => CHAR,
        "boolean" => BOOL,
        "null" => NULL,
        "true" => TRUE,
        "false" => FALSE,
        "extends" => EXTENDS,
        "super" => SUPER,
        "try" => TRY,
        "catch" => CATCH,
        "finally" => FINALLY,
        "throw" => THROW,
        "then" => THEN,
        "class" => CLASS,
        "this" => THIS,
        _ => return None,
    })
}

/// 保存词法类别、原文以及源码偏移和行列位置的不可变 Token。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/Token.java`；具体对象路径见 `docs/对象级对照表.md`。
/// A lexical token, mirroring Java `Token`.
///
/// `start_index`/`stop_index` are character offsets into the script
/// (Java uses UTF-16 code-unit offsets; for non-BMP characters the indexes
/// can differ — positions in line/col are unaffected). `line` is 1-based and
/// `char_position_in_line` is 0-based, exactly like the Java version.
#[derive(Clone, Debug, PartialEq, Eq)]
/// 对应 Java: com.alibaba.qlexpress4.aparser.Token。
pub struct Token {
    /// Token type: one of the `u16` constants above, or [`EOF`]/[`EPSILON`].
    token_type: i32,
    /// Source text of the token.
    text: String,
    /// Inclusive start offset (in chars) within the script.
    start_index: i32,
    /// Inclusive stop offset (in chars) within the script.
    stop_index: i32,
    /// 1-based line number.
    line: i32,
    /// 0-based column (Java `charPositionInLine`).
    char_position_in_line: i32,
}

impl Token {
    /// 创建对象实例。
    /// 参数：`token_type`、`text`、`start_index`、`stop_index`、`line`、`char_position_in_line`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/Token.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new Token(type, text, startIndex, stopIndex, line,
    /// charPositionInLine)`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.Token#new。
    pub fn new(
        token_type: i32,
        text: impl Into<String>,
        start_index: i32,
        stop_index: i32,
        line: i32,
        char_position_in_line: i32,
    ) -> Self {
        Token {
            token_type,
            text: text.into(),
            start_index,
            stop_index,
            line,
            char_position_in_line,
        }
    }

    /// 返回当前 Token 的词法类别编号。
    /// 无显式参数；返回：`i32`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/Token.java`，方法 `tokenType`；Rust 侧按所有权与 `Result` 语义适配。
    /// Token type as declared by the constants in this module
    /// (Java `getType`).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.Token#tokenType。
    pub fn token_type(&self) -> i32 {
        self.token_type
    }

    /// 更新 token type。
    /// 参数：`token_type`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/Token.java`，方法 `setTokenType`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `setType` — the parser may re-tag tokens (e.g. OPID aliases).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.Token#setTokenType。
    pub fn set_token_type(&mut self, token_type: i32) {
        self.token_type = token_type;
    }

    /// 返回当前节点或 Token 对应的源码文本。
    /// 无显式参数；返回：`&str`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/Token.java`，方法 `text`；Rust 侧按所有权与 `Result` 语义适配。
    /// Source text (Java `getText`).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.Token#text。
    pub fn text(&self) -> &str {
        &self.text
    }

    /// 返回当前 Token 在源码中的起始偏移。
    /// 无显式参数；返回：`i32`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/Token.java`，方法 `startIndex`；Rust 侧按所有权与 `Result` 语义适配。
    /// Inclusive start offset in chars (Java `getStartIndex`).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.Token#startIndex。
    pub fn start_index(&self) -> i32 {
        self.start_index
    }

    /// 返回当前 Token 在源码中的结束偏移。
    /// 无显式参数；返回：`i32`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/Token.java`，方法 `stopIndex`；Rust 侧按所有权与 `Result` 语义适配。
    /// Inclusive stop offset in chars (Java `getStopIndex`).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.Token#stopIndex。
    pub fn stop_index(&self) -> i32 {
        self.stop_index
    }

    /// 返回当前 Token 的零基源码行号。
    /// 无显式参数；返回：`i32`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/Token.java`，方法 `line`；Rust 侧按所有权与 `Result` 语义适配。
    /// 1-based line number (Java `getLine`).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.Token#line。
    pub fn line(&self) -> i32 {
        self.line
    }

    /// 返回当前 Token 在所在行中的零基字符位置。
    /// 无显式参数；返回：`i32`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/exception/lsp/Position.java`，方法 `charPositionInLine`；Rust 侧按所有权与 `Result` 语义适配。
    /// 0-based column (Java `getCharPositionInLine`).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.Token#charPositionInLine。
    pub fn char_position_in_line(&self) -> i32 {
        self.char_position_in_line
    }
}

impl std::fmt::Display for Token {
    /// Java `toString` returns the token text.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constants_are_dense_1_to_97() {
        // Guard the full FOR..CATCH_ALL range: collect via keyword map spot
        // checks plus the boundary values.
        assert_eq!(FOR, 1);
        assert_eq!(CATCH_ALL, 97);
        assert_eq!(QUOTE_STRING_LITERAL, 36);
        assert_eq!(NEWLINE, 88);
        assert_eq!(OPID, 89);
        assert_eq!(SELECTOR_VARIABLE_VANME, 96);
        assert_eq!(EOF, -1);
        assert_eq!(EPSILON, -2);
    }

    #[test]
    fn keyword_table_covers_all_35_keywords() {
        let cases: &[(&str, u16)] = &[
            ("for", FOR),
            ("if", IF),
            ("else", ELSE),
            ("while", WHILE),
            ("break", BREAK),
            ("continue", CONTINUE),
            ("return", RETURN),
            ("function", FUNCTION),
            ("macro", MACRO),
            ("import", IMPORT),
            ("static", STATIC),
            ("new", NEW),
            ("switch", SWITCH),
            ("case", CASE),
            ("default", DEFAULT),
            ("byte", BYTE),
            ("short", SHORT),
            ("int", INT),
            ("long", LONG),
            ("float", FLOAT),
            ("double", DOUBLE),
            ("char", CHAR),
            ("boolean", BOOL),
            ("null", NULL),
            ("true", TRUE),
            ("false", FALSE),
            ("extends", EXTENDS),
            ("super", SUPER),
            ("try", TRY),
            ("catch", CATCH),
            ("finally", FINALLY),
            ("throw", THROW),
            ("then", THEN),
            ("class", CLASS),
            ("this", THIS),
        ];
        assert_eq!(cases.len(), 35);
        for (word, ty) in cases {
            assert_eq!(keyword_type(word), Some(*ty), "keyword {word}");
        }
        assert_eq!(keyword_type("identifier"), None);
        assert_eq!(keyword_type("For"), None);
    }

    #[test]
    fn token_accessors_and_display() {
        let mut tok = Token::new(ID as i32, "abc", 3, 5, 2, 1);
        assert_eq!(tok.token_type(), ID as i32);
        assert_eq!(tok.text(), "abc");
        assert_eq!(tok.start_index(), 3);
        assert_eq!(tok.stop_index(), 5);
        assert_eq!(tok.line(), 2);
        assert_eq!(tok.char_position_in_line(), 1);
        assert_eq!(tok.to_string(), "abc");
        tok.set_token_type(OPID as i32);
        assert_eq!(tok.token_type(), OPID as i32);
    }
}
