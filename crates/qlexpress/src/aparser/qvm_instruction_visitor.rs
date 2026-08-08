//! Compilation of the syntax tree into a QVM instruction sequence,
//! mirroring Java `QvmInstructionVisitor` method by method.
//!
//! Java signals syntax errors by throwing `QLSyntaxException`
//! mid-traversal; the Rust [`Visitor`] returns `()`, so the first error is
//! recorded in [`QvmInstructionVisitor::syntax_error`] and emission stops
//! (every overridden visit short-circuits), mirroring the unwind.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use num_bigint::BigInt;

use super::compile_time_function::{CodeGenerator, CompileTimeFunction};
use super::generator_scope::GeneratorScope;
use super::import_manager::ImportManager;
use super::interpolation_mode::InterpolationMode;
use super::macro_define::MacroDefine;
use super::operator_factory::OperatorFactory;
use super::qlexer::java_digit_value;
use super::qlparser_base_visitor::Visitor;
use super::syntax_tree_factory::{
    BlockExprContext, BlockStatementsContext, BreakContinueStatementContext, CastExprContext,
    ChainKind, ContextSelectExprContext, CustomPathContext, DoubleQuoteStringLiteralContext,
    DyStrPart, ExpressionContext, FieldAccessContext, ForEachStatementContext,
    FunctionStatementContext, ImportClsContext, ImportPackContext, IndexExprContext,
    LambdaExprContext, LeftAssoContext, LeftHandSideContext, ListExprContext, LiteralContext,
    LocalVariableDeclarationContext, MacroStatementContext, MapExprContext, MethodAccessContext,
    MethodInvokeContext, NewEmptyArrExprContext, NewInitArrExprContext, NewObjExprContext, Node,
    PrimaryContext, QlIfContext, ReturnStatementContext, SwitchExprContext, SwitchExprGroupContext,
    SwitchStatementGroupContext, TernaryExprContext, ThrowStatementContext,
    TraditionalForStatementContext, TryCatchExprContext, TypeExprContext, VarIdExprContext,
    WhileStatementContext,
};
use super::token::{self, Token};
use crate::exception::default_err_reporter::DefaultErrReporter;
use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::pure_err_reporter::PureErrReporter;
use crate::exception::ql_syntax_exception::QLSyntaxException;
use crate::exception::QLException;
use crate::init_options::InitOptions;
use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::function::CustomFunction;
use crate::runtime::instruction::ReturnResultType;
use crate::runtime::instruction::{
    BreakContinueInstruction, CallFunctionInstruction, CastInstruction, CheckTimeOutInstruction,
    CloseScopeInstruction, ConstInstruction, DefineFunctionInstruction, DefineLocalInstruction,
    ForEachInstruction, ForInstruction, GetFieldInstruction, GetMethodInstruction,
    IndexInstruction, Instruction, JumpIfInstruction, JumpIfPopInstruction, JumpInstruction,
    LoadInstruction, LoadLambdaInstruction, MethodInvokeInstruction, MultiNewArrayInstruction,
    NewArrayInstruction, NewFilledInstanceInstruction, NewInstanceInstruction, NewListInstruction,
    NewMapInstruction, NewScopeInstruction, OperatorInstruction, PopInstruction, QLInstruction,
    ReturnInstruction, SliceInstruction, SliceMode, SpreadGetFieldInstruction,
    SpreadMethodInvokeInstruction, StringJoinInstruction, ThrowInstruction,
    TraceEvaluatedInstruction, TracePeekInstruction, TryCatchInstruction, UnaryInstruction,
    WhileInstruction,
};
use crate::runtime::member::{ClassRef, MetaClass};
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qlambda_definition_empty::QLambdaDefinitionEmpty;
use crate::runtime::qlambda_definition_inner::{Param, QLambdaDefinitionInner};
use crate::runtime::value::DataValue;
use crate::utils::ql_string_utils::QLStringUtils;

/// Java `SCOPE_SEPARATOR`.
const SCOPE_SEPARATOR: &str = "$";
const BLOCK_LAMBDA_NAME_PREFIX: &str = "BLOCK_";
const IF_PREFIX: &str = "IF_";
const THEN_SUFFIX: &str = "_THEN";
const ELSE_SUFFIX: &str = "_ELSE";
const MACRO_PREFIX: &str = "MACRO_";
const LAMBDA_PREFIX: &str = "LAMBDA_";
const LAZY_FUNCTION_PREFIX: &str = "LAZY_FUNCTION_";
const TRY_PREFIX: &str = "TRY_";
const CATCH_SUFFIX: &str = "_CATCH";
const FINAL_SUFFIX: &str = "_FINAL";
const FOR_PREFIX: &str = "FOR_";
const INIT_SUFFIX: &str = "_INIT";
const CONDITION_SUFFIX: &str = "_CONDITION";
const UPDATE_SUFFIX: &str = "_UPDATE";
const BODY_SUFFIX: &str = "_BODY";
const WHILE_PREFIX: &str = "WHILE_";

/// Java `BigDecimal(String.valueOf(Double.MAX_VALUE))`.
const MAX_DOUBLE_TEXT: &str = "1.7976931348623157E308";

/// Java `TIMEOUT_CHECK_GAP`.
const TIMEOUT_CHECK_GAP: i32 = 5;

pub use super::instruction_context::Context;

/// `SharedInstruction` 类型别名的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Instruction representation shared through scopes/macros (Java shares
/// the `QLInstruction` objects; the port shares them by `Rc`).
pub type SharedInstruction = Rc<dyn QLInstruction>;

/// `InstructionScope` 类型别名的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/GeneratorScope.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `GeneratorScope` instantiated with the shared instruction type.
pub type InstructionScope = GeneratorScope<SharedInstruction>;

/// `InstructionMacroDefine` 类型别名的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/MacroDefine.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `MacroDefine` instantiated with the shared instruction type.
pub type InstructionMacroDefine = MacroDefine<SharedInstruction>;

/// `CompileTimeFunctions` 类型别名的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/compiletimefunction/CompileTimeFunction.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Compile-time function registry (Java `Map<String, CompileTimeFunction>`).
pub type CompileTimeFunctions = HashMap<String, Rc<dyn CompileTimeFunction>>;

/// `UserDefineFunctions` 类型别名的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/function/CustomFunction.java`；具体对象路径见 `docs/对象级对照表.md`。
/// User-defined function registry (Java `Map<String, CustomFunction>`).
pub type UserDefineFunctions = HashMap<String, Rc<dyn CustomFunction>>;

/// 将语法树编译为 QVM 指令序列并计算最大操作数栈深的访问器。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `QvmInstructionVisitor`.
/// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor。
pub struct QvmInstructionVisitor<'a> {
    script: &'a str,
    /// `RefCell` because `visitImportCls`/`visitImportPack` mutate it
    /// (Java `importManager.addImport`) while other visits only read.
    import_manager: &'a RefCell<ImportManager<'a>>,
    generator_scope: Rc<InstructionScope>,
    operator_factory: &'a dyn OperatorFactory,
    compile_time_functions: &'a CompileTimeFunctions,
    user_define_functions: &'a UserDefineFunctions,
    init_options: &'a InitOptions,
    context: Context,
    instruction_list: Vec<Instruction>,
    stack_size: i32,
    max_stack_size: i32,
    if_counter: i32,
    switch_counter: i32,
    block_counter: i32,
    macro_counter: i32,
    lambda_counter: i32,
    lazy_function_counter: i32,
    try_counter: i32,
    for_counter: i32,
    while_counter: i32,
    timeout_check_point: i32,
    /// First recorded syntax error (Java: the thrown exception).
    syntax_error: Option<QLSyntaxException>,
    /// True while the previously pushed instruction is a
    /// `CheckTimeOutInstruction` (Java `lastInstruction instanceof
    /// CheckTimeOutInstruction` in `addTimeoutInstruction`).
    last_is_timeout_check: bool,
    /// When compiling a switch-statement body: indices (in this visitor's
    /// own `instruction_list`) of top-level `BreakContinueInstruction`s
    /// with `break` semantics. Java finds them with `instanceof` over the
    /// compiled body; the port records them at emission time.
    collect_break_indices: Option<Vec<usize>>,
}

include!("qvm_instruction_visitor/core.rs");
include!("qvm_instruction_visitor/compilation_helpers.rs");
include!("qvm_instruction_visitor/visit_statements.rs");
include!("qvm_instruction_visitor/visit_expressions.rs");
include!("qvm_instruction_visitor/visit_paths.rs");

impl<'a> Visitor for QvmInstructionVisitor<'a> {
    qvm_visit_statement_methods!();
    qvm_visit_expression_methods!();
    qvm_visit_path_methods!();
}

include!("qvm_instruction_visitor/expression_helpers.rs");

/// Java `ArgumentListContext.expression().size()`.
fn argument_count(argument_list: &Node) -> usize {
    match argument_list {
        Node::ArgumentList(ctx) => ctx.expressions.len(),
        _ => 0,
    }
}

/// Java `ArgumentListContext.expression()`.
fn argument_expressions(argument_list: &Node) -> Vec<&Node> {
    match argument_list {
        Node::ArgumentList(ctx) => ctx.expressions.iter().collect(),
        _ => vec![],
    }
}

/// Java `isEmptyIndex`.
fn is_empty_index(path_part: &Node) -> bool {
    match path_part {
        Node::IndexExpr(ctx) => ctx.index_value_expr.is_none(),
        _ => false,
    }
}

/// Iterate the `SwitchCaseGroup` children of a `SwitchExpr`.
fn switch_groups(ctx: &SwitchExprContext) -> impl Iterator<Item = &Node> {
    let groups: &[Node] = match ctx.groups.as_deref() {
        Some(Node::SwitchCaseGroups(groups_ctx)) => &groups_ctx.groups,
        _ => &[],
    };
    groups.iter()
}

/// Whether a switch `default` body ends in `return`/`break`/`continue`
/// (Java checks the last compiled instruction with `instanceof`; the port
/// checks the last statement syntactically — equivalent, since a
/// `return`/`break`/`continue` statement always compiles to a trailing
/// `ReturnInstruction`/`BreakContinueInstruction`).
fn last_stmt_is_return_or_break(block_statements: Option<&Node>) -> bool {
    let Some(Node::BlockStatements(ctx)) = block_statements else {
        return false;
    };
    ctx.statements
        .iter()
        .rfind(|bs| !matches!(bs, Node::EmptyStatement(_)))
        .is_some_and(|last| {
            matches!(
                last,
                Node::ReturnStatement(_) | Node::BreakContinueStatement(_)
            )
        })
}

/// Java `String.substring(1, length - 1)` for a quoted literal.
fn strip_quotes(text: &str) -> String {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() >= 2 {
        chars[1..chars.len() - 1].iter().collect()
    } else {
        String::new()
    }
}

/// Java `remove(String, char)`.
fn remove_char(target: &str, c: char) -> String {
    target.chars().filter(|&x| x != c).collect()
}

/// Java `parseInteger` (auto int/long/BigInteger by magnitude).
fn parse_integer_literal(int_text: &str) -> Option<DataValue> {
    let (base_text, force_long) = match int_text.chars().last() {
        Some('l') | Some('L') => (&int_text[..int_text.len() - 1], true),
        _ => (int_text, false),
    };
    let base_int = parse_base_integer(base_text)?;
    if force_long {
        // Java BigInteger.longValue() wraps on overflow.
        return Some(DataValue::Long(crate::runtime::data::convert::to_i64(
            &DataValue::BigInt(base_int),
        )));
    }
    if base_int <= BigInt::from(i32::MAX) {
        Some(DataValue::Int(
            crate::runtime::data::convert::to_i128(&DataValue::BigInt(base_int)) as i32,
        ))
    } else if base_int <= BigInt::from(i64::MAX) {
        Some(DataValue::Long(crate::runtime::data::convert::to_i64(
            &DataValue::BigInt(base_int),
        )))
    } else {
        Some(DataValue::BigInt(base_int))
    }
}

/// Java `parseBaseInteger` (hex/binary/octal/decimal by prefix).
fn parse_base_integer(int_text: &str) -> Option<BigInt> {
    if int_text.is_empty() {
        return None;
    }
    let (digits, radix) = if int_text.starts_with("0x") || int_text.starts_with("0X") {
        (&int_text[2..], 16)
    } else if int_text.starts_with("0b") || int_text.starts_with("0B") {
        (&int_text[2..], 2)
    } else if int_text.starts_with('0') && int_text.chars().count() > 1 {
        (int_text, 8)
    } else {
        (int_text, 10)
    };
    let normalized = normalize_java_digits(digits, radix)?;
    BigInt::parse_bytes(normalized.as_bytes(), radix)
}

/// Java `parseFloating` (f/F/d/D suffixes, Double/BigDecimal by exact
/// representability).
fn parse_floating_literal(floating_text: &str) -> Option<DataValue> {
    let last = floating_text.chars().next_back()?;
    let (base_text, flag) = match last {
        'f' | 'F' | 'd' | 'D' => (
            &floating_text[..floating_text.len() - last.len_utf8()],
            Some(last),
        ),
        _ => (floating_text, None),
    };
    let normalized = normalize_java_digits(base_text, 10)?;
    match flag {
        Some('f' | 'F') => normalized.parse::<f32>().ok().map(DataValue::Float),
        Some('d' | 'D') => normalized.parse::<f64>().ok().map(DataValue::Double),
        _ => {
            // Java: baseDecimal.compareTo(MAX_DOUBLE) <= 0 ?
            // maybePresentWithDouble : baseDecimal
            match cmp_decimal(&normalized, MAX_DOUBLE_TEXT) {
                Some(std::cmp::Ordering::Greater) => Some(DataValue::BigDec(normalized)),
                Some(_) => Some(maybe_present_with_double(&normalized)?),
                None => None,
            }
        }
    }
}

/// 把 Java `Character.digit(char, radix)` 可识别的字符规范化为 ASCII。
fn normalize_java_digits(text: &str, radix: u32) -> Option<String> {
    let mut normalized = String::with_capacity(text.len());
    for character in text.chars() {
        if let Some(value) = java_digit_value(character, radix) {
            normalized.push(char::from_digit(value, radix)?);
        } else {
            normalized.push(character);
        }
    }
    Some(normalized)
}

/// Java `maybePresentWithDouble`: present with a double when the double's
/// exact value equals the literal's decimal value, else BigDecimal.
fn maybe_present_with_double(origin_text: &str) -> Option<DataValue> {
    let double_value: f64 = origin_text.parse().ok()?;
    if double_value.is_infinite() {
        return Some(DataValue::BigDec(origin_text.to_string()));
    }
    // `new BigDecimal(double)` uses the exact binary value; 1100 fraction
    // digits cover the full decimal expansion of any f64.
    let reference = format!("{double_value:.1100}");
    if cmp_decimal(&reference, origin_text) == Some(std::cmp::Ordering::Equal) {
        Some(DataValue::Double(double_value))
    } else {
        Some(DataValue::BigDec(origin_text.to_string()))
    }
}

/// Canonical decimal parts: (negative, significant digits without
/// leading/trailing zeros, exponent) with value = 0.d1d2... * 10^(exp+len).
fn decimal_parts(text: &str) -> Option<(bool, Vec<u8>, i64)> {
    let mut s = text.trim();
    let mut negative = false;
    if let Some(rest) = s.strip_prefix('-') {
        negative = true;
        s = rest;
    } else if let Some(rest) = s.strip_prefix('+') {
        s = rest;
    }
    let (mantissa, exp) = match s.find(['e', 'E']) {
        Some(i) => (&s[..i], s[i + 1..].parse::<i64>().ok()?),
        None => (s, 0),
    };
    let mut digits: Vec<u8> = Vec::new();
    let mut frac_len: i64 = 0;
    let mut seen_point = false;
    for c in mantissa.chars() {
        if c == '.' {
            if seen_point {
                return None;
            }
            seen_point = true;
            continue;
        }
        if !c.is_ascii_digit() {
            return None;
        }
        digits.push(c as u8 - b'0');
        if seen_point {
            frac_len += 1;
        }
    }
    // value = digits * 10^(exp - frac_len)
    let mut exp10 = exp - frac_len;
    let leading_zeros = digits.iter().take_while(|&&d| d == 0).count();
    digits.drain(..leading_zeros);
    while digits.last() == Some(&0) {
        digits.pop();
        exp10 += 1;
    }
    Some((negative, digits, exp10))
}

/// Compare two decimal literals numerically (Java `BigDecimal.compareTo`).
fn cmp_decimal(a: &str, b: &str) -> Option<std::cmp::Ordering> {
    use std::cmp::Ordering;
    let (na, da, ea) = decimal_parts(a)?;
    let (nb, db, eb) = decimal_parts(b)?;
    if da.is_empty() && db.is_empty() {
        return Some(Ordering::Equal);
    }
    if na != nb {
        return Some(if na {
            Ordering::Less
        } else {
            Ordering::Greater
        });
    }
    let ordering = if da.is_empty() {
        Ordering::Less
    } else if db.is_empty() {
        Ordering::Greater
    } else {
        // order of magnitude: value = 0.digits * 10^(exp + len)
        let ma = ea + da.len() as i64;
        let mb = eb + db.len() as i64;
        if ma != mb {
            ma.cmp(&mb)
        } else {
            let n = da.len().max(db.len());
            let mut ord = Ordering::Equal;
            for i in 0..n {
                let x = da.get(i).copied().unwrap_or(0);
                let y = db.get(i).copied().unwrap_or(0);
                if x != y {
                    ord = x.cmp(&y);
                    break;
                }
            }
            ord
        }
    };
    Some(if na { ordering.reverse() } else { ordering })
}

// ---------------------------------------------------------------------------
// CodeGenerator handed to compile-time functions (Java anonymous class in
// `visitCallFunction`).
// ---------------------------------------------------------------------------

struct VisitorCodeGenerator<'v, 'a> {
    visitor: &'v mut QvmInstructionVisitor<'a>,
    function_name: String,
    function_token: Token,
    reporter: Rc<dyn ErrorReporter>,
}

impl CodeGenerator for VisitorCodeGenerator<'_, '_> {
    fn add_instruction(&mut self, instruction: Instruction) {
        self.visitor.add_instruction(instruction);
    }

    fn add_instructions_by_tree(&mut self, tree: &Node) {
        tree.accept(self.visitor);
    }

    fn report_parse_err(&mut self, err_code: &str, err_reason: &str) -> QLSyntaxException {
        let token = self.function_token.clone();
        self.visitor.report_parse_err(&token, err_code, err_reason)
    }

    fn generate_lambda_definition(
        &mut self,
        expression: &Node,
        params: Vec<Param>,
    ) -> Rc<dyn QLambdaDefinition> {
        let scope = Rc::clone(&self.visitor.generator_scope);
        let context = self.visitor.context;
        let sub_visitor = self
            .visitor
            .parse_expr_body_with_sub_visitor(expression, scope, context);
        let max_stack_size = sub_visitor.max_stack_size();
        let instructions = sub_visitor.take_instructions();
        Rc::new(QLambdaDefinitionInner::new(
            self.function_name.clone(),
            instructions,
            params,
            max_stack_size,
        ))
    }

    fn error_reporter(&self) -> Rc<dyn ErrorReporter> {
        Rc::clone(&self.reporter)
    }

    fn new_reporter_with_token(&self, token: &Token) -> Rc<dyn ErrorReporter> {
        self.visitor.new_reporter_with_token(token)
    }
}

// ---------------------------------------------------------------------------
// Convenience entry point (Java Express4Runner's parse+compile pipeline).
// ---------------------------------------------------------------------------

/// 构建或解析 script。
/// 参数：`script`、`tree`、`import_manager`、`global_scope`、`operator_factory`、`compile_time_functions`、`user_define_functions`、`init_options`；返回：`Result<(Vec<Instruction>, usize), QLSyntaxException>`。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/Express4Runner.java`，方法 `compileScript`；Rust 侧按所有权与 `Result` 语义适配。
/// Compile a parsed `Program` tree into an instruction sequence, mirroring
/// Java `Express4Runner`: `tree.accept(new QvmInstructionVisitor(...))`.
///
/// Returns the instructions and the max operand-stack size (for
/// `QLambdaDefinitionInner`), or the first syntax error.
#[allow(clippy::too_many_arguments)]
/// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#compileScript。
pub fn compile_script<'a>(
    script: &'a str,
    tree: &Node,
    import_manager: &'a RefCell<ImportManager<'a>>,
    global_scope: Option<Rc<InstructionScope>>,
    operator_factory: &'a dyn OperatorFactory,
    compile_time_functions: &'a CompileTimeFunctions,
    user_define_functions: &'a UserDefineFunctions,
    init_options: &'a InitOptions,
) -> Result<(Vec<Instruction>, usize), QLSyntaxException> {
    let visitor = QvmInstructionVisitor::new(
        script,
        import_manager,
        global_scope,
        operator_factory,
        compile_time_functions,
        user_define_functions,
        init_options,
    );
    visitor.compile(tree)
}

#[cfg(test)]
mod literal_parity_tests {
    use super::*;

    /// SOURCE_PARITY: Java `BigInteger(String,radix)` 通过
    /// `Character.digit` 接受 Unicode 数字和全角十六进制字母。
    #[test]
    fn parses_java_unicode_integer_digits() {
        assert_eq!(parse_integer_literal("١٢"), Some(DataValue::Int(12)));
        assert_eq!(parse_integer_literal("0xＦＦ"), Some(DataValue::Int(255)));
    }

    /// SOURCE_PARITY: Java `BigDecimal(String)` 接受 Unicode 十进制数字，
    /// 结果格式化为规范 ASCII 数字。
    #[test]
    fn parses_java_unicode_floating_digits() {
        assert_eq!(
            parse_floating_literal("١٢.٥"),
            Some(DataValue::Double(12.5))
        );
    }
}
