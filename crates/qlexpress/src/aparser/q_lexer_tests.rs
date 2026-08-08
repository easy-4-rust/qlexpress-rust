//! 从主对象文件机械搬移的聚焦单元测试；测试语义与来源标记保持不变。

use super::*;
use crate::aparser::parser_operator_manager::OpType;

/// Default scan: Script interpolation, `${`/`}` selectors, strict
/// newlines — mirrors `InitOptions.DEFAULT` usage.
fn lex(script: &str) -> Vec<Token> {
    tokenize(script, None, InterpolationMode::Script, "${", "}", true).expect("scan should succeed")
}

fn types(tokens: &[Token]) -> Vec<i32> {
    tokens.iter().map(Token::token_type).collect()
}

fn texts(tokens: &[Token]) -> Vec<&str> {
    tokens.iter().map(Token::text).collect()
}

struct AliasManager;
impl ParserOperatorManager for AliasManager {
    fn is_op_type(&self, _lexeme: &str, _op_type: OpType) -> bool {
        true
    }
    fn precedence(&self, _lexeme: &str) -> Option<i32> {
        None
    }
    fn get_alias(&self, lexeme: &str) -> Option<i32> {
        match lexeme {
            "and" | "or" | "like" => Some(token::OPID as i32),
            _ => None,
        }
    }
}

#[test]
fn empty_input_yields_only_eof() {
    let tokens = lex("");
    assert_eq!(types(&tokens), vec![token::EOF]);
    assert_eq!(tokens[0].text(), "<EOF>");
    assert_eq!(tokens[0].line(), 1);
    assert_eq!(tokens[0].char_position_in_line(), 0);
}

#[test]
fn keywords_and_identifiers() {
    let tokens = lex("for if else while break continue return function macro import static new");
    assert_eq!(
        types(&tokens),
        vec![
            token::FOR as i32,
            token::IF as i32,
            token::ELSE as i32,
            token::WHILE as i32,
            token::BREAK as i32,
            token::CONTINUE as i32,
            token::RETURN as i32,
            token::FUNCTION as i32,
            token::MACRO as i32,
            token::IMPORT as i32,
            token::STATIC as i32,
            token::NEW as i32,
            token::EOF,
        ]
    );
    let tokens = lex("switch case default byte short int long float double char boolean");
    assert_eq!(
        types(&tokens)[..11],
        [
            token::SWITCH as i32,
            token::CASE as i32,
            token::DEFAULT as i32,
            token::BYTE as i32,
            token::SHORT as i32,
            token::INT as i32,
            token::LONG as i32,
            token::FLOAT as i32,
            token::DOUBLE as i32,
            token::CHAR as i32,
            token::BOOL as i32,
        ]
    );
    let tokens = lex("null true false extends super try catch finally throw then class this");
    assert_eq!(
        types(&tokens)[..12],
        [
            token::NULL as i32,
            token::TRUE as i32,
            token::FALSE as i32,
            token::EXTENDS as i32,
            token::SUPER as i32,
            token::TRY as i32,
            token::CATCH as i32,
            token::FINALLY as i32,
            token::THROW as i32,
            token::THEN as i32,
            token::CLASS as i32,
            token::THIS as i32,
        ]
    );
    assert_eq!(tokens[12].token_type(), token::EOF);
}

#[test]
fn identifiers_with_special_start_and_unicode() {
    let tokens = lex("#a @b $c _d 变量x ef01");
    assert_eq!(
        texts(&tokens)[..6],
        ["#a", "@b", "$c", "_d", "变量x", "ef01"]
    );
    assert_eq!(types(&tokens)[..6], [token::ID as i32; 6]);
    // CJK punctuation allowed as id part.
    let tokens = lex("func（1）");
    assert_eq!(texts(&tokens)[..2], ["func（1）", "<EOF>"]);
}

/// SOURCE_PARITY: Java `QLexer#tokenize` 必须使用传入的操作符别名管理器，
/// 并在完整扫描后追加 EOF。
#[test]
fn word_operator_alias_via_manager() {
    let manager = AliasManager;
    let tokens = tokenize(
        "a and b",
        Some(&manager),
        InterpolationMode::Script,
        "${",
        "}",
        true,
    )
    .unwrap();
    assert_eq!(
        types(&tokens),
        vec![
            token::ID as i32,
            token::OPID as i32,
            token::ID as i32,
            token::EOF
        ]
    );
    assert_eq!(tokens[1].text(), "and");
}

#[test]
fn number_literals() {
    let tokens = lex("1 1L 0x1F 0X2a 0b101 0B11L 1.5 .5 1. 1e3 1E-3 2e+2f 3d 4F 1_000 0xff_ff");
    let expect = vec![
        (token::INTEGER_OR_FLOATING_LITERAL, "1"),
        (token::INTEGER_OR_FLOATING_LITERAL, "1L"),
        (token::INTEGER_LITERAL, "0x1F"),
        (token::INTEGER_LITERAL, "0X2a"),
        (token::INTEGER_LITERAL, "0b101"),
        (token::INTEGER_LITERAL, "0B11L"),
        // Like Java, `1.5`/`1.` stay IntegerOrFloatingLiteral; the
        // parser refines the kind (only `.5`, exponents, and float
        // suffixes are FloatingPointLiteral at scan time).
        (token::INTEGER_OR_FLOATING_LITERAL, "1.5"),
        (token::FLOATING_POINT_LITERAL, ".5"),
        (token::INTEGER_OR_FLOATING_LITERAL, "1."),
        (token::FLOATING_POINT_LITERAL, "1e3"),
        (token::FLOATING_POINT_LITERAL, "1E-3"),
        (token::FLOATING_POINT_LITERAL, "2e+2f"),
        (token::FLOATING_POINT_LITERAL, "3d"),
        (token::FLOATING_POINT_LITERAL, "4F"),
        (token::INTEGER_OR_FLOATING_LITERAL, "1_000"),
        (token::INTEGER_LITERAL, "0xff_ff"),
    ];
    let got: Vec<(u16, &str)> = tokens[..expect.len()]
        .iter()
        .map(|t| (t.token_type() as u16, t.text()))
        .collect();
    assert_eq!(got, expect);
}

#[test]
fn number_dot_before_method_not_consumed() {
    // `1.toString` -> `1` `.` `toString` (two ASCII letters after dot).
    let tokens = lex("1.toString");
    assert_eq!(
        types(&tokens),
        vec![
            token::INTEGER_OR_FLOATING_LITERAL as i32,
            token::DOT as i32,
            token::ID as i32,
            token::EOF,
        ]
    );
    // `1.x1` -> dot consumed (second char not a letter).
    let tokens = lex("1.x1");
    assert_eq!(texts(&tokens)[0], "1.");
    // `1.e` -> p+2 >= len, dot consumed.
    let tokens = lex("1.e");
    assert_eq!(texts(&tokens)[..2], ["1.", "e"]);
    // malformed exponent backtracks: `1e+` -> `1` `e` `+`
    let tokens = lex("1e+");
    assert_eq!(texts(&tokens)[..4], ["1", "e", "+", "<EOF>"]);
}

#[test]
fn single_quote_string() {
    let tokens = lex("'abc' 'it\\'s' ''");
    assert_eq!(texts(&tokens)[..4], ["'abc'", "'it\\'s'", "''", "<EOF>"]);
    assert_eq!(types(&tokens)[..3], [token::QUOTE_STRING_LITERAL as i32; 3]);
}

#[test]
fn double_quote_string_disable_mode() {
    let tokens = tokenize(
        "\"a ${x} b\"",
        None,
        InterpolationMode::Disable,
        "${",
        "}",
        true,
    )
    .unwrap();
    assert_eq!(texts(&tokens)[..4], ["\"", "a ${x} b", "\"", "<EOF>"]);
    assert_eq!(
        types(&tokens)[..3],
        [
            token::DOUBLE_QUOTE as i32,
            token::STATIC_STRING_CHARACTERS as i32,
            token::DOUBLE_QUOTE as i32,
        ]
    );
    // Empty string emits no text token.
    let tokens = tokenize("\"\"", None, InterpolationMode::Disable, "${", "}", true).unwrap();
    assert_eq!(
        types(&tokens),
        vec![
            token::DOUBLE_QUOTE as i32,
            token::DOUBLE_QUOTE as i32,
            token::EOF,
        ]
    );
}

#[test]
fn double_quote_string_script_interpolation() {
    let tokens = lex("\"a ${x + {1}} b\"");
    assert_eq!(
        types(&tokens),
        vec![
            token::DOUBLE_QUOTE as i32,
            token::DY_STR_TEXT as i32,
            token::DY_STR_EXPR_START as i32,
            token::ID as i32,                          // x
            token::ADD as i32,                         // +
            token::LBRACE as i32,                      // {
            token::INTEGER_OR_FLOATING_LITERAL as i32, // 1
            token::RBRACE as i32,                      // }
            token::RBRACE as i32,                      // } closing interpolation
            token::DY_STR_TEXT as i32,
            token::DOUBLE_QUOTE as i32,
            token::EOF,
        ]
    );
    assert_eq!(texts(&tokens)[1], "a ");
    assert_eq!(texts(&tokens)[9], " b");
}

#[test]
fn double_quote_string_variable_interpolation() {
    let tokens = tokenize(
        "\"hello ${name}!\"",
        None,
        InterpolationMode::Variable,
        "${",
        "}",
        true,
    )
    .unwrap();
    assert_eq!(
        types(&tokens),
        vec![
            token::DOUBLE_QUOTE as i32,
            token::DY_STR_TEXT as i32,
            token::DY_STR_EXPR_START as i32,
            token::SELECTOR_VARIABLE_VANME as i32,
            token::DY_STR_TEXT as i32,
            token::DOUBLE_QUOTE as i32,
            token::EOF,
        ]
    );
    assert_eq!(tokens[3].text(), "name");
}

#[test]
fn selector_outside_string() {
    let tokens = lex("${user.name} + 1");
    assert_eq!(
        types(&tokens)[..5],
        [
            token::SELECTOR_START as i32,
            token::SELECTOR_VARIABLE_VANME as i32,
            token::ADD as i32,
            token::INTEGER_OR_FLOATING_LITERAL as i32,
            token::EOF,
        ]
    );
    assert_eq!(tokens[1].text(), "user.name");
    // custom selector delimiters
    let tokens = tokenize("$[a]", None, InterpolationMode::Script, "$[", "]", true).unwrap();
    assert_eq!(texts(&tokens)[..3], ["$[", "a", "<EOF>"]);
}

#[test]
fn operator_longest_match() {
    let tokens =
        lex(">>>= >>> >>= <<= -> :: <> >> << >= <= ?. *. .* += -= &= |= *= %= /= ^= ++ --");
    assert_eq!(
        types(&tokens)[..24],
        [
            token::URSHIFT_ASSGIN as i32,
            token::URSHIFT as i32,
            token::RIGHSHIFT_ASSGIN as i32,
            token::LSHIFT_ASSGIN as i32,
            token::ARROW as i32,
            token::DCOLON as i32,
            token::NOEQ as i32,
            token::RIGHSHIFT as i32,
            token::LEFTSHIFT as i32,
            token::GE as i32,
            token::LE as i32,
            token::OPTIONAL_CHAINING as i32,
            token::SPREAD_CHAINING as i32,
            token::DOTMUL as i32,
            token::ADD_ASSIGN as i32,
            token::SUB_ASSIGN as i32,
            token::AND_ASSIGN as i32,
            token::OR_ASSIGN as i32,
            token::MUL_ASSIGN as i32,
            token::MOD_ASSIGN as i32,
            token::DIV_ASSIGN as i32,
            token::XOR_ASSIGN as i32,
            token::INC as i32,
            token::DEC as i32,
        ]
    );
}

#[test]
fn opid_operators_and_custom() {
    let tokens = lex("== != && || ** %% ~^");
    assert_eq!(
        texts(&tokens)[..8],
        ["==", "!=", "&&", "||", "**", "%%", "~^", "<EOF>"]
    );
    assert_eq!(types(&tokens)[..7], [token::OPID as i32; 7]);
    // `<` is not a custom-operator start, so `<=>` splits as `<=` + `>`.
    let tokens = lex("<=>");
    assert_eq!(
        types(&tokens),
        vec![token::LE as i32, token::GT as i32, token::EOF]
    );
}

#[test]
fn single_char_punctuation_and_operators() {
    let tokens = lex("( ) { } [ ] . ; , ? : > < = ^ ! ~ + - * / & | %");
    assert_eq!(
        types(&tokens)[..24],
        [
            token::LPAREN as i32,
            token::RPAREN as i32,
            token::LBRACE as i32,
            token::RBRACE as i32,
            token::LBRACK as i32,
            token::RBRACK as i32,
            token::DOT as i32,
            token::SEMI as i32,
            token::COMMA as i32,
            token::QUESTION as i32,
            token::COLON as i32,
            token::GT as i32,
            token::LT as i32,
            token::EQ as i32,
            token::CARET as i32,
            token::BANG as i32,
            token::TILDE as i32,
            token::ADD as i32,
            token::SUB as i32,
            token::MUL as i32,
            token::DIV as i32,
            token::BIT_AND as i32,
            token::BIT_OR as i32,
            token::MOD as i32,
        ]
    );
}

#[test]
fn unknown_char_is_catch_all() {
    let tokens = lex("a ` b");
    assert_eq!(
        types(&tokens),
        vec![
            token::ID as i32,
            token::CATCH_ALL as i32,
            token::ID as i32,
            token::EOF,
        ]
    );
    assert_eq!(tokens[1].text(), "`");
}

#[test]
fn comments_are_skipped() {
    let tokens = lex("1 // hello\n/* multi\nline */ 2");
    assert_eq!(texts(&tokens)[..4], ["1", "\n", "2", "<EOF>"]);
    assert_eq!(
        types(&tokens),
        vec![
            token::INTEGER_OR_FLOATING_LITERAL as i32,
            token::NEWLINE as i32,
            token::INTEGER_OR_FLOATING_LITERAL as i32,
            token::EOF,
        ]
    );
}

#[test]
fn newlines_strict_vs_relaxed() {
    let tokens = lex("a\nb\r\nc\rd");
    assert_eq!(
        types(&tokens),
        vec![
            token::ID as i32,
            token::NEWLINE as i32,
            token::ID as i32,
            token::NEWLINE as i32,
            token::ID as i32,
            token::NEWLINE as i32,
            token::ID as i32,
            token::EOF,
        ]
    );
    // \r\n is a single NEWLINE token.
    assert_eq!(tokens[3].text(), "\r\n");
    assert_eq!(tokens[4].line(), 3);
    let tokens = tokenize("a\nb", None, InterpolationMode::Script, "${", "}", false).unwrap();
    assert_eq!(
        types(&tokens),
        vec![token::ID as i32, token::ID as i32, token::EOF]
    );
}

#[test]
fn token_positions_are_tracked() {
    let tokens = lex("ab 12\ncd");
    assert_eq!(
        (
            tokens[0].line(),
            tokens[0].char_position_in_line(),
            tokens[0].start_index(),
            tokens[0].stop_index()
        ),
        (1, 0, 0, 1)
    );
    assert_eq!(
        (tokens[1].line(), tokens[1].char_position_in_line()),
        (1, 3)
    );
    assert_eq!(
        (tokens[2].line(), tokens[2].char_position_in_line()),
        (1, 5)
    );
    assert_eq!(
        (tokens[3].line(), tokens[3].char_position_in_line()),
        (2, 0)
    );
}

#[test]
fn unterminated_single_quote_string_reports_position() {
    let err = tokenize("a = 'abc", None, InterpolationMode::Script, "${", "}", true)
        .unwrap_err()
        .into_exception();
    assert!(err.is_syntax());
    assert_eq!(err.error_code(), error_codes::SYNTAX_ERROR);
    assert_eq!(err.reason(), "unterminated string literal");
    assert_eq!(err.line_no(), 1);
    assert_eq!(err.col_no(), 5);
    assert_eq!(err.err_lexeme(), "'abc");
}

#[test]
fn unterminated_double_quote_string() {
    let err = tokenize("\"abc", None, InterpolationMode::Script, "${", "}", true)
        .unwrap_err()
        .into_exception();
    assert_eq!(err.reason(), "unterminated string literal");
    assert_eq!(err.line_no(), 1);
    assert_eq!(err.col_no(), 5);
}

#[test]
fn unterminated_interpolation_reports_mismatched_eof() {
    let err = tokenize(
        "\"a ${x + 1",
        None,
        InterpolationMode::Script,
        "${",
        "}",
        true,
    )
    .unwrap_err()
    .into_exception();
    assert_eq!(err.reason(), "mismatched input '<EOF>' expecting '}'");
}

#[test]
fn unterminated_block_comment_reports_start_line() {
    let err = tokenize(
        "1\n/* oops\n2",
        None,
        InterpolationMode::Script,
        "${",
        "}",
        true,
    )
    .unwrap_err()
    .into_exception();
    assert_eq!(err.reason(), "unterminated comment");
    assert_eq!(err.line_no(), 2);
    assert_eq!(err.col_no(), 1);
    assert_eq!(err.err_lexeme(), "/*");
}

#[test]
fn unterminated_selector() {
    let err = tokenize("${abc\n", None, InterpolationMode::Script, "${", "}", true)
        .unwrap_err()
        .into_exception();
    assert_eq!(err.reason(), "unterminated selector");
    assert_eq!(err.line_no(), 1);
    let err = tokenize("${abc", None, InterpolationMode::Script, "${", "}", true)
        .unwrap_err()
        .into_exception();
    assert_eq!(err.reason(), "unterminated selector");
}

#[test]
fn variable_mode_unterminated_selector() {
    let err = tokenize(
        "\"a ${x\n\"",
        None,
        InterpolationMode::Variable,
        "${",
        "}",
        true,
    )
    .unwrap_err()
    .into_exception();
    assert_eq!(err.reason(), "unterminated selector");
}

#[test]
fn crlf_counts_as_one_line_advance() {
    // Mirrors Java advance(): \r\n inside strings is a single newline.
    let tokens = lex("'a\r\nb' c");
    assert_eq!(tokens[0].text(), "'a\r\nb'");
    assert_eq!(tokens[1].line(), 2);
}

/// SOURCE_PARITY: Java `String` 与 LSP 均使用 UTF-16 code-unit
/// 位置；非 BMP 字符占两个 offset/column。
#[test]
fn token_positions_use_java_utf16_units() {
    let tokens = lex("\"😀\" x");
    assert_eq!(tokens[0].text(), "\"");
    assert_eq!((tokens[1].start_index(), tokens[1].stop_index()), (1, 2));
    assert_eq!(
        (tokens[2].start_index(), tokens[2].char_position_in_line()),
        (3, 3)
    );
    assert_eq!(
        (tokens[3].start_index(), tokens[3].char_position_in_line()),
        (5, 5)
    );
}

/// SOURCE_PARITY: Java `Character.isLetter(char)` 不会把 surrogate pair
/// 组合成补充平面字母；两个 UTF-16 单元分别成为 CATCH_ALL。
#[test]
fn supplementary_letter_is_not_a_java_identifier() {
    let tokens = lex("𐐀");
    assert_eq!(
        types(&tokens),
        vec![token::CATCH_ALL as i32, token::CATCH_ALL as i32, token::EOF]
    );
    assert_eq!((tokens[0].start_index(), tokens[0].stop_index()), (0, 0));
    assert_eq!((tokens[1].start_index(), tokens[1].stop_index()), (1, 1));
    assert_eq!(tokens[2].start_index(), 2);
}

/// SOURCE_PARITY: Java `Character.isDigit` 与 `Character.digit` 接受
/// BMP Unicode 十进制数字及全角十六进制字母。
#[test]
fn unicode_java_digits_are_scanned_as_numbers() {
    let tokens = lex("١٢ 0xＦＦ");
    assert_eq!(
        tokens[0].token_type(),
        token::INTEGER_OR_FLOATING_LITERAL as i32
    );
    assert_eq!(tokens[0].text(), "١٢");
    assert_eq!(tokens[1].token_type(), token::INTEGER_LITERAL as i32);
    assert_eq!(tokens[1].text(), "0xＦＦ");
}
