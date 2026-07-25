//! Stage 3b end-to-end tests: script -> tokens -> syntax tree -> QVM
//! instruction sequence -> `QvmRuntime` execution.
//!
//! Concrete operators arrive in Stage 4, so these tests register mock
//! arithmetic/comparison/logic operators into an
//! [`OperatorManager`] (the same instance feeds the lexer/parser and the
//! compile visitor, like Java's `OperatorManager`).

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress_rust::aparser::compile_time_function::{CodeGenerator, CompileTimeFunction};
use qlexpress_rust::aparser::import_manager::ImportManager;
use qlexpress_rust::aparser::interpolation_mode::InterpolationMode;
use qlexpress_rust::aparser::operator_factory::{OperatorFactory, OperatorManager};
use qlexpress_rust::aparser::qlparser::build_tree;
use qlexpress_rust::aparser::qvm_instruction_visitor::{
    compile_script, CompileTimeFunctions, UserDefineFunctions,
};
use qlexpress_rust::class_supplier::DefaultClassSupplier;
use qlexpress_rust::exception::error_reporter::ErrorReporter;
use qlexpress_rust::exception::pure_err_reporter::PureErrReporter;
use qlexpress_rust::exception::QLException;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::ql_precedences;
use qlexpress_rust::runtime::data::index_map::IndexMap;
use qlexpress_rust::runtime::function::{CustomFunction, LazyArgCustomFunction};
use qlexpress_rust::runtime::instruction::{
    ConstInstruction, Instruction, QLInstruction,
};
use qlexpress_rust::runtime::member::{NativeRegistry, NativeType};
use qlexpress_rust::runtime::operator::base::{BinaryOperator, UnaryOperator};
use qlexpress_rust::runtime::operator::custom_binary_operator::CustomBinaryOperator;
use qlexpress_rust::runtime::parameters::Parameters;
use qlexpress_rust::runtime::qcontext::QContext;
use qlexpress_rust::runtime::qlambda::{QLambdaDefinition, QLambdaDefinitionInner};
use qlexpress_rust::runtime::qvm_global_scope::QvmGlobalScope;
use qlexpress_rust::runtime::qvm_runtime::QvmRuntime;
use qlexpress_rust::runtime::value::{DataValue, QValue};

// ---- mock operators (Stage 4 delivers the real ones) ---------------------

#[derive(Clone, Copy)]
enum BinKind {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

struct MockBin {
    kind: BinKind,
    lexeme: &'static str,
    priority: i32,
}

fn as_f64(v: &DataValue) -> Option<f64> {
    match v {
        DataValue::Byte(x) => Some(*x as f64),
        DataValue::Short(x) => Some(*x as f64),
        DataValue::Int(x) => Some(*x as f64),
        DataValue::Long(x) => Some(*x as f64),
        DataValue::Float(x) => Some(*x as f64),
        DataValue::Double(x) => Some(*x),
        DataValue::BigInt(x) => Some(*x as f64),
        _ => None,
    }
}

fn is_float(v: &DataValue) -> bool {
    matches!(v, DataValue::Float(_) | DataValue::Double(_))
}

fn num_bin(l: &DataValue, r: &DataValue, f: impl Fn(f64, f64) -> f64) -> DataValue {
    let v = f(as_f64(l).unwrap(), as_f64(r).unwrap());
    if is_float(l) || is_float(r) {
        DataValue::Double(v)
    } else if matches!(l, DataValue::Int(_)) && matches!(r, DataValue::Int(_)) {
        DataValue::Int(v as i32)
    } else {
        DataValue::Long(v as i64)
    }
}

impl BinaryOperator for MockBin {
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _ctx: &mut dyn QContext,
        _opts: &QLOptions,
        reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let l = left.get();
        let r = right.get();
        Ok(match self.kind {
            BinKind::Assign => {
                let Some(slot) = left.as_left() else {
                    return Err(reporter.report("INVALID_ASSIGNMENT", "not a left value"));
                };
                slot.borrow_mut().set(r.clone(), reporter)?;
                r
            }
            BinKind::Add => {
                if matches!(l, DataValue::Str(_)) || matches!(r, DataValue::Str(_)) {
                    DataValue::Str(format!(
                        "{}{}",
                        l.string_value_of(),
                        r.string_value_of()
                    ))
                } else {
                    num_bin(&l, &r, |a, b| a + b)
                }
            }
            BinKind::Sub => num_bin(&l, &r, |a, b| a - b),
            BinKind::Mul => num_bin(&l, &r, |a, b| a * b),
            BinKind::Div => num_bin(&l, &r, |a, b| a / b),
            BinKind::Rem => num_bin(&l, &r, |a, b| a % b),
            BinKind::Eq => DataValue::Bool(l == r),
            BinKind::Ne => DataValue::Bool(l != r),
            BinKind::Lt => {
                DataValue::Bool(as_f64(&l).zip(as_f64(&r)).is_some_and(|(a, b)| a < b))
            }
            BinKind::Le => {
                DataValue::Bool(as_f64(&l).zip(as_f64(&r)).is_some_and(|(a, b)| a <= b))
            }
            BinKind::Gt => {
                DataValue::Bool(as_f64(&l).zip(as_f64(&r)).is_some_and(|(a, b)| a > b))
            }
            BinKind::Ge => {
                DataValue::Bool(as_f64(&l).zip(as_f64(&r)).is_some_and(|(a, b)| a >= b))
            }
            BinKind::And => match (&l, &r) {
                (DataValue::Bool(a), DataValue::Bool(b)) => DataValue::Bool(*a && *b),
                _ => return Err(reporter.report("INVALID_BINARY_OPERAND", "not booleans")),
            },
            BinKind::Or => match (&l, &r) {
                (DataValue::Bool(a), DataValue::Bool(b)) => DataValue::Bool(*a || *b),
                _ => return Err(reporter.report("INVALID_BINARY_OPERAND", "not booleans")),
            },
        })
    }

    fn operator(&self) -> &str {
        self.lexeme
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

struct MockUnary {
    lexeme: &'static str,
    not: bool,
}

impl UnaryOperator for MockUnary {
    fn execute(
        &self,
        value: &QValue,
        reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let v = value.get();
        if self.not {
            return match v {
                DataValue::Bool(b) => Ok(DataValue::Bool(!b)),
                _ => Err(reporter.report("INVALID_UNARY_OPERAND", "not boolean")),
            };
        }
        // unary minus
        match v {
            DataValue::Int(x) => Ok(DataValue::Int(-x)),
            DataValue::Long(x) => Ok(DataValue::Long(-x)),
            DataValue::Double(x) => Ok(DataValue::Double(-x)),
            _ => Err(reporter.report("INVALID_UNARY_OPERAND", "not number")),
        }
    }

    fn operator(&self) -> &str {
        self.lexeme
    }

    fn priority(&self) -> i32 {
        ql_precedences::UNARY
    }
}

fn operator_manager() -> OperatorManager {
    let mut manager = OperatorManager::new();
    let mut reg = |lexeme: &'static str, kind: BinKind, priority: i32| {
        manager.register_default_binary_operator(Rc::new(MockBin {
            kind,
            lexeme,
            priority,
        }));
    };
    reg("=", BinKind::Assign, ql_precedences::ASSIGN);
    reg("||", BinKind::Or, ql_precedences::OR);
    reg("&&", BinKind::And, ql_precedences::AND);
    reg("==", BinKind::Eq, ql_precedences::EQUAL);
    reg("!=", BinKind::Ne, ql_precedences::EQUAL);
    reg("<", BinKind::Lt, ql_precedences::COMPARE);
    reg("<=", BinKind::Le, ql_precedences::COMPARE);
    reg(">", BinKind::Gt, ql_precedences::COMPARE);
    reg(">=", BinKind::Ge, ql_precedences::COMPARE);
    reg("+", BinKind::Add, ql_precedences::ADD);
    reg("-", BinKind::Sub, ql_precedences::ADD);
    reg("*", BinKind::Mul, ql_precedences::MULTI);
    reg("/", BinKind::Div, ql_precedences::MULTI);
    reg("%", BinKind::Rem, ql_precedences::MULTI);
    manager.register_default_prefix_unary_operator(Rc::new(MockUnary {
        lexeme: "!",
        not: true,
    }));
    manager.register_default_prefix_unary_operator(Rc::new(MockUnary {
        lexeme: "-",
        not: false,
    }));
    manager
}

// ---- harness --------------------------------------------------------------

fn init_options() -> InitOptions {
    InitOptions::default()
}

/// Compile `script` to an instruction sequence and assert stack balance
/// (Java maintains the same running stackSize invariant).
fn compile(
    script: &str,
    operator_manager: &OperatorManager,
    supplier: &DefaultClassSupplier,
    compile_time_functions: &CompileTimeFunctions,
    user_define_functions: &UserDefineFunctions,
) -> (Vec<Instruction>, usize) {
    let options = init_options();
    let tree = build_tree(
        script,
        Some(operator_manager),
        false,
        |_| {},
        options.interpolation_mode(),
        options.selector_start(),
        options.selector_end(),
        options.is_strict_new_lines(),
    )
    .unwrap_or_else(|err| panic!("parse failed for {script:?}: {err:?}"));
    let import_manager = RefCell::new(ImportManager::new(supplier, vec![]));
    let (instructions, max_stack) = compile_script(
        script,
        &tree,
        &import_manager,
        None,
        operator_manager,
        compile_time_functions,
        user_define_functions,
        &options,
    )
    .unwrap_or_else(|err| panic!("compile failed for {script:?}: {err:?}"));
    assert_stack_balance(&instructions);
    (instructions, max_stack)
}

/// The running stack depth must never go negative and must return to zero
/// at the end of the top-level sequence.
fn assert_stack_balance(instructions: &[Instruction]) {
    let mut depth = 0i32;
    for (i, instruction) in instructions.iter().enumerate() {
        depth += instruction.stack_output() - instruction.stack_input();
        assert!(
            depth >= 0,
            "stack underflow at instruction {i} (depth {depth})"
        );
    }
    assert_eq!(depth, 0, "top-level instruction sequence not balanced");
}

fn run_with_scope(
    script: &str,
    global_scope: QvmGlobalScope,
    registry: NativeRegistry,
    user_define_functions: UserDefineFunctions,
) -> DataValue {
    let operator_manager = operator_manager();
    let supplier = DefaultClassSupplier::instance();
    let compile_time_functions = CompileTimeFunctions::new();
    let (instructions, max_stack) = compile(
        script,
        &operator_manager,
        &supplier,
        &compile_time_functions,
        &user_define_functions,
    );
    let root: Rc<dyn QLambdaDefinition> = Rc::new(QLambdaDefinitionInner::new(
        "main",
        instructions,
        vec![],
        max_stack,
    ));
    let runtime = Rc::new(QvmRuntime::for_test(Rc::new(registry)));
    let options = QLOptions::builder().build();
    let result = runtime
        .execute(global_scope, root, &options)
        .unwrap_or_else(|err| panic!("execution failed for {script:?}: {err:?}"));
    result.value()
}

fn run(script: &str) -> DataValue {
    run_with_scope(
        script,
        QvmGlobalScope::empty(),
        NativeRegistry::with_builtins(),
        UserDefineFunctions::new(),
    )
}

// ---- compile-time function (Java CompileTimeFunction mechanism) -----------

/// Emits a constant instruction at compile time, like Java's built-in
/// compile-time functions (e.g. `_max`).
struct FortyTwo;

impl CompileTimeFunction for FortyTwo {
    fn create_function_instruction(
        &self,
        _function_name: &str,
        _arguments: &[&qlexpress_rust::aparser::syntax_tree::Node],
        _operator_factory: &dyn OperatorFactory,
        code_generator: &mut dyn CodeGenerator,
    ) {
        let reporter = code_generator.error_reporter();
        code_generator.add_instruction(Box::new(ConstInstruction::new(
            reporter,
            DataValue::Int(42),
            None,
        )));
    }
}

// ---- lazy-arg custom function ---------------------------------------------

/// `choose(cond, lazyValue, fallback)`: the second argument is compiled
/// into a lambda and only invoked when `cond` is true.
struct Choose;

impl CustomFunction for Choose {
    fn call(
        &self,
        _q_context: &mut dyn QContext,
        parameters: &Parameters,
    ) -> Result<DataValue, QLException> {
        let values = parameters.values();
        match &values[..] {
            [DataValue::Bool(true), DataValue::Lambda(lambda), _] => {
                Ok(lambda.call(&[])?.value())
            }
            [DataValue::Bool(false), _, fallback] => Ok(fallback.clone()),
            _ => Ok(DataValue::Null),
        }
    }

    fn as_lazy_arg(&self) -> Option<&dyn LazyArgCustomFunction> {
        Some(self)
    }
}

impl LazyArgCustomFunction for Choose {
    fn is_lazy_arg(&self, index: usize) -> bool {
        index == 1
    }
}

// ---- tests ----------------------------------------------------------------

#[test]
fn arithmetic_and_precedence() {
    assert_eq!(run("1 + 2 * 3"), DataValue::Int(7));
    assert_eq!(run("(1 + 2) * 3"), DataValue::Int(9));
    assert_eq!(run("10 % 3"), DataValue::Int(1));
    assert_eq!(run("-5 + 2"), DataValue::Int(-3));
    assert_eq!(run("1.5 + 2.5"), DataValue::Double(4.0));
}

#[test]
fn variables_and_assignment() {
    assert_eq!(run("x = 3; y = x * 2; y + 1"), DataValue::Int(7));
    assert_eq!(run("int a = 5; a + 1"), DataValue::Int(6));
}

#[test]
fn context_variable() {
    let external = Rc::new(RefCell::new(IndexMap::from_entries(vec![(
        DataValue::Str("x".to_string()),
        DataValue::Int(40),
    )])));
    let scope = QvmGlobalScope::new(external, HashMap::new(), false);
    assert_eq!(
        run_with_scope(
            "x + 2",
            scope,
            NativeRegistry::with_builtins(),
            UserDefineFunctions::new()
        ),
        DataValue::Int(42)
    );
}

#[test]
fn if_else_and_ternary() {
    assert_eq!(run("if (1 < 2) { 'yes' } else { 'no' }"), DataValue::Str("yes".into()));
    assert_eq!(run("if (1 > 2) { 'yes' } else { 'no' }"), DataValue::Str("no".into()));
    assert_eq!(run("x = 5; if (x > 3) 'big' else 'small'"), DataValue::Str("big".into()));
    assert_eq!(run("1 < 2 ? 10 : 20"), DataValue::Int(10));
    assert_eq!(run("1 > 2 ? 10 : 20"), DataValue::Int(20));
}

#[test]
fn short_circuit_logic() {
    assert_eq!(run("true && false"), DataValue::Bool(false));
    assert_eq!(run("true || false"), DataValue::Bool(true));
    // short circuit: right side must not blow up
    assert_eq!(run("false && (1 / 0 > 0)"), DataValue::Bool(false));
    assert_eq!(run("!(1 > 2)"), DataValue::Bool(true));
}

#[test]
fn while_loop_with_break_continue() {
    assert_eq!(
        run("s = 0; i = 0; while (i < 10) { i = i + 1; if (i == 3) continue; if (i > 5) break; s = s + i; } s"),
        // 1+2+4+5+6
        DataValue::Int(18)
    );
}

#[test]
fn traditional_for_loop() {
    assert_eq!(
        run("s = 0; for (i = 0; i < 5; i = i + 1) { s = s + i; } s"),
        DataValue::Int(10)
    );
    assert_eq!(
        run("s = 0; for (int i = 1; i <= 3; i = i + 1) s = s + i; s"),
        DataValue::Int(6)
    );
}

#[test]
fn foreach_over_list() {
    assert_eq!(
        run("s = 0; for (x : [1, 2, 3, 4]) { s = s + x; } s"),
        DataValue::Int(10)
    );
}

#[test]
fn function_definition_call_and_recursion() {
    assert_eq!(
        run("function add(a, b) { return a + b; } add(2, 3)"),
        DataValue::Int(5)
    );
    assert_eq!(
        run("function fib(n) { if (n < 2) { return n; } return fib(n - 1) + fib(n - 2); } fib(10)"),
        DataValue::Int(55)
    );
    // forward reference (Java: function definitions hoist)
    assert_eq!(
        run("f(3); function f(x) { return x * 2; }"),
        DataValue::Int(6)
    );
}

#[test]
fn lambda_definition_and_call() {
    assert_eq!(run("f = x -> x * 2; f(21)"), DataValue::Int(42));
    assert_eq!(
        run("f = (a, b) -> { return a * b; }; f(6, 7)"),
        DataValue::Int(42)
    );
}

#[test]
fn try_catch_and_throw() {
    assert_eq!(
        run("try { throw 'boom'; } catch (e) { 'caught'; }"),
        DataValue::Str("caught".into())
    );
    assert_eq!(
        run("x = 0; try { x = 1; throw 'x'; } catch (e) { x = 2; } finally { x = x + 10; } x"),
        DataValue::Int(12)
    );
}

#[test]
fn string_interpolation() {
    assert_eq!(
        run("x = 3; \"a ${x + 1} b\""),
        DataValue::Str("a 4 b".into())
    );
    assert_eq!(run("\"plain\""), DataValue::Str("plain".into()));
}

#[test]
fn list_and_map_literals() {
    assert_eq!(
        run("[1, 2, 3]"),
        DataValue::list(vec![
            DataValue::Int(1),
            DataValue::Int(2),
            DataValue::Int(3)
        ])
    );
    assert_eq!(run("m = {'a': 1, 'b': 2}; m['a'] + m['b']"), DataValue::Int(3));
    assert_eq!(run("l = [10, 20, 30]; l[1]"), DataValue::Int(20));
}

#[test]
fn switch_statement_and_expression() {
    // traditional statement style (bodies are var declarations, keeping
    // the stack balanced per case)
    let external = Rc::new(RefCell::new(IndexMap::from_entries(vec![(
        DataValue::Str("hit".to_string()),
        DataValue::Int(0),
    )])));
    let scope = QvmGlobalScope::new(external, HashMap::new(), false);
    let script = "x = 2; switch (x) { case 1: int r = 10; break; case 2: hit = 22; break; default: hit = 99; } hit";
    assert_eq!(
        run_with_scope(script, scope, NativeRegistry::with_builtins(), UserDefineFunctions::new()),
        DataValue::Int(22)
    );
}

#[test]
fn new_native_instance() {
    let mut registry = NativeRegistry::with_builtins();
    let mut point_type = NativeType::named("com.example.Point");
    point_type.constructor = Some(Rc::new(|args: &[DataValue]| {
        let mut map = IndexMap::new();
        map.insert(DataValue::Str("x".into()), args.first().cloned().unwrap_or(DataValue::Null));
        Ok(DataValue::map(map))
    }));
    let mut fields = HashMap::new();
    fields.insert(
        "x".to_string(),
        Rc::new(|bean: &DataValue| match bean {
            DataValue::Map(map) => map.borrow().get(&DataValue::Str("x".into())).cloned(),
            _ => None,
        }) as qlexpress_rust::runtime::member::NativeFieldGetter,
    );
    point_type.fields = fields;
    registry.register_type(point_type);

    let mut supplier = DefaultClassSupplier::instance();
    supplier.register("com.example.Point");

    let operator_manager = operator_manager();
    let options = init_options();
    let script = "p = new com.example.Point(7); p.x";
    let tree = build_tree(
        script,
        Some(&operator_manager),
        false,
        |_| {},
        options.interpolation_mode(),
        options.selector_start(),
        options.selector_end(),
        options.is_strict_new_lines(),
    )
    .expect("parse");
    let import_manager = RefCell::new(ImportManager::new(&supplier, vec![]));
    let (instructions, max_stack) = compile_script(
        script,
        &tree,
        &import_manager,
        None,
        &operator_manager,
        &CompileTimeFunctions::new(),
        &UserDefineFunctions::new(),
        &options,
    )
    .expect("compile");
    let root: Rc<dyn QLambdaDefinition> =
        Rc::new(QLambdaDefinitionInner::new("main", instructions, vec![], max_stack));
    let runtime = Rc::new(QvmRuntime::for_test(Rc::new(registry)));
    let result = runtime
        .execute(QvmGlobalScope::empty(), root, &QLOptions::builder().build())
        .expect("execute");
    assert_eq!(result.value(), DataValue::Int(7));
}

#[test]
fn macro_definition_and_expansion() {
    assert_eq!(run("macro inc { a + 1 } a = 5; inc"), DataValue::Int(6));
    assert_eq!(
        run("macro twice { x = x * 2 } x = 3; twice; x"),
        DataValue::Int(6)
    );
}

#[test]
fn compile_time_function_mechanism() {
    let operator_manager = operator_manager();
    let supplier = DefaultClassSupplier::instance();
    let mut compile_time_functions = CompileTimeFunctions::new();
    compile_time_functions.insert("_fortytwo".to_string(), Rc::new(FortyTwo));
    let (instructions, max_stack) = compile(
        "_fortytwo()",
        &operator_manager,
        &supplier,
        &compile_time_functions,
        &UserDefineFunctions::new(),
    );
    let root: Rc<dyn QLambdaDefinition> =
        Rc::new(QLambdaDefinitionInner::new("main", instructions, vec![], max_stack));
    let runtime = Rc::new(QvmRuntime::for_test(Rc::new(NativeRegistry::with_builtins())));
    let result = runtime
        .execute(QvmGlobalScope::empty(), root, &QLOptions::builder().build())
        .expect("execute");
    assert_eq!(result.value(), DataValue::Int(42));
}

#[test]
fn lazy_argument_function() {
    let mut user_functions = UserDefineFunctions::new();
    user_functions.insert("choose".to_string(), Rc::new(Choose) as Rc<dyn CustomFunction>);

    let mut external_functions: HashMap<String, Rc<dyn CustomFunction>> = HashMap::new();
    external_functions.insert("choose".to_string(), Rc::new(Choose));
    let scope = QvmGlobalScope::new(
        Rc::new(RefCell::new(IndexMap::new())),
        external_functions,
        false,
    );
    // the lazy argument `1 / 0` is wrapped in a lambda and never evaluated
    assert_eq!(
        run_with_scope(
            "choose(false, 1 / 0, 'safe')",
            scope,
            NativeRegistry::with_builtins(),
            user_functions
        ),
        DataValue::Str("safe".into())
    );
}

#[test]
fn custom_binary_operator_end_to_end() {
    struct Pow;
    impl CustomBinaryOperator for Pow {
        fn execute(&self, left: &QValue, right: &QValue) -> Result<DataValue, QLException> {
            let l = as_f64(&left.get()).unwrap_or(0.0);
            let r = as_f64(&right.get()).unwrap_or(0.0);
            Ok(DataValue::Int(l.powf(r) as i32))
        }
    }

    let mut operator_manager = operator_manager();
    assert!(operator_manager.add_binary_operator("**", Rc::new(Pow), ql_precedences::MULTI));
    let supplier = DefaultClassSupplier::instance();
    let (instructions, max_stack) = compile(
        "2 ** 3",
        &operator_manager,
        &supplier,
        &CompileTimeFunctions::new(),
        &UserDefineFunctions::new(),
    );
    let root: Rc<dyn QLambdaDefinition> =
        Rc::new(QLambdaDefinitionInner::new("main", instructions, vec![], max_stack));
    let runtime = Rc::new(QvmRuntime::for_test(Rc::new(NativeRegistry::with_builtins())));
    let result = runtime
        .execute(QvmGlobalScope::empty(), root, &QLOptions::builder().build())
        .expect("execute");
    assert_eq!(result.value(), DataValue::Int(8));
}

#[test]
fn generated_instruction_stack_balance_spot_check() {
    // explicitly spot-check a script with nested control flow
    let operator_manager = operator_manager();
    let supplier = DefaultClassSupplier::instance();
    compile(
        "function f(n) { if (n < 2) { return n; } return f(n-1) + f(n-2); } s = 0; for (i = 0; i < 5; i = i + 1) { s = s + f(i); } s",
        &operator_manager,
        &supplier,
        &CompileTimeFunctions::new(),
        &UserDefineFunctions::new(),
    );
}

#[test]
fn interpolation_disable_mode() {
    let operator_manager = operator_manager();
    let supplier = DefaultClassSupplier::instance();
    let options = InitOptions::builder()
        .interpolation_mode(InterpolationMode::Disable)
        .build();
    let script = "\"a ${1 + 1} b\"";
    let tree = build_tree(
        script,
        Some(&operator_manager),
        false,
        |_| {},
        options.interpolation_mode(),
        options.selector_start(),
        options.selector_end(),
        options.is_strict_new_lines(),
    )
    .expect("parse");
    let import_manager = RefCell::new(ImportManager::new(&supplier, vec![]));
    let (instructions, max_stack) = compile_script(
        script,
        &tree,
        &import_manager,
        None,
        &operator_manager,
        &CompileTimeFunctions::new(),
        &UserDefineFunctions::new(),
        &options,
    )
    .expect("compile");
    let root: Rc<dyn QLambdaDefinition> =
        Rc::new(QLambdaDefinitionInner::new("main", instructions, vec![], max_stack));
    let runtime = Rc::new(QvmRuntime::for_test(Rc::new(NativeRegistry::with_builtins())));
    let result = runtime
        .execute(QvmGlobalScope::empty(), root, &QLOptions::builder().build())
        .expect("execute");
    assert_eq!(result.value(), DataValue::Str("a ${1 + 1} b".into()));
}
