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

impl<'a> QvmInstructionVisitor<'a> {
    /// 创建对象实例。
    /// 参数：`script`、`import_manager`、`global_scope`、`operator_factory`、`compile_time_functions`、`user_define_functions`、`init_options`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java main constructor: `new QvmInstructionVisitor(script,
    /// importManager, globalScope, operatorFactory, compileTimeFunctions,
    /// userDefineFunctions, initOptions)`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#new。
    pub fn new(
        script: &'a str,
        import_manager: &'a RefCell<ImportManager<'a>>,
        global_scope: Option<Rc<InstructionScope>>,
        operator_factory: &'a dyn OperatorFactory,
        compile_time_functions: &'a CompileTimeFunctions,
        user_define_functions: &'a UserDefineFunctions,
        init_options: &'a InitOptions,
    ) -> Self {
        Self::for_recursion(
            script,
            import_manager,
            Rc::new(GeneratorScope::new("main", global_scope)),
            operator_factory,
            Context::Block,
            compile_time_functions,
            user_define_functions,
            init_options,
        )
    }

    /// 附加 context 配置并返回新值。
    /// 参数：`script`、`import_manager`、`generator_scope`、`operator_factory`、`context`、`compile_time_functions`、`user_define_functions`、`init_options`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，方法 `withContext`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java macro constructor: `new QvmInstructionVisitor(script,
    /// importManager, generatorScope, operatorFactory, context,
    /// compileTimeFunctions, userDefineFunctions, initOptions)` —
    /// used by `Express4Runner.parseMacroDefine` with `Context.MACRO`.
    #[allow(clippy::too_many_arguments)]
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#withContext。
    pub fn with_context(
        script: &'a str,
        import_manager: &'a RefCell<ImportManager<'a>>,
        generator_scope: Rc<InstructionScope>,
        operator_factory: &'a dyn OperatorFactory,
        context: Context,
        compile_time_functions: &'a CompileTimeFunctions,
        user_define_functions: &'a UserDefineFunctions,
        init_options: &'a InitOptions,
    ) -> Self {
        Self::for_recursion(
            script,
            import_manager,
            generator_scope,
            operator_factory,
            context,
            compile_time_functions,
            user_define_functions,
            init_options,
        )
    }

    /// Java recursion constructor.
    #[expect(
        clippy::too_many_arguments,
        reason = "对应 Java QvmInstructionVisitor 递归构造器的参数契约"
    )]
    fn for_recursion(
        script: &'a str,
        import_manager: &'a RefCell<ImportManager<'a>>,
        generator_scope: Rc<InstructionScope>,
        operator_factory: &'a dyn OperatorFactory,
        context: Context,
        compile_time_functions: &'a CompileTimeFunctions,
        user_define_functions: &'a UserDefineFunctions,
        init_options: &'a InitOptions,
    ) -> Self {
        QvmInstructionVisitor {
            script,
            import_manager,
            generator_scope,
            operator_factory,
            compile_time_functions,
            user_define_functions,
            init_options,
            context,
            instruction_list: Vec::new(),
            stack_size: 0,
            max_stack_size: 0,
            if_counter: 0,
            switch_counter: 0,
            block_counter: 0,
            macro_counter: 0,
            lambda_counter: 0,
            lazy_function_counter: 0,
            try_counter: 0,
            for_counter: 0,
            while_counter: 0,
            timeout_check_point: -1,
            syntax_error: None,
            last_is_timeout_check: false,
            collect_break_indices: None,
        }
    }

    /// 返回已经生成的 QVM 指令只读切片。
    /// 无显式参数；返回：`&[Instruction]`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/instruction/QLInstruction.java`，方法 `instructions`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getInstructions`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#instructions。
    pub fn instructions(&self) -> &[Instruction] {
        &self.instruction_list
    }

    /// 移出并返回已经生成的全部 QVM 指令。
    /// 无显式参数；返回：`Vec<Instruction>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，方法 `takeInstructions`；Rust 侧按所有权与 `Result` 语义适配。
    /// Take the compiled instruction list (sub-visitor handover).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#takeInstructions。
    pub fn take_instructions(self) -> Vec<Instruction> {
        self.instruction_list
    }

    /// 返回编译结果所需的最大操作数栈深。
    /// 无显式参数；返回：`usize`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，方法 `maxStackSize`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getMaxStackSize`.
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#maxStackSize。
    pub fn max_stack_size(&self) -> usize {
        self.max_stack_size as usize
    }

    /// 按当前源码位置构造编译期语法错误。
    /// 无显式参数；返回：`Option<&QLSyntaxException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，方法 `syntaxError`；Rust 侧按所有权与 `Result` 语义适配。
    /// The first recorded syntax error, if any (Java: thrown).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#syntaxError。
    pub fn syntax_error(&self) -> Option<&QLSyntaxException> {
        self.syntax_error.as_ref()
    }

    /// 把语法树编译为可执行 QVM 指令。
    /// 参数：`tree`；返回：`Result<(Vec<Instruction>, usize), QLSyntaxException>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/QvmInstructionVisitor.java`，方法 `compile`；Rust 侧按所有权与 `Result` 语义适配。
    /// Compile `tree` and return the instructions plus max stack size, or
    /// the first syntax error (Java: the exception unwinds `accept`).
    /// 对应 Java: com.alibaba.qlexpress4.aparser.QvmInstructionVisitor#compile。
    pub fn compile(mut self, tree: &Node) -> Result<(Vec<Instruction>, usize), QLSyntaxException> {
        tree.accept(&mut self);
        match self.syntax_error {
            Some(err) => Err(err),
            None => Ok((self.instruction_list, self.max_stack_size as usize)),
        }
    }

    // ------------------------------------------------------------------
    // Error plumbing (Java throws QLSyntaxException)
    // ------------------------------------------------------------------

    /// Stop emitting once an error was recorded.
    fn failed(&self) -> bool {
        self.syntax_error.is_some()
    }

    /// Java `reportParseErr`.
    fn report_parse_err(
        &mut self,
        token: &Token,
        err_code: &str,
        err_reason: &str,
    ) -> QLSyntaxException {
        let err = QLException::report_scanner_err(
            self.script,
            token.start_index(),
            token.line(),
            token.char_position_in_line() + 1,
            token.text(),
            err_code,
            err_reason,
        );
        if self.syntax_error.is_none() {
            self.syntax_error = Some(err.clone());
        }
        err
    }

    /// Absorb a sub-visitor's error (Java: the exception propagates out of
    /// the recursive `accept`).
    fn propagate_error(&mut self, sub: &QvmInstructionVisitor<'a>) {
        if self.syntax_error.is_none() {
            if let Some(err) = sub.syntax_error() {
                self.syntax_error = Some(err.clone());
            }
        }
    }

    // ------------------------------------------------------------------
    // Sub-visitor plumbing (Java `parseWithSubVisitor` etc.)
    // ------------------------------------------------------------------

    /// Java `parseWithSubVisitor`.
    fn parse_with_sub_visitor(
        &mut self,
        node: &Node,
        generator_scope: Rc<InstructionScope>,
        context: Context,
    ) -> QvmInstructionVisitor<'a> {
        let mut sub = self.sub_visitor(generator_scope, context);
        node.accept(&mut sub);
        self.propagate_error(&sub);
        sub
    }

    /// Java `parseExprBodyWithSubVisitor`.
    fn parse_expr_body_with_sub_visitor(
        &mut self,
        expression: &Node,
        generator_scope: Rc<InstructionScope>,
        context: Context,
    ) -> QvmInstructionVisitor<'a> {
        let mut sub = self.sub_visitor(generator_scope, context);
        // reduce the level of syntax tree when expression is a block
        sub.visit_body_expression(expression);
        self.propagate_error(&sub);
        sub
    }

    fn sub_visitor(
        &self,
        generator_scope: Rc<InstructionScope>,
        context: Context,
    ) -> QvmInstructionVisitor<'a> {
        QvmInstructionVisitor::for_recursion(
            self.script,
            self.import_manager,
            generator_scope,
            self.operator_factory,
            context,
            self.compile_time_functions,
            self.user_define_functions,
            self.init_options,
        )
    }

    /// Java `visitBodyExpression`: when the body is a bare block `{ ... }`
    /// compile the block inline; otherwise compile the expression and add
    /// a `RETURN` instruction.
    fn visit_body_expression(&mut self, expression: &Node) {
        if self.failed() {
            return;
        }
        if let Some(block_expr) = block_expr_of(expression) {
            if let Some(block_statements) = &block_expr.block_statements {
                block_statements.accept(self);
            }
            return;
        }
        expression.accept(self);
        let reporter = expression
            .start_token()
            .map(|t| self.new_reporter_with_token(t))
            .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));
        self.add_instruction(Box::new(ReturnInstruction::new(
            reporter,
            ReturnResultType::Return,
            None,
        )));
    }

    // ------------------------------------------------------------------
    // Instruction emission (Java pureAddInstruction/addInstruction/
    // addTimeoutInstruction/expandStackSize)
    // ------------------------------------------------------------------

    /// Java `pureAddInstruction`.
    fn pure_add_instruction(&mut self, instruction: Instruction) {
        let stack_expand_size = instruction.stack_output() - instruction.stack_input();
        self.expand_stack_size(stack_expand_size);
        self.last_is_timeout_check = false;
        self.instruction_list.push(instruction);
    }

    /// Push a shared instruction (Java shares the same object, e.g. macro
    /// bodies and back-patched jumps).
    fn pure_add_shared(&mut self, instruction: SharedInstruction) {
        let stack_expand_size = instruction.stack_output() - instruction.stack_input();
        self.expand_stack_size(stack_expand_size);
        self.last_is_timeout_check = false;
        self.instruction_list.push(Box::new(instruction));
    }

    /// Java `addInstruction` for a regular (non-call) instruction.
    fn add_instruction(&mut self, instruction: Instruction) {
        self.add_instruction_inner(instruction, false);
    }

    /// Java `addInstruction` for `MethodInvokeInstruction` /
    /// `CallFunctionInstruction` / `CallConstInstruction` /
    /// `CallInstruction` (a timeout check follows).
    fn add_call_instruction(&mut self, instruction: Instruction) {
        self.add_instruction_inner(instruction, true);
    }

    fn add_instruction_inner(&mut self, instruction: Instruction, is_call: bool) {
        if self.instruction_list.len() as i32 - self.timeout_check_point > TIMEOUT_CHECK_GAP {
            self.add_timeout_instruction();
        }
        self.pure_add_instruction(instruction);
        if is_call {
            self.add_timeout_instruction();
        }
    }

    /// Java `addTimeoutInstruction`.
    fn add_timeout_instruction(&mut self) {
        if self.last_is_timeout_check {
            return;
        }
        let Some(last_instruction) = self.instruction_list.last() else {
            return;
        };
        let reporter = Rc::clone(last_instruction.error_reporter());
        self.timeout_check_point = self.instruction_list.len() as i32;
        self.last_is_timeout_check = true;
        self.instruction_list
            .push(Box::new(CheckTimeOutInstruction::new(reporter)));
    }

    /// Java `expandStackSize`.
    fn expand_stack_size(&mut self, stack_expand_size: i32) {
        self.stack_size += stack_expand_size;
        if self.stack_size > self.max_stack_size {
            self.max_stack_size = self.stack_size;
        }
    }

    // ------------------------------------------------------------------
    // Reporters and counters
    // ------------------------------------------------------------------

    /// Java `newReporterWithToken`.
    fn new_reporter_with_token(&self, token: &Token) -> Rc<dyn ErrorReporter> {
        Rc::new(DefaultErrReporter::new(
            self.script,
            token.start_index(),
            token.line(),
            token.char_position_in_line() + 1,
            token.text(),
        ))
    }

    fn reporter_of(&self, node: &Node) -> Rc<dyn ErrorReporter> {
        node.start_token()
            .map(|t| self.new_reporter_with_token(t))
            .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE))
    }

    fn while_count(&mut self) -> i32 {
        let count = self.while_counter;
        self.while_counter += 1;
        count
    }

    fn for_count(&mut self) -> i32 {
        let count = self.for_counter;
        self.for_counter += 1;
        count
    }

    fn if_count(&mut self) -> i32 {
        let count = self.if_counter;
        self.if_counter += 1;
        count
    }

    fn switch_count(&mut self) -> i32 {
        let count = self.switch_counter;
        self.switch_counter += 1;
        count
    }

    fn lazy_function_count(&mut self) -> i32 {
        let count = self.lazy_function_counter;
        self.lazy_function_counter += 1;
        count
    }

    fn try_count(&mut self) -> i32 {
        let count = self.try_counter;
        self.try_counter += 1;
        count
    }

    fn block_scope_name(&mut self) -> String {
        let name = format!(
            "{}{}{}{}",
            self.generator_scope.name(),
            SCOPE_SEPARATOR,
            BLOCK_LAMBDA_NAME_PREFIX,
            self.block_counter
        );
        self.block_counter += 1;
        name
    }

    fn macro_scope_name(&mut self) -> String {
        let name = format!(
            "{}{}{}{}",
            self.generator_scope.name(),
            SCOPE_SEPARATOR,
            MACRO_PREFIX,
            self.macro_counter
        );
        self.macro_counter += 1;
        name
    }

    fn lambda_scope_name(&mut self) -> String {
        let name = format!(
            "{}{}{}{}",
            self.generator_scope.name(),
            SCOPE_SEPARATOR,
            LAMBDA_PREFIX,
            self.lambda_counter
        );
        self.lambda_counter += 1;
        name
    }

    /// Java `generatorScope.getName() + SCOPE_SEPARATOR + ...`.
    fn child_scope_name(&self, stem: &str) -> String {
        format!("{}{}{}", self.generator_scope.name(), SCOPE_SEPARATOR, stem)
    }

    fn child_scope(&self, name: impl Into<String>) -> Rc<InstructionScope> {
        Rc::new(GeneratorScope::new(
            name,
            Some(Rc::clone(&self.generator_scope)),
        ))
    }

    // ------------------------------------------------------------------
    // Shared compile helpers
    // ------------------------------------------------------------------

    /// Java `ifElseInstructions`.
    fn if_else_instructions(
        &mut self,
        condition_reporter: Rc<dyn ErrorReporter>,
        then_instructions: Vec<Instruction>,
        then_trace_key: Option<i32>,
        else_instructions: Vec<Instruction>,
        else_trace_key: Option<i32>,
        trace_key: Option<i32>,
    ) {
        let jump_if = Rc::new(JumpIfPopInstruction::new(
            Rc::clone(&condition_reporter),
            false,
            -1,
        ));
        self.pure_add_shared(Rc::clone(&jump_if) as SharedInstruction);
        let mut jump_start = self.instruction_list.len();
        for instruction in then_instructions {
            self.pure_add_instruction(instruction);
        }
        if self.init_options.is_trace_expression() {
            if then_trace_key.is_some() {
                self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                    Rc::clone(&condition_reporter),
                    then_trace_key,
                )));
            }
            self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                Rc::clone(&condition_reporter),
                trace_key,
            )));
        }
        self.add_timeout_instruction();

        let jump = Rc::new(JumpInstruction::new(Rc::clone(&condition_reporter), -1));
        self.pure_add_shared(Rc::clone(&jump) as SharedInstruction);

        jump_if.set_position((self.instruction_list.len() - jump_start) as i32);

        jump_start = self.instruction_list.len();
        for instruction in else_instructions {
            self.pure_add_instruction(instruction);
        }
        if self.init_options.is_trace_expression() {
            if else_trace_key.is_some() {
                self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                    Rc::clone(&condition_reporter),
                    else_trace_key,
                )));
            }
            self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                Rc::clone(&condition_reporter),
                trace_key,
            )));
        }
        self.add_timeout_instruction();
        jump.set_position((self.instruction_list.len() - jump_start) as i32);
    }

    /// Java `jumpRightIfExpect` (short-circuit `&&` / `||`).
    fn jump_right_if_expect(
        &mut self,
        expect: bool,
        op_err_reporter: Rc<dyn ErrorReporter>,
        right: &Node,
        operator_id: &str,
        trace_key: Option<i32>,
    ) {
        let right_visitor =
            self.parse_with_sub_visitor(right, Rc::clone(&self.generator_scope), Context::Macro);
        let right_instructions = right_visitor.take_instructions();

        let jump_if = Rc::new(JumpIfInstruction::new(
            Rc::clone(&op_err_reporter),
            expect,
            -1,
            trace_key,
        ));
        self.pure_add_shared(Rc::clone(&jump_if) as SharedInstruction);

        let jump_start = self.instruction_list.len();

        for instruction in right_instructions {
            self.pure_add_instruction(instruction);
        }
        let binary_operator = self
            .operator_factory
            .get_binary_operator(operator_id)
            .expect("short-circuit operator must exist");
        self.add_instruction(Box::new(OperatorInstruction::new(
            op_err_reporter,
            binary_operator,
            trace_key,
        )));
        self.add_timeout_instruction();

        jump_if.set_position((self.instruction_list.len() - jump_start) as i32);
    }

    /// Java `loopBodyVisitorDefinition`.
    fn loop_body_visitor_definition(
        &mut self,
        body: Option<&Node>,
        scope_name: String,
        params_type: Vec<Param>,
        error_reporter: Rc<dyn ErrorReporter>,
    ) -> (Rc<dyn QLambdaDefinition>, Option<usize>) {
        let Some(body) = body else {
            return (Rc::new(QLambdaDefinitionEmpty::INSTANCE), None);
        };
        let body_scope = self.child_scope(scope_name.clone());
        let body_visitor = self.parse_with_sub_visitor(body, body_scope, Context::Macro);
        let max_stack_size = body_visitor.max_stack_size();
        let body_instructions = body_visitor.take_instructions();

        let mut result_instructions: Vec<Instruction> = Vec::new();
        result_instructions.push(Box::new(CheckTimeOutInstruction::new(error_reporter)));
        result_instructions.extend(body_instructions);

        (
            Rc::new(QLambdaDefinitionInner::new(
                scope_name,
                result_instructions,
                params_type,
                max_stack_size,
            )),
            Some(max_stack_size),
        )
    }

    // ------------------------------------------------------------------
    // Types (Java parseDeclType/parseClsIds/BuiltInTypesSet/wrapInArray)
    // ------------------------------------------------------------------

    /// Java `BuiltInTypesSet.getCls`.
    fn built_in_cls(lexeme: &str) -> Option<ClassRef> {
        let target = match lexeme {
            "byte" => TargetType::Byte,
            "short" => TargetType::Short,
            "int" => TargetType::Int,
            "long" => TargetType::Long,
            "float" => TargetType::Float,
            "double" => TargetType::Double,
            "boolean" => TargetType::Boolean,
            "char" => TargetType::Character,
            _ => return None,
        };
        // Java `BuiltInTypesSet` 返回包装类，而不是 `int.class` 等原语类。
        Some(ClassRef::Boxed(target))
    }

    /// Java `parseDeclTypeNoArr`.
    fn parse_decl_type_no_arr(&mut self, node: &Node) -> ClassRef {
        let Node::DeclTypeNoArr(ctx) = node else {
            return object_cls();
        };
        if let Some(primitive) = &ctx.primitive_type {
            let text = primitive.text();
            if let Some(cls) = Self::built_in_cls(&text) {
                return cls;
            }
        }
        match &ctx.cls_type {
            Some(cls_type) => self.parse_cls_ids(cls_type_children(cls_type)),
            None => object_cls(),
        }
    }

    /// Java `parseDeclType` (base type wrapped in `dims` array layers).
    fn parse_decl_type(&mut self, node: &Node) -> ClassRef {
        let Node::DeclType(ctx) = node else {
            return object_cls();
        };
        let base_cls = if let Some(primitive) = &ctx.primitive_type {
            Self::built_in_cls(&primitive.text()).unwrap_or_else(object_cls)
        } else if let Some(cls_type) = &ctx.cls_type {
            self.parse_cls_ids(cls_type_children(cls_type))
        } else {
            object_cls()
        };
        let layers = ctx.dims.as_ref().map_or(0, |d| dims_dim_count(d));
        wrap_in_array(base_cls, layers)
    }

    /// Java `parseClsIds`: resolve a dotted class name through the import
    /// manager, reporting `CLASS_NOT_FOUND` when unresolvable.
    fn parse_cls_ids(&mut self, var_ids: &[Node]) -> ClassRef {
        let field_ids: Vec<String> = var_ids.iter().map(|id| id.text()).collect();
        let result = self.import_manager.borrow().load_part_qualified(&field_ids);
        match result.cls() {
            Some(cls) if result.rest_index() == field_ids.len() => ClassRef::from_name(cls),
            _ => {
                let last_id = var_ids.last().expect("class ids non-empty");
                let reason = error_codes::format_msg(
                    error_codes::error_msg(error_codes::CLASS_NOT_FOUND),
                    &[field_ids.join(".")],
                );
                if let Some(token) = last_id.start_token() {
                    let token = token.clone();
                    self.report_parse_err(&token, error_codes::CLASS_NOT_FOUND, &reason);
                }
                ClassRef::Named(field_ids.join("."))
            }
        }
    }
}

/// Java `ClsTypeContext.varId()` children.
fn cls_type_children(cls_type: &Node) -> &[Node] {
    match cls_type {
        Node::ClsType(ctx) => &ctx.var_ids,
        _ => &[],
    }
}

/// Java `DimsContext.LBRACK().size()`.
fn dims_dim_count(dims: &Node) -> usize {
    match dims {
        Node::Dims(ctx) => ctx.dim_count(),
        _ => 0,
    }
}

/// Java `wrapInArray`: array class literals are represented by appending
/// `[]` to the Java-style name (Rust has no array class object; only used
/// for `MetaClass` constants and error messages).
fn wrap_in_array(base_type: ClassRef, layers: usize) -> ClassRef {
    let mut result = base_type;
    for _ in 0..layers {
        result = ClassRef::array_of(result);
    }
    result
}

/// Java `Object.class` (untyped declaration target).
fn object_cls() -> ClassRef {
    ClassRef::Named("java.lang.Object".to_string())
}

/// Java `blockExpr`: reduce `{ ... }` used as an expression body.
fn block_expr_of(expression: &Node) -> Option<&BlockExprContext> {
    let Node::Expression(ctx) = expression else {
        return None;
    };
    if ctx.is_assign() {
        return None;
    }
    // Java fast fail: start token `{` and stop token `}`.
    if expression.start_token().map(Token::text) != Some("{")
        || expression.stop_token().map(Token::text) != Some("}")
    {
        return None;
    }
    let ternary = ctx.ternary.as_deref()?;
    let Node::TernaryExpr(ternary_ctx) = ternary else {
        return None;
    };
    if ternary_ctx.question.is_some() {
        return None;
    }
    let Node::BaseExpr(base_expr) = &*ternary_ctx.condition else {
        return None;
    };
    if !base_expr.left_assos.is_empty() {
        return None;
    }
    let Node::Primary(primary) = &*base_expr.primary else {
        return None;
    };
    if primary.non_pathable.is_some() {
        return None;
    }
    // Java checks neither prefix/suffix nor path parts here.
    match primary.pathable.as_deref() {
        Some(Node::BlockExpr(block_expr)) => Some(block_expr),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Statements
// ---------------------------------------------------------------------------

impl<'a> QvmInstructionVisitor<'a> {
    /// Java `handleStmt`: inline a macro call or compile the statement;
    /// returns whether the statement is an expression statement (its value
    /// remains on the stack).
    fn handle_stmt(&mut self, statement: &Node) -> bool {
        if self.maybe_macro_call(statement) {
            let macro_name = statement
                .start_token()
                .map(Token::text)
                .unwrap_or_default()
                .to_string();
            if let Some(macro_define) = self.generator_scope.get_macro_instructions(&macro_name) {
                for instruction in macro_define.macro_instructions() {
                    self.pure_add_shared(Rc::clone(instruction));
                }
                self.add_timeout_instruction();
                return macro_define.is_last_stmt_express();
            }
        }
        statement.accept(self);
        matches!(statement, Node::ExpressionStatement(_))
    }

    /// Java `maybeMacroCall`: an expression statement consisting of a
    /// single `ID` token.
    fn maybe_macro_call(&self, statement: &Node) -> bool {
        let Node::ExpressionStatement(expr_stmt) = statement else {
            return false;
        };
        let expression = &expr_stmt.expression;
        let (Some(start), Some(stop)) = (expression.start_token(), expression.stop_token()) else {
            return false;
        };
        std::ptr::eq(start, stop) && start.token_type() == token::ID as i32
    }

    /// Java `generateForInitLambda`.
    fn generate_for_init_lambda(
        &mut self,
        for_count: i32,
        for_init: &Node,
    ) -> Option<Rc<QLambdaDefinitionInner>> {
        let Node::ForInit(ctx) = for_init else {
            return None;
        };
        if let Some(local_variable_declaration) = &ctx.local_variable_declaration {
            let scope_name =
                self.child_scope_name(&format!("{FOR_PREFIX}{for_count}{INIT_SUFFIX}"));
            let scope = self.child_scope(scope_name.clone());
            let sub =
                self.parse_with_sub_visitor(local_variable_declaration, scope, Context::Macro);
            let max_stack_size = sub.max_stack_size();
            let instructions = sub.take_instructions();
            Some(Rc::new(QLambdaDefinitionInner::new(
                scope_name,
                instructions,
                vec![],
                max_stack_size,
            )))
        } else {
            ctx.expression
                .as_ref()
                .and_then(|expr| self.generate_for_express_lambda(for_count, INIT_SUFFIX, expr))
        }
    }

    /// Java `generateForExpressLambda`.
    fn generate_for_express_lambda(
        &mut self,
        for_count: i32,
        scope_suffix: &str,
        expression: &Node,
    ) -> Option<Rc<QLambdaDefinitionInner>> {
        let scope_name = self.child_scope_name(&format!("{FOR_PREFIX}{for_count}{scope_suffix}"));
        let scope = self.child_scope(scope_name.clone());
        let sub = self.parse_expr_body_with_sub_visitor(expression, scope, Context::Block);
        let max_stack_size = sub.max_stack_size();
        let instructions = sub.take_instructions();
        Some(Rc::new(QLambdaDefinitionInner::new(
            scope_name,
            instructions,
            vec![],
            max_stack_size,
        )))
    }

    /// Java `parseFunctionDefinition`.
    fn parse_function_definition(
        &mut self,
        function_name: &str,
        block_statements: Option<&Node>,
        params: Vec<Param>,
    ) -> Rc<dyn QLambdaDefinition> {
        let Some(block_statements) = block_statements else {
            return Rc::new(QLambdaDefinitionInner::new(
                function_name,
                vec![],
                params,
                0,
            ));
        };
        let scope = self.child_scope(function_name);
        let sub = self.parse_with_sub_visitor(block_statements, scope, Context::Block);
        let max_stack_size = sub.max_stack_size();
        let instructions = sub.take_instructions();
        Rc::new(QLambdaDefinitionInner::new(
            function_name,
            instructions,
            params,
            max_stack_size,
        ))
    }

    /// Java `parseFormalOrInferredParameterList`.
    fn parse_formal_or_inferred_parameter_list(&mut self, params_node: &Node) -> Vec<Param> {
        let Node::FormalOrInferredParameterList(ctx) = params_node else {
            return vec![];
        };
        ctx.params
            .iter()
            .map(|param| self.formal_or_inferred_parameter_to_param(param))
            .collect()
    }

    /// Java `formalOrInferredParameter2Param`.
    fn formal_or_inferred_parameter_to_param(&mut self, param_node: &Node) -> Param {
        let Node::FormalOrInferredParameter(ctx) = param_node else {
            return Param::new("", None);
        };
        let param_name = ctx.var_id.text();
        let param_cls = ctx
            .decl_type
            .as_ref()
            .map(|decl| self.parse_decl_type(decl));
        Param::new(param_name, param_cls)
    }

    /// Java `parseLambdaParams`.
    fn parse_lambda_params(&mut self, lambda_parameters: &Node) -> Vec<Param> {
        let Node::LambdaParameters(ctx) = lambda_parameters else {
            return vec![];
        };
        if let Some(var_id) = &ctx.var_id {
            return vec![Param::new(var_id.text(), Some(object_cls()))];
        }
        match &ctx.params {
            Some(params) => self.parse_formal_or_inferred_parameter_list(params),
            None => vec![],
        }
    }

    /// Java `parseExceptionTable`.
    fn parse_exception_table(
        &mut self,
        try_count: i32,
        try_catches: Option<&Node>,
    ) -> Vec<(ClassRef, Rc<dyn QLambdaDefinition>)> {
        let mut exception_table = Vec::new();
        let Some(Node::TryCatches(catches_ctx)) = try_catches else {
            return exception_table;
        };
        for try_catch in &catches_ctx.catches {
            let Node::TryCatch(try_catch_ctx) = try_catch else {
                continue;
            };
            let Node::CatchParams(catch_params) = &*try_catch_ctx.catch_params else {
                continue;
            };
            let e_name = catch_params.var_id.text();
            let catch_body_name =
                self.child_scope_name(&format!("{TRY_PREFIX}{try_count}{CATCH_SUFFIX}"));
            let catch_sub = try_catch_ctx.block_statements.as_ref().map(|block| {
                let scope = self.child_scope(catch_body_name.clone());
                self.parse_with_sub_visitor(block, scope, Context::Block)
            });
            // Java compiles the body once and shares the instruction list
            // across the catch's declared types. The port moves the
            // compiled instructions into the first handler and recompiles
            // the body for each additional declared type (identical
            // instruction sequences, fresh objects).
            let mut compiled = catch_sub.map(|sub| (sub.max_stack_size(), sub.take_instructions()));
            let mut handler_for = |visitor: &mut Self, param: Param| -> Rc<dyn QLambdaDefinition> {
                match compiled.take() {
                    Some((max_stack, instructions)) => Rc::new(QLambdaDefinitionInner::new(
                        catch_body_name.clone(),
                        instructions,
                        vec![param],
                        max_stack,
                    )),
                    None => match &try_catch_ctx.block_statements {
                        None => Rc::new(QLambdaDefinitionEmpty::INSTANCE),
                        Some(block) => {
                            let scope = visitor.child_scope(catch_body_name.clone());
                            let sub = visitor.parse_with_sub_visitor(block, scope, Context::Block);
                            let max_stack = sub.max_stack_size();
                            Rc::new(QLambdaDefinitionInner::new(
                                catch_body_name.clone(),
                                sub.take_instructions(),
                                vec![param],
                                max_stack,
                            ))
                        }
                    },
                }
            };
            if catch_params.decl_types.is_empty() {
                let handler =
                    handler_for(self, Param::new(e_name.clone(), Some(object_cls())));
                exception_table.push((object_cls(), handler));
            }
            for decl_type in &catch_params.decl_types {
                let exception_type = self.parse_decl_type(decl_type);
                let param = Param::new(e_name.clone(), Some(exception_type.clone()));
                let handler = handler_for(self, param);
                exception_table.push((exception_type, handler));
            }
        }
        exception_table
    }

    /// Java `parseFinalBodyDefinition`.
    fn parse_final_body_definition(
        &mut self,
        try_count: i32,
        try_finally: Option<&Node>,
    ) -> Option<Rc<dyn QLambdaDefinition>> {
        let Node::TryFinally(finally_ctx) = try_finally? else {
            return None;
        };
        let block_statements = finally_ctx.block_statements.as_ref()?;
        let final_scope_name =
            self.child_scope_name(&format!("{TRY_PREFIX}{try_count}{FINAL_SUFFIX}"));
        let scope = self.child_scope(final_scope_name.clone());
        let sub = self.parse_with_sub_visitor(block_statements, scope, Context::Block);
        let max_stack_size = sub.max_stack_size();
        let instructions = sub.take_instructions();
        Some(Rc::new(QLambdaDefinitionInner::new(
            final_scope_name,
            instructions,
            vec![],
            max_stack_size,
        )))
    }

    /// Java `ifBodyFillConst`.
    fn if_body_fill_const(
        expression: Option<&Node>,
        non_expression_statement: Option<&Node>,
        block_statements: Option<&Node>,
    ) -> bool {
        if expression.is_some() {
            return false;
        }
        if let Some(non_expression_statement) = non_expression_statement {
            return Self::non_expression_stmt_fill_const(non_expression_statement);
        }
        if let Some(block_statements) = block_statements {
            if let Node::BlockStatements(ctx) = block_statements {
                let statements: Vec<&Node> = ctx
                    .statements
                    .iter()
                    .filter(|bs| !matches!(bs, Node::EmptyStatement(_)))
                    .collect();
                return statements
                    .last()
                    .is_none_or(|last| Self::block_stmt_fill_const(last));
            }
            return true;
        }
        true
    }

    /// Java `stmtFillConst(NonExpressionStatementContext)`: fill unless the
    /// statement is a `return`.
    fn non_expression_stmt_fill_const(non_expression_statement: &Node) -> bool {
        let Node::NonExpressionStatement(ctx) = non_expression_statement else {
            return true;
        };
        !matches!(&*ctx.statement, Node::ReturnStatement(_))
    }

    /// Java `stmtFillConst(BlockStatementContext)`.
    fn block_stmt_fill_const(block_statement: &Node) -> bool {
        !matches!(
            block_statement,
            Node::ExpressionStatement(_) | Node::ReturnStatement(_)
        )
    }

    /// Java `getMacroLastStmt`: whether the macro's last statement is an
    /// expression statement.
    fn macro_last_stmt_is_expression(macro_block_statements: Option<&Node>) -> bool {
        let Some(Node::BlockStatements(ctx)) = macro_block_statements else {
            return false;
        };
        ctx.statements
            .iter()
            .rfind(|bs| !matches!(bs, Node::EmptyStatement(_)))
            .is_some_and(|last| matches!(last, Node::ExpressionStatement(_)))
    }

    /// Java `getMacroInstructions`.
    fn get_macro_instructions(
        &mut self,
        macro_block_statements: Option<&Node>,
    ) -> Vec<SharedInstruction> {
        let Some(block_statements) = macro_block_statements else {
            return vec![];
        };
        let scope_name = self.macro_scope_name();
        let scope = self.child_scope(scope_name);
        let sub = self.parse_with_sub_visitor(block_statements, scope, Context::Macro);
        sub.take_instructions().into_iter().map(Rc::from).collect()
    }

    /// Java `parseInitializer`.
    fn parse_initializer(&mut self, variable_initializer: &Node, decl_cls: &ClassRef) {
        let Node::VariableInitializer(ctx) = variable_initializer else {
            return;
        };
        if let Some(expression) = &ctx.expression {
            expression.accept(self);
            return;
        }
        if let Some(array_initializer) = &ctx.array_initializer {
            self.new_arr_with_initializers(decl_cls.clone(), array_initializer);
        }
    }

    /// Java `newArrWithInitializers`.
    fn new_arr_with_initializers(&mut self, component_cls: ClassRef, array_initializer: &Node) {
        let Node::ArrayInitializer(ctx) = array_initializer else {
            return;
        };
        let initializers: &[Node] = match ctx.initializers.as_deref() {
            Some(Node::VariableInitializerList(list)) => &list.initializers,
            _ => &[],
        };
        for initializer in initializers {
            initializer.accept(self);
        }
        let reporter = self.reporter_of(array_initializer);
        self.add_instruction(Box::new(NewArrayInstruction::new(
            reporter,
            component_cls,
            initializers.len(),
        )));
    }
}

impl<'a> Visitor for QvmInstructionVisitor<'a> {
    type T = ();

    /// Java `visitImportCls`.
    fn visit_import_cls(&mut self, ctx: &ImportClsContext) {
        if self.failed() {
            return;
        }
        let import_cls_path = ctx
            .var_ids
            .iter()
            .map(|id| id.text())
            .collect::<Vec<_>>()
            .join(".");
        self.import_manager
            .borrow_mut()
            .add_import(super::import_manager::QLImport::import_cls(import_cls_path));
    }

    /// Java `visitImportPack`.
    fn visit_import_pack(&mut self, ctx: &ImportPackContext) {
        if self.failed() {
            return;
        }
        let Some(last) = ctx.var_ids.last() else {
            return;
        };
        let last_text = last.text();
        let is_inner_cls = last_text.chars().next().is_some_and(|c| !c.is_lowercase());
        let import_path = ctx
            .var_ids
            .iter()
            .map(|id| id.text())
            .collect::<Vec<_>>()
            .join(".");
        let import = if is_inner_cls {
            super::import_manager::QLImport::import_inner_cls(import_path)
        } else {
            super::import_manager::QLImport::import_pack(import_path)
        };
        self.import_manager.borrow_mut().add_import(import);
    }

    /// Java `visitBlockStatements`: macros first, then function
    /// definitions, then the remaining statements.
    fn visit_block_statements(&mut self, ctx: &BlockStatementsContext) {
        if self.failed() {
            return;
        }
        let mut is_pre_express = false;
        let non_empty: Vec<&Node> = ctx
            .statements
            .iter()
            .filter(|bs| !matches!(bs, Node::EmptyStatement(_)))
            .collect();

        // First pass: process macro definitions to ensure they are
        // available for functions.
        for child in &non_empty {
            if let Node::MacroStatement(macro_ctx) = child {
                self.visit_macro_statement(macro_ctx);
            }
        }

        // Second pass: process all function definitions to support
        // forward references.
        for child in &non_empty {
            if let Node::FunctionStatement(function_ctx) = child {
                self.visit_function_statement(function_ctx);
            }
        }

        // Third pass: process all other statements.
        for child in &non_empty {
            if self.failed() {
                return;
            }
            if !matches!(child, Node::FunctionStatement(_) | Node::MacroStatement(_)) {
                if is_pre_express {
                    self.add_instruction(Box::new(PopInstruction::new(Rc::new(
                        PureErrReporter::INSTANCE,
                    ))));
                }
                is_pre_express = self.handle_stmt(child);
            }
        }

        if self.context == Context::Block && is_pre_express {
            self.add_instruction(Box::new(ReturnInstruction::new(
                Rc::new(PureErrReporter::INSTANCE),
                ReturnResultType::Continue,
                None,
            )));
        }
    }

    /// Java `visitTraditionalForStatement`.
    fn visit_traditional_for_statement(&mut self, ctx: &TraditionalForStatementContext) {
        if self.failed() {
            return;
        }
        let for_count = self.for_count();
        let for_err_reporter = self.new_reporter_with_token(ctx.for_token.symbol());

        // for init
        let for_init_lambda = self.generate_for_init_lambda(for_count, &ctx.for_init);

        // condition
        let for_condition_lambda = ctx
            .for_condition
            .as_ref()
            .and_then(|cond| self.generate_for_express_lambda(for_count, CONDITION_SUFFIX, cond));
        let condition_error_reporter = ctx
            .for_condition
            .as_ref()
            .and_then(|cond| cond.start_token())
            .map(|t| self.new_reporter_with_token(t))
            .unwrap_or_else(|| Rc::clone(&for_err_reporter));

        // for update
        let for_update_lambda = ctx
            .for_update
            .as_ref()
            .and_then(|upd| self.generate_for_express_lambda(for_count, UPDATE_SUFFIX, upd));

        // for body
        let body_scope_name =
            self.child_scope_name(&format!("{FOR_PREFIX}{for_count}{BODY_SUFFIX}"));
        let (for_body_lambda, _) = self.loop_body_visitor_definition(
            ctx.block_statements.as_deref(),
            body_scope_name,
            vec![],
            Rc::clone(&for_err_reporter),
        );

        let init_size = for_init_lambda.as_ref().map_or(0, |l| l.max_stack_size());
        let condition_size = for_condition_lambda
            .as_ref()
            .map_or(0, |l| l.max_stack_size());
        let update_size = for_update_lambda.as_ref().map_or(0, |l| l.max_stack_size());
        let for_scope_max_stack_size = init_size.max(condition_size).max(update_size);

        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                Rc::clone(&for_err_reporter),
                Some(ctx.for_token.symbol().start_index()),
            )));
        }

        self.add_instruction(Box::new(ForInstruction::new(
            for_err_reporter,
            for_init_lambda.map(|l| -> Rc<dyn QLambdaDefinition> { l }),
            for_condition_lambda.map(|l| -> Rc<dyn QLambdaDefinition> { l }),
            condition_error_reporter,
            for_update_lambda.map(|l| -> Rc<dyn QLambdaDefinition> { l }),
            for_scope_max_stack_size,
            for_body_lambda,
        )));
    }

    /// Java `visitForEachStatement`.
    fn visit_for_each_statement(&mut self, ctx: &ForEachStatementContext) {
        if self.failed() {
            return;
        }
        ctx.expression.accept(self);

        let it_var_cls = ctx
            .decl_type
            .as_ref()
            .map(|decl| self.parse_decl_type(decl))
            .unwrap_or_else(object_cls);

        let for_each_err_reporter = self.new_reporter_with_token(ctx.for_token.symbol());
        let for_count = self.for_count();
        let body_scope_name =
            self.child_scope_name(&format!("{FOR_PREFIX}{for_count}{BODY_SUFFIX}"));
        let (body_definition, _) = self.loop_body_visitor_definition(
            ctx.block_statements.as_deref(),
            body_scope_name,
            vec![Param::new(ctx.var_id.text(), Some(it_var_cls.clone()))],
            Rc::clone(&for_each_err_reporter),
        );

        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                Rc::clone(&for_each_err_reporter),
                Some(ctx.for_token.symbol().start_index()),
            )));
        }

        let target_reporter = self.reporter_of(&ctx.expression);
        self.add_instruction(Box::new(ForEachInstruction::new(
            for_each_err_reporter,
            body_definition,
            it_var_cls,
            target_reporter,
        )));
    }

    /// Java `visitWhileStatement`.
    fn visit_while_statement(&mut self, ctx: &WhileStatementContext) {
        if self.failed() {
            return;
        }
        let while_count = self.while_count();

        let while_condition_scope =
            self.child_scope_name(&format!("{WHILE_PREFIX}{while_count}{CONDITION_SUFFIX}"));
        let scope = self.child_scope(while_condition_scope.clone());
        let condition_sub =
            self.parse_expr_body_with_sub_visitor(&ctx.expression, scope, Context::Block);
        let condition_max_stack = condition_sub.max_stack_size();
        let condition_instructions = condition_sub.take_instructions();
        let condition_lambda: Rc<QLambdaDefinitionInner> = Rc::new(QLambdaDefinitionInner::new(
            while_condition_scope,
            condition_instructions,
            vec![],
            condition_max_stack,
        ));

        let while_err_reporter = self.new_reporter_with_token(ctx.while_token.symbol());
        let body_scope_name =
            self.child_scope_name(&format!("{WHILE_PREFIX}{while_count}{BODY_SUFFIX}"));
        let (while_body_lambda, body_max_stack) = self.loop_body_visitor_definition(
            ctx.block_statements.as_deref(),
            body_scope_name,
            vec![],
            Rc::clone(&while_err_reporter),
        );

        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                Rc::clone(&while_err_reporter),
                Some(ctx.while_token.symbol().start_index()),
            )));
        }

        // Java: max(condition, body) when the body is a
        // QLambdaDefinitionInner, else condition's.
        let while_scope_max_stack_size = match body_max_stack {
            Some(body_max) => condition_max_stack.max(body_max),
            None => condition_max_stack,
        };
        self.add_instruction(Box::new(WhileInstruction::new(
            while_err_reporter,
            condition_lambda,
            while_body_lambda,
            while_scope_max_stack_size,
        )));
    }

    /// Java `visitThrowStatement`.
    fn visit_throw_statement(&mut self, ctx: &ThrowStatementContext) {
        if self.failed() {
            return;
        }
        ctx.expression.accept(self);
        self.add_instruction(Box::new(ThrowInstruction::new(
            self.new_reporter_with_token(ctx.throw_token.symbol()),
        )));
    }

    /// Java `visitReturnStatement`.
    fn visit_return_statement(&mut self, ctx: &ReturnStatementContext) {
        if self.failed() {
            return;
        }
        let error_reporter = self.new_reporter_with_token(ctx.return_token.symbol());
        match &ctx.expression {
            None => {
                self.add_instruction(Box::new(ConstInstruction::new(
                    Rc::clone(&error_reporter),
                    DataValue::NULL_VALUE,
                    None,
                )));
            }
            Some(expression) => expression.accept(self),
        }
        self.add_instruction(Box::new(ReturnInstruction::new(
            error_reporter,
            ReturnResultType::Return,
            Some(ctx.return_token.symbol().start_index()),
        )));
    }

    /// Java `visitFunctionStatement`.
    fn visit_function_statement(&mut self, ctx: &FunctionStatementContext) {
        if self.failed() {
            return;
        }
        let params = ctx
            .params
            .as_ref()
            .map(|p| self.parse_formal_or_inferred_parameter_list(p))
            .unwrap_or_default();
        let function_name = ctx.var_id.text();
        let function_definition =
            self.parse_function_definition(&function_name, ctx.block_statements.as_deref(), params);

        let error_reporter = ctx
            .var_id
            .start_token()
            .map(|t| self.new_reporter_with_token(t))
            .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));

        if self.init_options.is_trace_expression() {
            let trace_key = ctx.var_id.start_token().map(Token::start_index);
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                Rc::clone(&error_reporter),
                trace_key,
            )));
        }

        self.add_instruction(Box::new(DefineFunctionInstruction::new(
            error_reporter,
            function_definition.name().to_string(),
            function_definition,
        )));
    }

    /// Java `visitBreakContinueStatement`.
    fn visit_break_continue_statement(&mut self, ctx: &BreakContinueStatementContext) {
        if self.failed() {
            return;
        }
        let is_break = ctx.is_break();

        if self.init_options.is_trace_expression() {
            let trace_key = ctx.token.symbol().start_index();
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                self.new_reporter_with_token(ctx.token.symbol()),
                Some(trace_key),
            )));
        }

        self.add_instruction(Box::new(BreakContinueInstruction::new(
            self.new_reporter_with_token(ctx.token.symbol()),
            is_break,
        )));
        if is_break {
            if let Some(indices) = &mut self.collect_break_indices {
                indices.push(self.instruction_list.len() - 1);
            }
        }
    }

    /// Java `visitQlIf`.
    fn visit_ql_if(&mut self, ctx: &QlIfContext) {
        if self.failed() {
            return;
        }
        ctx.condition.accept(self);

        let if_count = self.if_count();
        let if_error_reporter = self.new_reporter_with_token(ctx.if_token.symbol());
        let if_scope_name = self.child_scope_name(&format!("{IF_PREFIX}{if_count}"));
        self.add_instruction(Box::new(NewScopeInstruction::new(
            Rc::clone(&if_error_reporter),
            if_scope_name.clone(),
        )));

        // then branch
        let then_scope_name = self.child_scope_name(&format!("{IF_PREFIX}{if_count}{THEN_SUFFIX}"));
        let then_scope = self.child_scope(then_scope_name);
        let then_visitor = self.parse_with_sub_visitor(&ctx.then_body, then_scope, Context::Macro);
        let mut then_instructions = then_visitor.take_instructions();
        if let Node::ThenBody(then_body) = &*ctx.then_body {
            if Self::if_body_fill_const(
                then_body.expression.as_deref(),
                then_body.non_expression_statement.as_deref(),
                then_body.block_statements.as_deref(),
            ) {
                then_instructions.push(Box::new(ConstInstruction::new(
                    Rc::clone(&if_error_reporter),
                    DataValue::NULL_VALUE,
                    None,
                )));
            }
        }
        let then_trace_key = match &*ctx.then_body {
            Node::ThenBody(then_body) if then_body.lbrace.is_some() => {
                ctx.then_body.start_token().map(Token::start_index)
            }
            _ => None,
        };

        // else branch
        let else_scope_name = self.child_scope_name(&format!("{IF_PREFIX}{if_count}{ELSE_SUFFIX}"));
        let else_instructions = match &ctx.else_body {
            None => vec![Box::new(ConstInstruction::new(
                Rc::clone(&if_error_reporter),
                DataValue::NULL_VALUE,
                None,
            )) as Instruction],
            Some(else_body) => {
                let else_scope = self.child_scope(else_scope_name);
                let else_visitor =
                    self.parse_with_sub_visitor(else_body, else_scope, Context::Macro);
                let mut instructions = else_visitor.take_instructions();
                if let Node::ElseBody(else_ctx) = &**else_body {
                    if else_ctx.ql_if.is_none()
                        && Self::if_body_fill_const(
                            else_ctx.expression.as_deref(),
                            else_ctx.non_expression_statement.as_deref(),
                            else_ctx.block_statements.as_deref(),
                        )
                    {
                        instructions.push(Box::new(ConstInstruction::new(
                            Rc::clone(&if_error_reporter),
                            DataValue::NULL_VALUE,
                            None,
                        )));
                    }
                }
                instructions
            }
        };
        let else_trace_key = match ctx.else_body.as_deref() {
            Some(Node::ElseBody(else_ctx)) if else_ctx.lbrace.is_some() => ctx
                .else_body
                .as_ref()
                .and_then(|b| b.start_token())
                .map(Token::start_index),
            _ => None,
        };
        let trace_key = ctx.if_token.symbol().start_index();
        self.if_else_instructions(
            Rc::clone(&if_error_reporter),
            then_instructions,
            then_trace_key,
            else_instructions,
            else_trace_key,
            Some(trace_key),
        );

        self.add_instruction(Box::new(CloseScopeInstruction::new(
            if_error_reporter,
            if_scope_name,
        )));
    }

    /// Java `visitMacroStatement`.
    fn visit_macro_statement(&mut self, ctx: &MacroStatementContext) {
        if self.failed() {
            return;
        }
        let macro_id = ctx.var_id.text();
        let macro_block = ctx.block_statements.as_deref();
        let last_stmt_express = Self::macro_last_stmt_is_expression(macro_block);
        let macro_instructions = self.get_macro_instructions(macro_block);
        self.generator_scope.define_macro(
            macro_id,
            MacroDefine::new(macro_instructions, last_stmt_express),
        );

        if self.init_options.is_trace_expression() {
            let trace_key = ctx.var_id.start_token().map(Token::start_index);
            let reporter = ctx
                .var_id
                .start_token()
                .map(|t| self.new_reporter_with_token(t))
                .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));
            self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                reporter, trace_key,
            )));
        }
    }

    /// Java `visitLocalVariableDeclaration`.
    fn visit_local_variable_declaration(&mut self, ctx: &LocalVariableDeclarationContext) {
        if self.failed() {
            return;
        }
        if self.init_options.is_trace_expression() {
            let trace_key = ctx.decl_type.start_token().map(Token::start_index);
            let reporter = self.reporter_of(&ctx.decl_type);
            self.add_instruction(Box::new(TraceEvaluatedInstruction::new(
                reporter, trace_key,
            )));
        }

        let decl_cls = self.parse_decl_type(&ctx.decl_type);
        let Node::VariableDeclaratorList(list) = &*ctx.variable_declarator_list else {
            return;
        };
        for variable_declarator in &list.variables {
            if self.failed() {
                return;
            }
            let Node::VariableDeclarator(declarator) = variable_declarator else {
                continue;
            };
            match &declarator.initializer {
                None => {
                    // Java `visitLocalVariableDeclaration` 无论声明类型为何，都
                    // 为缺省初始化器压入 `null`，再由 DefineLocal 执行转换。
                    let reporter = variable_declarator
                        .stop_token()
                        .map(|t| self.new_reporter_with_token(t))
                        .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));
                    self.add_instruction(Box::new(ConstInstruction::new(
                        reporter,
                        DataValue::Null,
                        None,
                    )));
                }
                Some(initializer) => self.parse_initializer(initializer, &decl_cls),
            }
            // Java `variableDeclaratorId().getStart()`: the variable id
            // token (not any trailing `[]` dims).
            let id_token = declarator.id.start_token().cloned();
            let reporter = id_token
                .as_ref()
                .map(|t| self.new_reporter_with_token(t))
                .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));
            let variable_name = id_token
                .as_ref()
                .map(|t| t.text().to_string())
                .unwrap_or_default();
            self.add_instruction(Box::new(DefineLocalInstruction::new(
                reporter,
                variable_name,
                Some(decl_cls.clone()),
            )));
        }
    }

    /// Java `visitTryCatchExpr`.
    fn visit_try_catch_expr(&mut self, ctx: &TryCatchExprContext) {
        if self.failed() {
            return;
        }
        let Some(block_statements) = &ctx.block_statements else {
            self.add_instruction(Box::new(ConstInstruction::new(
                self.new_reporter_with_token(ctx.try_token.symbol()),
                DataValue::NULL_VALUE,
                Some(ctx.try_token.symbol().start_index()),
            )));
            return;
        };

        let try_count = self.try_count();
        let try_scope_name = self.child_scope_name(&format!("{TRY_PREFIX}{try_count}"));
        let scope = self.child_scope(try_scope_name.clone());
        let body_sub = self.parse_with_sub_visitor(block_statements, scope, Context::Block);
        let body_max_stack = body_sub.max_stack_size();
        let body_instructions = body_sub.take_instructions();
        let body_lambda_definition: Rc<dyn QLambdaDefinition> = Rc::new(
            QLambdaDefinitionInner::new(try_scope_name, body_instructions, vec![], body_max_stack),
        );
        let exception_table = self.parse_exception_table(try_count, ctx.try_catches.as_deref());
        let final_body_definition =
            self.parse_final_body_definition(try_count, ctx.try_finally.as_deref());

        // Java 行为:try-catch 始终作为表达式。控制信号(Return/Break/
        // Continue(null))由 should_exit_try_catch 判断是否透传,
        // 不依赖 is_expression_form。catch body 含控制信号时,
        // 仍为表达式(is_expression_form=true),由 execute 内部
        // should_exit_try_catch 处理传播。
        self.add_instruction(Box::new(
            TryCatchInstruction::new(
                self.new_reporter_with_token(ctx.try_token.symbol()),
                body_lambda_definition,
                exception_table,
                final_body_definition,
            )
            .with_expression_form(true),
        ));
    }

    /// Java `visitExpression` (assignment).
    fn visit_expression(&mut self, ctx: &ExpressionContext) {
        if self.failed() {
            return;
        }
        if let Some(ternary) = &ctx.ternary {
            ternary.accept(self);
            return;
        }

        if let Some(left) = &ctx.left {
            left.accept(self);
        }
        if let Some(expression) = &ctx.expression {
            expression.accept(self);
        }

        let Some(assign_operator) = &ctx.assign_operator else {
            return;
        };
        let operator_id = assign_operator.text();
        let Some(binary_operator) = self.operator_factory.get_binary_operator(&operator_id) else {
            return;
        };
        let reporter = self.reporter_of(assign_operator);
        let trace_key = assign_operator.start_token().map(Token::start_index);
        self.add_instruction(Box::new(OperatorInstruction::new(
            reporter,
            binary_operator,
            trace_key,
        )));
    }

    /// Java `visitTernaryExpr`.
    fn visit_ternary_expr(&mut self, ctx: &TernaryExprContext) {
        if self.failed() {
            return;
        }
        ctx.condition.accept(self);

        if let Some(question) = &ctx.question {
            let then_visitor = self.parse_with_sub_visitor(
                ctx.then_expr.as_ref().expect("ternary then expr"),
                Rc::clone(&self.generator_scope),
                Context::Macro,
            );
            let else_visitor = self.parse_with_sub_visitor(
                ctx.else_expr.as_ref().expect("ternary else expr"),
                Rc::clone(&self.generator_scope),
                Context::Macro,
            );
            let then_instructions = then_visitor.take_instructions();
            let else_instructions = else_visitor.take_instructions();
            self.if_else_instructions(
                self.new_reporter_with_token(question.symbol()),
                then_instructions,
                None,
                else_instructions,
                None,
                Some(question.symbol().start_index()),
            );
        }
    }

    /// Java `visitBlockExpr`.
    fn visit_block_expr(&mut self, ctx: &BlockExprContext) {
        if self.failed() {
            return;
        }
        let block_err_reporter = self.new_reporter_with_token(ctx.lbrace.symbol());
        let Some(block_statements) = &ctx.block_statements else {
            self.add_instruction(Box::new(ConstInstruction::new(
                block_err_reporter,
                DataValue::NULL_VALUE,
                None,
            )));
            return;
        };

        let block_scope_name = self.block_scope_name();
        let scope = self.child_scope(block_scope_name.clone());
        let block_sub_visitor =
            self.parse_with_sub_visitor(block_statements, scope, Context::Macro);
        let block_instructions = block_sub_visitor.take_instructions();

        self.add_instruction(Box::new(NewScopeInstruction::new(
            Rc::clone(&block_err_reporter),
            block_scope_name.clone(),
        )));
        for instruction in block_instructions {
            self.pure_add_instruction(instruction);
        }
        self.add_instruction(Box::new(CloseScopeInstruction::new(
            Rc::clone(&block_err_reporter),
            block_scope_name,
        )));
        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                block_err_reporter,
                Some(ctx.lbrace.symbol().start_index()),
            )));
        }
    }

    /// Java `visitCastExpr`.
    fn visit_cast_expr(&mut self, ctx: &CastExprContext) {
        if self.failed() {
            return;
        }
        let cast_cls = self.parse_decl_type(&ctx.decl_type);
        let error_reporter = self.reporter_of(&ctx.decl_type);
        self.add_instruction(Box::new(ConstInstruction::new(
            Rc::clone(&error_reporter),
            MetaClass::new(cast_cls).into_data_value(),
            None,
        )));
        ctx.primary.accept(self);
        self.add_instruction(Box::new(CastInstruction::new(error_reporter)));
    }

    /// Java `visitSwitchExpr`.
    fn visit_switch_expr(&mut self, ctx: &SwitchExprContext) {
        if self.failed() {
            return;
        }
        let mut groups = switch_groups(ctx);
        let Some(first_group) = groups.next() else {
            // Empty switch, push null as result
            self.add_instruction(Box::new(ConstInstruction::new(
                self.new_reporter_with_token(ctx.switch_token.symbol()),
                DataValue::NULL_VALUE,
                None,
            )));
            return;
        };

        // Check the type of first group to determine switch style
        let is_statement_style = matches!(first_group, Node::SwitchStatementGroup(_));
        let is_expression_style = matches!(first_group, Node::SwitchExprGroup(_));

        // Validate that all groups have the same type
        for group in groups {
            let current_is_statement = matches!(group, Node::SwitchStatementGroup(_));
            let current_is_expression = matches!(group, Node::SwitchExprGroup(_));
            if (is_statement_style && !current_is_statement)
                || (is_expression_style && !current_is_expression)
            {
                let error_token = group.start_token().cloned();
                if let Some(error_token) = error_token {
                    self.report_parse_err(
                        &error_token,
                        "SWITCH_STYLE_MISMATCH",
                        "Cannot mix traditional switch syntax (case X:) with switch expression syntax (case X ->) in the same switch block",
                    );
                }
                return;
            }
        }

        if is_statement_style {
            self.visit_switch_statement(ctx);
        } else if is_expression_style {
            self.visit_switch_expression(ctx);
        }
    }

    /// Java `visitListExpr`.
    fn visit_list_expr(&mut self, ctx: &ListExprContext) {
        if self.failed() {
            return;
        }
        let reporter = self.new_reporter_with_token(ctx.lbrack.symbol());
        self.visit_list_expr_inner(ctx.list_items.as_deref(), reporter);
    }

    /// Java `visitMapExpr`.
    fn visit_map_expr(&mut self, ctx: &MapExprContext) {
        if self.failed() {
            return;
        }
        let Node::MapEntries(entries_ctx) = &*ctx.map_entries else {
            return;
        };
        let mut keys: Vec<String> = Vec::with_capacity(entries_ctx.entries.len());
        let mut cls: Option<ClassRef> = None;
        for map_entry in &entries_ctx.entries {
            if self.failed() {
                return;
            }
            let Node::MapEntry(entry_ctx) = map_entry else {
                continue;
            };
            match &*entry_ctx.map_value {
                Node::EValue(e_value) => {
                    keys.push(self.parse_map_key(&entry_ctx.map_key));
                    e_value.expression.accept(self);
                }
                Node::ClsValue(cls_value) => {
                    let cls_text = cls_value.quote.text();
                    let cls_name = strip_quotes(cls_text);
                    let may_be_cls = self.import_manager.borrow().load_qualified(&cls_name);
                    match may_be_cls {
                        None => {
                            let key_text = entry_ctx.map_key.text();
                            keys.push(strip_quotes(&key_text));
                            let reporter = self.new_reporter_with_token(cls_value.quote.symbol());
                            self.add_instruction(Box::new(ConstInstruction::new(
                                reporter,
                                DataValue::Str(QLStringUtils::parse_string_escape(cls_text)),
                                None,
                            )));
                            // @class override
                            cls = None;
                        }
                        Some(loaded) => {
                            cls = Some(ClassRef::from_name(&loaded));
                        }
                    }
                }
                _ => {}
            }
        }
        let reporter = self.new_reporter_with_token(ctx.lbrace.symbol());
        match cls {
            None => {
                self.add_instruction(Box::new(NewMapInstruction::new(reporter, keys)));
            }
            Some(cls) => {
                self.add_instruction(Box::new(NewFilledInstanceInstruction::new(
                    reporter, cls, keys,
                )));
            }
        }
    }

    /// Java `visitNewObjExpr`.
    fn visit_new_obj_expr(&mut self, ctx: &NewObjExprContext) {
        if self.failed() {
            return;
        }
        let new_cls = self.parse_cls_ids(&ctx.var_ids);
        if let Some(argument_list) = &ctx.argument_list {
            argument_list.accept(self);
        }
        let arg_num = ctx.argument_list.as_deref().map_or(0, argument_count);
        self.add_instruction(Box::new(NewInstanceInstruction::new(
            self.new_reporter_with_token(ctx.new_token.symbol()),
            new_cls,
            arg_num,
        )));
    }

    /// Java `visitNewEmptyArrExpr`.
    fn visit_new_empty_arr_expr(&mut self, ctx: &NewEmptyArrExprContext) {
        if self.failed() {
            return;
        }
        ctx.dim_exprs.accept(self);
        let dims = match &*ctx.dim_exprs {
            Node::DimExprs(dim_exprs) => dim_exprs.expressions.len(),
            _ => 0,
        };
        let arr_cls = self.parse_decl_type_no_arr(&ctx.decl_type_no_arr);
        self.add_instruction(Box::new(MultiNewArrayInstruction::new(
            self.new_reporter_with_token(ctx.new_token.symbol()),
            arr_cls,
            dims,
        )));
    }

    /// Java `visitNewInitArrExpr`.
    fn visit_new_init_arr_expr(&mut self, ctx: &NewInitArrExprContext) {
        if self.failed() {
            return;
        }
        let cls = self.parse_decl_type_no_arr(&ctx.decl_type_no_arr);
        // Java `embedClsInDims(cls, dims - 1)`: array-of-component class.
        let dimensions = dims_dim_count(&ctx.dims);
        self.new_arr_with_initializers(
            wrap_in_array(cls, dimensions.saturating_sub(1)),
            &ctx.array_initializer,
        );
    }

    /// Java `visitLambdaExpr`.
    fn visit_lambda_expr(&mut self, ctx: &LambdaExprContext) {
        if self.failed() {
            return;
        }
        let lambda_params = self.parse_lambda_params(&ctx.lambda_parameters);
        let lambda_scope_name = self.lambda_scope_name();

        let arrow_error_reporter = self.new_reporter_with_token(ctx.arrow.symbol());
        let sub_visitor = if let Some(expression) = &ctx.expression {
            let scope = self.child_scope(lambda_scope_name.clone());
            Some(self.parse_expr_body_with_sub_visitor(expression, scope, Context::Block))
        } else {
            ctx.block_statements.as_ref().map(|block_statements| {
                let scope = self.child_scope(lambda_scope_name.clone());
                self.parse_with_sub_visitor(block_statements, scope, Context::Block)
            })
        };

        match sub_visitor {
            None => {
                self.add_instruction(Box::new(LoadLambdaInstruction::new(
                    arrow_error_reporter,
                    Rc::new(QLambdaDefinitionEmpty::INSTANCE),
                )));
            }
            Some(sub_visitor) => {
                let max_stack_size = sub_visitor.max_stack_size();
                let instructions = sub_visitor.take_instructions();
                let lambda_definition = QLambdaDefinitionInner::new(
                    lambda_scope_name,
                    instructions,
                    lambda_params,
                    max_stack_size,
                );
                self.add_instruction(Box::new(LoadLambdaInstruction::new(
                    arrow_error_reporter,
                    Rc::new(lambda_definition),
                )));
            }
        }
    }

    /// Java `visitVarIdExpr`.
    fn visit_var_id_expr(&mut self, ctx: &VarIdExprContext) {
        if self.failed() {
            return;
        }
        let reporter = self.reporter_of(&ctx.var_id);
        let trace_key = ctx.var_id.start_token().map(Token::start_index);
        self.add_instruction(Box::new(LoadInstruction::new(
            reporter,
            ctx.var_id.text(),
            trace_key,
        )));
    }

    /// Java `visitMethodInvoke` / `visitOptionalMethodInvoke` /
    /// `visitSpreadMethodInvoke` (merged via `ChainKind`).
    fn visit_method_invoke(&mut self, ctx: &MethodInvokeContext) {
        if self.failed() {
            return;
        }
        match ctx.chain {
            ChainKind::Plain => {
                self.visit_method_invoke_inner(ctx.argument_list.as_deref(), &ctx.var_id, false)
            }
            ChainKind::Optional => {
                self.visit_method_invoke_inner(ctx.argument_list.as_deref(), &ctx.var_id, true)
            }
            ChainKind::Spread => {
                if let Some(argument_list) = &ctx.argument_list {
                    argument_list.accept(self);
                }
                let arg_num = ctx.argument_list.as_deref().map_or(0, argument_count);
                let reporter = self.reporter_of(&ctx.var_id);
                self.add_instruction(Box::new(SpreadMethodInvokeInstruction::new(
                    reporter,
                    ctx.var_id.text(),
                    arg_num,
                )));
            }
        }
    }

    /// Java `visitFieldAccess` / `visitOptionalFieldAccess` /
    /// `visitSpreadFieldAccess` (merged via `ChainKind`).
    fn visit_field_access(&mut self, ctx: &FieldAccessContext) {
        if self.failed() {
            return;
        }
        let field_name = Self::parse_field_id(&ctx.field_id);
        let reporter = ctx
            .field_id
            .stop_token()
            .map(|t| self.new_reporter_with_token(t))
            .unwrap_or_else(|| self.new_reporter_with_token(ctx.dot.symbol()));
        match ctx.chain {
            ChainKind::Plain => self.add_instruction(Box::new(GetFieldInstruction::new(
                reporter, field_name, false,
            ))),
            ChainKind::Optional => self.add_instruction(Box::new(GetFieldInstruction::new(
                reporter, field_name, true,
            ))),
            ChainKind::Spread => self.add_instruction(Box::new(SpreadGetFieldInstruction::new(
                reporter, field_name,
            ))),
        }
    }

    /// Java `visitMethodAccess` (`Cls::method`).
    fn visit_method_access(&mut self, ctx: &MethodAccessContext) {
        if self.failed() {
            return;
        }
        self.add_instruction(Box::new(GetMethodInstruction::new(
            self.new_reporter_with_token(ctx.dcolon.symbol()),
            ctx.var_id.text(),
        )));
    }

    /// Java `visitIndexExpr`.
    fn visit_index_expr(&mut self, ctx: &IndexExprContext) {
        if self.failed() {
            return;
        }
        let Some(index_value_expr) = &ctx.index_value_expr else {
            // Java `ctx.getStop()` 指向 `]`，不能用起始 `[`，否则列号少 1。
            let stop = ctx.rbrack.symbol().clone();
            self.report_parse_err(
                &stop,
                error_codes::MISSING_INDEX,
                error_codes::error_msg(error_codes::MISSING_INDEX),
            );
            return;
        };
        let error_reporter = self.new_reporter_with_token(ctx.lbrack.symbol());
        match &**index_value_expr {
            Node::SingleIndex(single) => {
                single.expression.accept(self);
                self.add_instruction(Box::new(IndexInstruction::new(error_reporter)));
            }
            Node::SliceIndex(slice) => match (&slice.start, &slice.end) {
                (None, None) => self.add_instruction(Box::new(SliceInstruction::new(
                    error_reporter,
                    SliceMode::Copy,
                ))),
                (None, Some(end)) => {
                    end.accept(self);
                    self.add_instruction(Box::new(SliceInstruction::new(
                        error_reporter,
                        SliceMode::Left,
                    )));
                }
                (Some(start), None) => {
                    start.accept(self);
                    self.add_instruction(Box::new(SliceInstruction::new(
                        error_reporter,
                        SliceMode::Right,
                    )));
                }
                (Some(start), Some(end)) => {
                    start.accept(self);
                    end.accept(self);
                    self.add_instruction(Box::new(SliceInstruction::new(
                        error_reporter,
                        SliceMode::Both,
                    )));
                }
            },
            _ => {}
        }
    }

    /// Java `visitCustomPath`.
    fn visit_custom_path(&mut self, ctx: &CustomPathContext) {
        if self.failed() {
            return;
        }
        let error_reporter = self.reporter_of(&ctx.op_id);
        self.add_instruction(Box::new(ConstInstruction::new(
            Rc::clone(&error_reporter),
            DataValue::Str(ctx.path_text.clone()),
            None,
        )));

        let operator_id = ctx.op_id.text();
        let Some(binary_operator) = self.operator_factory.get_binary_operator(&operator_id) else {
            return;
        };
        let trace_key = ctx.op_id.start_token().map(Token::start_index);
        self.add_instruction(Box::new(OperatorInstruction::new(
            error_reporter,
            binary_operator,
            trace_key,
        )));
    }

    /// Java `visitLeftAsso` (binary chain step, with `&&`/`||`
    /// short-circuit).
    fn visit_left_asso(&mut self, ctx: &LeftAssoContext) {
        if self.failed() {
            return;
        }
        let operator_id = ctx.binaryop.text();
        let op_err_reporter = self.reporter_of(&ctx.binaryop);
        let trace_key = ctx.binaryop.start_token().map(Token::start_index);
        // short circuit operator
        if operator_id == "&&" {
            self.jump_right_if_expect(false, op_err_reporter, &ctx.right, &operator_id, trace_key);
        } else if operator_id == "||" {
            self.jump_right_if_expect(true, op_err_reporter, &ctx.right, &operator_id, trace_key);
        } else {
            ctx.right.accept(self);
            let Some(binary_operator) = self.operator_factory.get_binary_operator(&operator_id)
            else {
                return;
            };
            self.add_instruction(Box::new(OperatorInstruction::new(
                op_err_reporter,
                binary_operator,
                trace_key,
            )));
        }
    }

    /// Java `visitLeftHandSide`.
    fn visit_left_hand_side(&mut self, ctx: &LeftHandSideContext) {
        if self.failed() {
            return;
        }
        let tail_part_start = self.parse_id_head_part(
            &ctx.var_id,
            ctx.lparen.is_some(),
            ctx.argument_list.as_deref(),
            &ctx.path_parts,
        );
        for path_part in &ctx.path_parts[tail_part_start..] {
            if self.failed() {
                return;
            }
            path_part.accept(self);
        }
    }

    /// Java `visitPrimary`.
    fn visit_primary(&mut self, ctx: &PrimaryContext) {
        if self.failed() {
            return;
        }
        if let Some(non_pathable) = &ctx.non_pathable {
            non_pathable.accept(self);
            return;
        }
        let Some(pathable) = &ctx.pathable else {
            return;
        };

        // path: head part
        let tail_part_start = self.parse_path_head_part(pathable, &ctx.path_parts);

        // tail part
        for path_part in &ctx.path_parts[tail_part_start..] {
            if self.failed() {
                return;
            }
            path_part.accept(self);
        }

        if let Some(suffix_express) = &ctx.suffix {
            let suffix_operator = suffix_express.text();
            let suffix_unary_operator = self
                .operator_factory
                .get_suffix_unary_operator(&suffix_operator)
                .expect("suffix unary operator must exist");
            let reporter = self.reporter_of(suffix_express);
            let trace_key = suffix_express.start_token().map(Token::start_index);
            self.add_instruction(Box::new(UnaryInstruction::new(
                reporter,
                suffix_unary_operator,
                trace_key,
            )));
        }

        if let Some(prefix_express) = &ctx.prefix {
            let prefix_operator = prefix_express.text();
            let prefix_unary_operator = self
                .operator_factory
                .get_prefix_unary_operator(&prefix_operator)
                .expect("prefix unary operator must exist");
            let reporter = self.reporter_of(prefix_express);
            let trace_key = prefix_express.start_token().map(Token::start_index);
            self.add_instruction(Box::new(UnaryInstruction::new(
                reporter,
                prefix_unary_operator,
                trace_key,
            )));
        }
    }

    /// Java `visitTypeExpr`.
    fn visit_type_expr(&mut self, ctx: &TypeExprContext) {
        if self.failed() {
            return;
        }
        let cls = self.parse_decl_type(&ctx.decl_type);
        let reporter = self.reporter_of(&ctx.decl_type);
        self.add_instruction(Box::new(ConstInstruction::new(
            reporter,
            MetaClass::new(cls).into_data_value(),
            None,
        )));
    }

    /// Java `visitContextSelectExpr`.
    fn visit_context_select_expr(&mut self, ctx: &ContextSelectExprContext) {
        if self.failed() {
            return;
        }
        let variable_name = ctx.selector_variable.text().trim().to_string();
        let reporter = self.new_reporter_with_token(ctx.selector_start.symbol());
        let trace_key = Some(ctx.selector_start.symbol().start_index());
        self.add_instruction(Box::new(LoadInstruction::new(
            reporter,
            variable_name,
            trace_key,
        )));
    }

    /// Java `visitLiteral`.
    fn visit_literal(&mut self, ctx: &LiteralContext) {
        if self.failed() {
            return;
        }
        if let Some(terminal) = &ctx.token {
            let symbol = terminal.symbol();
            let token_type = symbol.token_type();
            let text = symbol.text();
            let reporter = self.new_reporter_with_token(symbol);
            let trace_key = Some(symbol.start_index());
            let value: Option<DataValue> = match token_type as u16 {
                token::INTEGER_LITERAL => parse_integer_literal(&remove_char(text, '_')),
                token::FLOATING_POINT_LITERAL => parse_floating_literal(&remove_char(text, '_')),
                token::INTEGER_OR_FLOATING_LITERAL => {
                    let cleaned = remove_char(text, '_');
                    if cleaned.contains('.') {
                        parse_floating_literal(&cleaned)
                    } else {
                        parse_integer_literal(&cleaned)
                    }
                }
                token::QUOTE_STRING_LITERAL => {
                    Some(DataValue::Str(QLStringUtils::parse_string_escape(text)))
                }
                token::NULL => Some(DataValue::NULL_VALUE),
                _ => None,
            };
            match value {
                Some(value) => {
                    self.add_instruction(Box::new(ConstInstruction::new(
                        reporter, value, trace_key,
                    )));
                }
                None if matches!(
                    token_type as u16,
                    token::INTEGER_LITERAL
                        | token::FLOATING_POINT_LITERAL
                        | token::INTEGER_OR_FLOATING_LITERAL
                ) =>
                {
                    let symbol = symbol.clone();
                    self.report_parse_err(
                        &symbol,
                        error_codes::INVALID_NUMBER,
                        error_codes::error_msg(error_codes::INVALID_NUMBER),
                    );
                }
                _ => {}
            }
            return;
        }
        if let Some(boolen) = &ctx.boolen {
            if let Node::BoolenLiteral(boolen_ctx) = &**boolen {
                let symbol = boolen_ctx.token.symbol();
                let bool_value = symbol.text() == "true";
                let reporter = self.new_reporter_with_token(symbol);
                let trace_key = Some(symbol.start_index());
                self.add_instruction(Box::new(ConstInstruction::new(
                    reporter,
                    DataValue::Bool(bool_value),
                    trace_key,
                )));
            }
            return;
        }
        if let Some(double_quote) = &ctx.double_quote_string {
            double_quote.accept(self);
        }
    }

    /// Java `visitDoubleQuoteStringLiteral` (string interpolation).
    fn visit_double_quote_string_literal(&mut self, ctx: &DoubleQuoteStringLiteralContext) {
        if self.failed() {
            return;
        }
        if self.init_options.interpolation_mode() == InterpolationMode::Disable {
            match &ctx.static_characters {
                None => {
                    let reporter = self.new_reporter_with_token(ctx.open_quote.symbol());
                    self.add_instruction(Box::new(ConstInstruction::new(
                        reporter,
                        DataValue::Str(String::new()),
                        None,
                    )));
                }
                Some(characters) => {
                    let text = characters.text();
                    let reporter = self.new_reporter_with_token(ctx.open_quote.symbol());
                    self.add_instruction(Box::new(ConstInstruction::new(
                        reporter,
                        DataValue::Str(QLStringUtils::parse_string_escape_start_end(
                            text,
                            0,
                            text.chars().count(),
                        )),
                        None,
                    )));
                }
            }
            return;
        }
        // Children between the quotes (Java iterates children 1..count-1).
        let mut part_count = 0usize;
        if let Some(characters) = &ctx.static_characters {
            let text = characters.text().to_string();
            let reporter = self.new_reporter_with_token(characters.symbol());
            let trace_key = Some(ctx.open_quote.symbol().start_index());
            self.add_instruction(Box::new(ConstInstruction::new(
                reporter,
                DataValue::Str(QLStringUtils::parse_string_escape_start_end(
                    &text,
                    0,
                    text.chars().count(),
                )),
                trace_key,
            )));
            part_count += 1;
        } else {
            for part in &ctx.parts {
                if self.failed() {
                    return;
                }
                match part {
                    DyStrPart::Expr(node) => {
                        if let Node::StringExpression(string_expression) = &**node {
                            if let Some(expression) = &string_expression.expression {
                                // SCRIPT
                                expression.accept(self);
                            } else if let Some(var_terminal) = &string_expression.selector_variable
                            {
                                // VARIABLE
                                let var_name = var_terminal.text().trim().to_string();
                                let reporter = self.new_reporter_with_token(var_terminal.symbol());
                                self.add_instruction(Box::new(LoadInstruction::new(
                                    reporter, var_name, None,
                                )));
                            }
                        }
                    }
                    DyStrPart::Text(terminal) => {
                        let origin_str = terminal.text();
                        let reporter = self.new_reporter_with_token(terminal.symbol());
                        let trace_key = Some(ctx.open_quote.symbol().start_index());
                        self.add_instruction(Box::new(ConstInstruction::new(
                            reporter,
                            DataValue::Str(QLStringUtils::parse_string_escape_start_end(
                                origin_str,
                                0,
                                origin_str.chars().count(),
                            )),
                            trace_key,
                        )));
                    }
                }
                part_count += 1;
            }
        }
        let reporter = self.new_reporter_with_token(ctx.open_quote.symbol());
        self.add_instruction(Box::new(StringJoinInstruction::new(reporter, part_count)));
    }
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

impl<'a> QvmInstructionVisitor<'a> {
    /// Java `visitMethodInvokeInner`.
    fn visit_method_invoke_inner(
        &mut self,
        argument_list: Option<&Node>,
        method_name: &Node,
        optional: bool,
    ) {
        if let Some(argument_list) = argument_list {
            argument_list.accept(self);
        }
        let arg_num = argument_list.map_or(0, argument_count);
        let reporter = self.reporter_of(method_name);
        self.add_call_instruction(Box::new(MethodInvokeInstruction::new(
            reporter,
            method_name.text(),
            arg_num,
            optional,
        )));
    }

    /// Java `parseFieldId`.
    fn parse_field_id(field_id: &Node) -> String {
        if let Node::FieldId(ctx) = field_id {
            if let Some(quote) = &ctx.quote {
                return QLStringUtils::parse_string_escape(quote.text());
            }
        }
        field_id
            .start_token()
            .map(Token::text)
            .unwrap_or_default()
            .to_string()
    }

    /// Java `parseDimParts`: count leading empty `[]` index parts.
    fn parse_dim_parts(&self, start: usize, path_parts: &[Node]) -> usize {
        let mut i = start;
        while i < path_parts.len() && is_empty_index(&path_parts[i]) {
            i += 1;
        }
        i - start
    }

    /// Java `parsePathHeadPart`.
    fn parse_path_head_part(&mut self, pathable: &Node, path_parts: &[Node]) -> usize {
        match pathable {
            Node::TypeExpr(_) => {
                let text = pathable.start_token().map(Token::text).unwrap_or_default();
                let cls = Self::built_in_cls(text).unwrap_or_else(object_cls);
                let dim_part_num = self.parse_dim_parts(0, path_parts);
                let cls = wrap_in_array(cls, dim_part_num);
                let reporter = self.reporter_of(pathable);
                self.add_instruction(Box::new(ConstInstruction::new(
                    reporter,
                    MetaClass::new(cls).into_data_value(),
                    None,
                )));
                dim_part_num
            }
            Node::VarIdExpr(id_context) => self.parse_id_head_part(
                &id_context.var_id,
                id_context.lparen.is_some(),
                id_context.argument_list.as_deref(),
                path_parts,
            ),
            _ => {
                pathable.accept(self);
                0
            }
        }
    }

    /// Java `parseIdHeadPart`.
    fn parse_id_head_part(
        &mut self,
        id_context: &Node,
        function_call: bool,
        argument_list: Option<&Node>,
        path_parts: &[Node],
    ) -> usize {
        if function_call {
            self.visit_call_function(id_context, argument_list);
            return 0;
        }
        let mut head_part_ids = vec![id_context.text()];
        for path_part in path_parts {
            match path_part {
                Node::FieldAccess(field_access) if field_access.chain == ChainKind::Plain => {
                    head_part_ids.push(Self::parse_field_id(&field_access.field_id));
                }
                _ => break,
            }
        }
        let result = self
            .import_manager
            .borrow()
            .load_part_qualified(&head_part_ids);
        match result.cls() {
            Some(cls) => {
                let cls = ClassRef::from_name(cls);
                let rest_index = result.rest_index() as i32 - 1;
                let report_token = if rest_index == 0 {
                    id_context.start_token()
                } else {
                    path_parts
                        .get((rest_index - 1).max(0) as usize)
                        .and_then(|p| p.stop_token())
                };
                let dim_part_num = self.parse_dim_parts(rest_index.max(0) as usize, path_parts);
                let cls = wrap_in_array(cls, dim_part_num);
                let reporter = report_token
                    .map(|t| self.new_reporter_with_token(t))
                    .unwrap_or_else(|| Rc::new(PureErrReporter::INSTANCE));
                self.add_instruction(Box::new(ConstInstruction::new(
                    reporter,
                    MetaClass::new(cls).into_data_value(),
                    None,
                )));
                rest_index.max(0) as usize + dim_part_num
            }
            None => {
                let reporter = self.reporter_of(id_context);
                let trace_key = id_context.start_token().map(Token::start_index);
                self.add_instruction(Box::new(LoadInstruction::new(
                    reporter,
                    id_context.text(),
                    trace_key,
                )));
                0
            }
        }
    }

    /// Java `visitCallFunction`.
    fn visit_call_function(&mut self, function_name_context: &Node, argument_list: Option<&Node>) {
        let function_name = function_name_context.text();
        let compile_time_function = self.compile_time_functions.get(&function_name).cloned();
        if let Some(compile_time_function) = compile_time_function {
            let function_token = function_name_context
                .start_token()
                .cloned()
                .unwrap_or_else(|| Token::new(0, "", 0, 0, 1, 0));
            let reporter = self.reporter_of(function_name_context);
            let arguments: Vec<&Node> = argument_list.map(argument_expressions).unwrap_or_default();
            let operator_factory = self.operator_factory;
            let mut code_generator = VisitorCodeGenerator {
                visitor: self,
                function_name: function_name.clone(),
                function_token,
                reporter,
            };
            compile_time_function.create_function_instruction(
                &function_name,
                &arguments,
                operator_factory,
                &mut code_generator,
            );
            return;
        }

        if let Some(arg_list) = argument_list {
            let lazy_flags: Option<Vec<bool>> = self
                .user_define_functions
                .get(&function_name)
                .and_then(|f| f.as_lazy_arg())
                .map(|lazy| {
                    (0..argument_count(arg_list))
                        .map(|i| lazy.is_lazy_arg(i))
                        .collect()
                });
            match lazy_flags {
                Some(flags) => {
                    let lazy_function_count = self.lazy_function_count();
                    for (i, expr) in argument_expressions(arg_list).iter().enumerate() {
                        if !flags[i] {
                            expr.accept(self);
                            continue;
                        }
                        let scope_name = self.child_scope_name(&format!(
                            "{LAZY_FUNCTION_PREFIX}{lazy_function_count}_{function_name}{i}"
                        ));
                        let scope = self.child_scope(scope_name.clone());
                        let lazy_visitor =
                            self.parse_expr_body_with_sub_visitor(expr, scope, Context::Block);
                        let max_stack_size = lazy_visitor.max_stack_size();
                        let instructions = lazy_visitor.take_instructions();
                        let lazy_lambda = QLambdaDefinitionInner::new(
                            scope_name,
                            instructions,
                            vec![],
                            max_stack_size,
                        );
                        let reporter = self.reporter_of(expr);
                        self.add_instruction(Box::new(LoadLambdaInstruction::new(
                            reporter,
                            Rc::new(lazy_lambda),
                        )));
                    }
                }
                None => arg_list.accept(self),
            }
        }
        let arg_size = argument_list.map_or(0, argument_count);
        let reporter = self.reporter_of(function_name_context);
        let trace_key = function_name_context.start_token().map(Token::start_index);
        self.add_call_instruction(Box::new(CallFunctionInstruction::new(
            reporter,
            function_name,
            arg_size,
            trace_key,
        )));
    }

    /// Java `visitListExprInner`.
    fn visit_list_expr_inner(
        &mut self,
        list_items: Option<&Node>,
        list_error_reporter: Rc<dyn ErrorReporter>,
    ) {
        let Some(Node::ListItems(ctx)) = list_items else {
            self.add_instruction(Box::new(NewListInstruction::new(list_error_reporter, 0)));
            return;
        };
        for expression in &ctx.expressions {
            expression.accept(self);
        }
        self.add_instruction(Box::new(NewListInstruction::new(
            list_error_reporter,
            ctx.expressions.len(),
        )));
    }

    /// Java `parseMapKey`.
    fn parse_map_key(&self, map_key: &Node) -> String {
        match map_key {
            Node::IdKey(_) => map_key.text(),
            Node::StringKey(_) | Node::QuoteStringKey(_) => {
                QLStringUtils::parse_string_escape(&map_key.text())
            }
            // shouldn't run here
            _ => panic!("unexpected map key node"),
        }
    }

    /// Java `visitSwitchStatement` (traditional `case X:` style).
    fn visit_switch_statement(&mut self, ctx: &SwitchExprContext) {
        // Evaluate switch expression once and store in temporary variable
        let switch_count = self.switch_count();
        let switch_var_name = format!("@switch_{switch_count}");
        let switch_key_token = ctx.switch_token.symbol();
        let switch_error_reporter = self.new_reporter_with_token(switch_key_token);

        // Create scope for switch
        let switch_scope_name = self.child_scope_name(&format!("SWITCH_{switch_count}"));
        self.add_instruction(Box::new(NewScopeInstruction::new(
            Rc::clone(&switch_error_reporter),
            switch_scope_name.clone(),
        )));

        // Evaluate and store switch expression
        ctx.expression.accept(self);
        self.add_instruction(Box::new(DefineLocalInstruction::new(
            Rc::clone(&switch_error_reporter),
            switch_var_name.clone(),
            Some(object_cls()),
        )));

        let groups: Vec<&SwitchStatementGroupContext> = switch_groups(ctx)
            .filter_map(|g| match g {
                Node::SwitchStatementGroup(group) => Some(group),
                _ => None,
            })
            .collect();

        // Collect case bodies and metadata
        let mut case_bodies: Vec<Vec<Instruction>> = Vec::new();
        let mut case_breaks: Vec<Vec<usize>> = Vec::new();
        let mut case_trace_keys: Vec<Option<i32>> = Vec::new();
        let mut case_conditions: Vec<Vec<&Node>> = Vec::new();
        let mut default_index: i32 = -1;

        for (i, group) in groups.iter().enumerate() {
            let mut conditions = Vec::new();
            if let Node::SwitchLabels(labels_ctx) = &*group.labels {
                for label in &labels_ctx.labels {
                    if let Node::SwitchLabel(label_ctx) = label {
                        if label_ctx.case_token.is_some() {
                            if let Some(expr) = &label_ctx.expression {
                                conditions.push(&**expr);
                            }
                        } else if label_ctx.default_token.is_some() {
                            default_index = i as i32;
                        }
                    }
                }
            }
            case_conditions.push(conditions);
            case_trace_keys.push(
                group
                    .block_statements
                    .as_deref()
                    .and_then(Node::start_token)
                    .map(Token::start_index),
            );

            // Generate case body instructions; record top-level `break`s
            // (Java finds them with `instanceof` afterwards).
            let (body_instructions, break_indices) = match &group.block_statements {
                None => (Vec::new(), Vec::new()),
                Some(body) => {
                    let mut body_visitor =
                        self.sub_visitor(Rc::clone(&self.generator_scope), Context::Macro);
                    body_visitor.collect_break_indices = Some(Vec::new());
                    body.accept(&mut body_visitor);
                    self.propagate_error(&body_visitor);
                    let breaks = body_visitor
                        .collect_break_indices
                        .take()
                        .unwrap_or_default();
                    (body_visitor.take_instructions(), breaks)
                }
            };
            case_bodies.push(body_instructions);
            case_breaks.push(break_indices);
        }

        // Generate comparison and jump logic
        let mut case_jumps: Vec<(Rc<JumpIfPopInstruction>, usize)> = Vec::new();

        for conditions in &case_conditions {
            for cond in conditions {
                // Load switch value
                self.add_instruction(Box::new(LoadInstruction::new(
                    Rc::clone(&switch_error_reporter),
                    switch_var_name.clone(),
                    None,
                )));
                // Evaluate case expression
                cond.accept(self);
                // Compare for equality using == operator
                let equals_op = self
                    .operator_factory
                    .get_binary_operator("==")
                    .expect("'==' operator must exist");
                self.add_instruction(Box::new(OperatorInstruction::new(
                    Rc::clone(&switch_error_reporter),
                    equals_op,
                    None,
                )));

                // If equal (result is true), jump to case body
                let jump_to_case = Rc::new(JumpIfPopInstruction::new(
                    Rc::clone(&switch_error_reporter),
                    true,
                    -1,
                ));
                self.pure_add_shared(Rc::clone(&jump_to_case) as SharedInstruction);
                case_jumps.push((jump_to_case, self.instruction_list.len() - 1));
            }
        }

        // No match, jump to default or end
        let jump_to_default_or_end =
            Rc::new(JumpInstruction::new(Rc::clone(&switch_error_reporter), -1));
        self.pure_add_shared(Rc::clone(&jump_to_default_or_end) as SharedInstruction);
        let jump_to_default_start_pos = self.instruction_list.len();

        // Generate case bodies
        let mut break_jumps: Vec<(Rc<JumpInstruction>, usize)> = Vec::new();
        let mut case_jump_index = 0;

        for (i, body) in case_bodies.into_iter().enumerate() {
            // Set jump targets for this case
            let num_conditions = case_conditions[i].len();
            let case_start_pos = self.instruction_list.len();

            for _ in 0..num_conditions {
                if case_jump_index < case_jumps.len() {
                    let (jump, jump_instr_pos) = &case_jumps[case_jump_index];
                    // Position should be relative to the instruction AFTER
                    // the JumpIfPop
                    let jump_start = jump_instr_pos + 1;
                    jump.set_position((case_start_pos - jump_start) as i32);
                    case_jump_index += 1;
                }
            }

            // Set default jump target
            if i as i32 == default_index {
                jump_to_default_or_end
                    .set_position((case_start_pos - jump_to_default_start_pos) as i32);
            }

            // Java 的 switch trace 将被选中的 statement group 记录为
            // `BLOCK ... null`，包括以 break 结束、没有产生表达式值的分支。
            // case 体本身不是 BlockExpr，不会生成 TracePeek，因此在进入
            // 分支时显式标记其 block trace 已执行。
            if self.init_options.is_trace_expression() {
                if let Some(trace_key) = case_trace_keys[i] {
                    self.pure_add_instruction(Box::new(TraceEvaluatedInstruction::new(
                        Rc::clone(&switch_error_reporter),
                        Some(trace_key),
                    )));
                }
            }

            // Add case body, replacing top-level `break` with a jump to
            // the end of the switch (Java behaviour).
            for (idx, instruction) in body.into_iter().enumerate() {
                if case_breaks[i].contains(&idx) {
                    let break_jump =
                        Rc::new(JumpInstruction::new(Rc::clone(&switch_error_reporter), -1));
                    self.pure_add_shared(Rc::clone(&break_jump) as SharedInstruction);
                    break_jumps.push((break_jump, self.instruction_list.len() - 1));
                } else {
                    self.pure_add_instruction(instruction);
                }
            }
        }

        // Set end position
        let end_position = self.instruction_list.len();

        // Fix up break jumps
        for (break_jump, break_jump_pos) in break_jumps {
            break_jump.set_position((end_position - break_jump_pos - 1) as i32);
        }

        // If no default, set jump to end
        if default_index == -1 {
            jump_to_default_or_end.set_position((end_position - jump_to_default_start_pos) as i32);
        }

        // If no case matched and no explicit return, push null
        let needs_default_value = if default_index >= 0 {
            let default_body = &groups[default_index as usize].block_statements;
            !last_stmt_is_return_or_break(default_body.as_deref())
        } else {
            true
        };
        if needs_default_value {
            self.add_instruction(Box::new(ConstInstruction::new(
                Rc::clone(&switch_error_reporter),
                DataValue::NULL_VALUE,
                None,
            )));
        }
        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                Rc::clone(&switch_error_reporter),
                Some(switch_key_token.start_index()),
            )));
        }

        self.add_instruction(Box::new(CloseScopeInstruction::new(
            switch_error_reporter,
            switch_scope_name,
        )));
    }

    /// Java `visitSwitchExpression` (`case X -> expr` style).
    fn visit_switch_expression(&mut self, ctx: &SwitchExprContext) {
        // Evaluate switch expression once and store in temporary variable
        let switch_count = self.switch_count();
        let switch_var_name = format!("@switch_expr_{switch_count}");
        let switch_key_token = ctx.switch_token.symbol();
        let switch_error_reporter = self.new_reporter_with_token(switch_key_token);

        // Create scope for switch expression
        let switch_scope_name = self.child_scope_name(&format!("SWITCH_EXPR_{switch_count}"));
        self.add_instruction(Box::new(NewScopeInstruction::new(
            Rc::clone(&switch_error_reporter),
            switch_scope_name.clone(),
        )));

        // Evaluate and store switch expression
        ctx.expression.accept(self);
        self.add_instruction(Box::new(DefineLocalInstruction::new(
            Rc::clone(&switch_error_reporter),
            switch_var_name.clone(),
            Some(object_cls()),
        )));

        let groups: Vec<&SwitchExprGroupContext> = switch_groups(ctx)
            .filter_map(|g| match g {
                Node::SwitchExprGroup(group) => Some(group),
                _ => None,
            })
            .collect();

        // Generate jump instructions for each case
        let mut case_jumps: Vec<(Rc<JumpIfPopInstruction>, usize)> = Vec::new();
        let mut default_index: i32 = -1;

        // First pass: generate comparisons and collect metadata
        for (i, group) in groups.iter().enumerate() {
            if let Node::SwitchExpressionLabel(label) = &*group.label {
                if label.case_token.is_some() {
                    if let Some(expr_list) = &label.expression_list {
                        if let Node::ExpressionList(list_ctx) = &**expr_list {
                            for case_value in &list_ctx.expressions {
                                // Load switch value
                                self.add_instruction(Box::new(LoadInstruction::new(
                                    Rc::clone(&switch_error_reporter),
                                    switch_var_name.clone(),
                                    None,
                                )));
                                // Evaluate case expression
                                case_value.accept(self);
                                // Compare for equality using == operator
                                let equals_op = self
                                    .operator_factory
                                    .get_binary_operator("==")
                                    .expect("'==' operator must exist");
                                self.add_instruction(Box::new(OperatorInstruction::new(
                                    Rc::clone(&switch_error_reporter),
                                    equals_op,
                                    None,
                                )));

                                // If equal (result is true), jump to case
                                // body
                                let jump_to_case = Rc::new(JumpIfPopInstruction::new(
                                    Rc::clone(&switch_error_reporter),
                                    true,
                                    -1,
                                ));
                                self.pure_add_shared(Rc::clone(&jump_to_case) as SharedInstruction);
                                case_jumps.push((jump_to_case, self.instruction_list.len() - 1));
                            }
                        }
                    }
                } else if label.default_token.is_some() {
                    default_index = i as i32;
                }
            }
        }

        // No match, jump to default or error
        let jump_to_default_or_error =
            Rc::new(JumpInstruction::new(Rc::clone(&switch_error_reporter), -1));
        self.pure_add_shared(Rc::clone(&jump_to_default_or_error) as SharedInstruction);
        let jump_to_default_start_pos = self.instruction_list.len();

        // Second pass: generate case bodies
        let mut end_jumps: Vec<(Rc<JumpInstruction>, usize)> = Vec::new();
        let mut case_jump_index = 0;

        for (i, group) in groups.iter().enumerate() {
            let Node::SwitchExpressionLabel(label) = &*group.label else {
                continue;
            };

            // Set jump targets for this case
            let case_start_pos = self.instruction_list.len();

            if label.case_token.is_some() {
                let num_case_values = match label.expression_list.as_deref() {
                    Some(Node::ExpressionList(list_ctx)) => list_ctx.expressions.len(),
                    _ => 0,
                };
                for _ in 0..num_case_values {
                    if case_jump_index < case_jumps.len() {
                        let (jump, jump_instr_pos) = &case_jumps[case_jump_index];
                        let jump_start = jump_instr_pos + 1;
                        jump.set_position((case_start_pos - jump_start) as i32);
                        case_jump_index += 1;
                    }
                }
            } else if label.default_token.is_some() && i as i32 == default_index {
                // Set default jump target
                jump_to_default_or_error
                    .set_position((case_start_pos - jump_to_default_start_pos) as i32);
            }

            // Evaluate result expression for this case
            group.expression.accept(self);

            // Jump to end after evaluating result
            let jump_to_end = Rc::new(JumpInstruction::new(Rc::clone(&switch_error_reporter), -1));
            self.pure_add_shared(Rc::clone(&jump_to_end) as SharedInstruction);
            end_jumps.push((jump_to_end, self.instruction_list.len() - 1));
        }

        // Set end position
        let end_position = self.instruction_list.len();

        // Fix up all end jumps
        for (end_jump, end_jump_pos) in end_jumps {
            end_jump.set_position((end_position - end_jump_pos - 1) as i32);
        }

        // If no default, jump to end with null (should not happen in
        // well-formed switch expressions)
        if default_index == -1 {
            jump_to_default_or_error
                .set_position((end_position - jump_to_default_start_pos) as i32);
            self.add_instruction(Box::new(ConstInstruction::new(
                Rc::clone(&switch_error_reporter),
                DataValue::NULL_VALUE,
                None,
            )));
        }

        if self.init_options.is_trace_expression() {
            self.pure_add_instruction(Box::new(TracePeekInstruction::new(
                Rc::clone(&switch_error_reporter),
                Some(switch_key_token.start_index()),
            )));
        }

        self.add_instruction(Box::new(CloseScopeInstruction::new(
            switch_error_reporter,
            switch_scope_name,
        )));
    }
}

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
                Some(std::cmp::Ordering::Greater) => {
                    Some(DataValue::BigDec(normalized))
                }
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
