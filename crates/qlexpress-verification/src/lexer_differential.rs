//! Java/Rust QLexer 逐 Token 差分执行器。

use qlexpress::aparser::interpolation_mode::InterpolationMode;
use qlexpress::aparser::op_type::OpType;
use qlexpress::aparser::parser_operator_manager::ParserOperatorManager;
use qlexpress::aparser::q_lexer::tokenize;
use qlexpress::aparser::token::{self, Token};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

#[derive(Deserialize)]
struct LexerCase {
    id: String,
    #[serde(default)]
    script: String,
    #[serde(default)]
    interpolation_mode: Mode,
    #[serde(default = "default_selector_start")]
    selector_start: String,
    #[serde(default = "default_selector_end")]
    selector_end: String,
    #[serde(default = "default_true")]
    strict_new_lines: bool,
    #[serde(default)]
    aliases: HashSet<String>,
}

#[derive(Clone, Copy, Default, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum Mode {
    #[default]
    Script,
    Variable,
    Disable,
}

impl From<Mode> for InterpolationMode {
    fn from(value: Mode) -> Self {
        match value {
            Mode::Script => InterpolationMode::Script,
            Mode::Variable => InterpolationMode::Variable,
            Mode::Disable => InterpolationMode::Disable,
        }
    }
}

struct AliasManager {
    aliases: HashSet<String>,
}

impl ParserOperatorManager for AliasManager {
    fn is_op_type(&self, _lexeme: &str, _op_type: OpType) -> bool {
        false
    }

    fn precedence(&self, _lexeme: &str) -> Option<i32> {
        None
    }

    fn get_alias(&self, lexeme: &str) -> Option<i32> {
        self.aliases.contains(lexeme).then_some(token::OPID as i32)
    }
}

#[derive(Serialize)]
struct TokenRecord<'a> {
    #[serde(rename = "type")]
    token_type: i32,
    text: &'a str,
    start: i32,
    stop: i32,
    line: i32,
    column: i32,
}

impl<'a> From<&'a Token> for TokenRecord<'a> {
    fn from(token: &'a Token) -> Self {
        Self {
            token_type: token.token_type(),
            text: token.text(),
            start: token.start_index(),
            stop: token.stop_index(),
            line: token.line(),
            column: token.char_position_in_line(),
        }
    }
}

#[derive(Serialize)]
struct LexerRecord<'a> {
    id: &'a str,
    outcome: &'static str,
    tokens: Vec<TokenRecord<'a>>,
    error_code: Option<&'a str>,
    line: Option<i32>,
    column: Option<i32>,
    reason: Option<&'a str>,
}

pub(crate) fn run(corpus: &Path, output: &Path) -> Result<(), String> {
    let source = File::open(corpus)
        .map_err(|error| format!("open lexer corpus {}: {error}", corpus.display()))?;
    let target = File::create(output)
        .map_err(|error| format!("create lexer output {}: {error}", output.display()))?;
    let mut writer = BufWriter::new(target);
    let mut completed = 0_usize;

    for (index, line) in BufReader::new(source).lines().enumerate() {
        let line = line.map_err(|error| format!("read lexer case {}: {error}", index + 1))?;
        if line.trim().is_empty() || line.trim_start().starts_with('#') {
            continue;
        }
        let case: LexerCase = serde_json::from_str(&line)
            .map_err(|error| format!("parse lexer case {}: {error}", index + 1))?;
        let manager = AliasManager {
            aliases: case.aliases,
        };
        let operator_manager: Option<&dyn ParserOperatorManager> = if manager.aliases.is_empty() {
            None
        } else {
            Some(&manager)
        };
        let result = tokenize(
            &case.script,
            operator_manager,
            case.interpolation_mode.into(),
            &case.selector_start,
            &case.selector_end,
            case.strict_new_lines,
        );
        match result {
            Ok(tokens) => {
                let record = LexerRecord {
                    id: &case.id,
                    outcome: "ok",
                    tokens: tokens.iter().map(TokenRecord::from).collect(),
                    error_code: None,
                    line: None,
                    column: None,
                    reason: None,
                };
                serde_json::to_writer(&mut writer, &record)
                    .map_err(|error| format!("serialize lexer case {}: {error}", case.id))?;
            }
            Err(error) => {
                let record = LexerRecord {
                    id: &case.id,
                    outcome: "error",
                    tokens: Vec::new(),
                    error_code: Some(error.error_code()),
                    line: Some(error.line_no()),
                    column: Some(error.col_no()),
                    reason: Some(error.reason()),
                };
                serde_json::to_writer(&mut writer, &record).map_err(|serialize| {
                    format!("serialize lexer case {}: {serialize}", case.id)
                })?;
            }
        }
        writer
            .write_all(b"\n")
            .map_err(|error| format!("write lexer case {}: {error}", case.id))?;
        completed += 1;
    }
    writer
        .flush()
        .map_err(|error| format!("flush lexer output {}: {error}", output.display()))?;
    eprintln!("rust lexer differential cases completed: {completed}");
    Ok(())
}

fn default_selector_start() -> String {
    "${".to_string()
}

fn default_selector_end() -> String {
    "}".to_string()
}

fn default_true() -> bool {
    true
}
