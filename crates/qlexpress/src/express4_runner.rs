//! 引擎门面,对应 Java `com.alibaba.qlexpress4.Express4Runner`。
//!
//! 职责:脚本执行(`execute` 系列)、编译(`parse_to_syntax_tree`/
//! `parse_to_instructions`/`parse_to_definition_with_cache`)、函数/宏/
//! 编译期函数/操作符注册、外部变量与函数收集、安全策略与 parse cache
//! 接线。
//!
//! Rust 适配要点(对照 Java 逐条):
//! - Java 的编译缓存为 `ConcurrentHashMap<String, Future<QCompileCache>>`
//!   (并发去重编译);Rust 单线程 `Rc` 体系使用不淘汰的兼容缓存，命中语义
//!   一致(同一 script 只编译一次)。`execute_checked` 另用物理隔离的按租户
//!   有界 LRU，不能淘汰或污染兼容缓存。
//!   多线程宿主使用 `ConcurrentParseCache` 共享纯数据编译产物，每个工作
//!   线程持有本地 Runner，避免把 `Rc/RefCell` 错误标记为 `Send`。
//! - Java `Map<String, Object>` 上下文 → Rust `HashMap<String, DataValue>`
//!   (值已是脚本值,无需再装箱)。
//! - Java 反射式 API(`addFunctionOfServiceMethod` 的方法查找、
//!   `addObjFunction`/`addStaticFunction` 的注解扫描)按 SPEC §4 改为
//!   显式传入 `IMethod`/`NativeMethod`(见各方法文档)。
//! - 表达式 trace:编译期由 `TraceExpressionVisitor` 生成静态点树，
//!   执行期在初始化选项和执行选项同时开启时创建本次执行专属的值树。

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Instant;

use crate::annotation::ql_function_method::QLFunctionMethod;
use crate::annotation::ql_function_provider::QLFunctionProvider;
use crate::aparser::check_visitor::CheckVisitor;
use crate::aparser::compile_cache::QCompileCache;
use crate::aparser::compile_cache_store::CompileCacheStore;
use crate::aparser::compile_time_function::CompileTimeFunction;
use crate::aparser::generator_scope::GeneratorScope;
use crate::aparser::import_manager::ImportManager;
use crate::aparser::macro_define::MacroDefine;
use crate::aparser::out_function_visitor::OutFunctionVisitor;
use crate::aparser::out_var_attrs_visitor::OutVarAttrsVisitor;
use crate::aparser::out_var_names_visitor::OutVarNamesVisitor;
use crate::aparser::qlparser::build_tree;
use crate::aparser::qvm_instruction_visitor::{
    compile_script, CompileTimeFunctions, Context as VisitorContext, InstructionScope,
    QvmInstructionVisitor, UserDefineFunctions,
};
use crate::aparser::syntax_tree_factory::Node;
use crate::aparser::trace_expression_visitor::TraceExpressionVisitor;
use crate::api::batch_add_function_result::BatchAddFunctionResult;
use crate::api::parsecache::serializable_parse_cache_exporter::SerializableParseCacheExporter;
use crate::api::parsecache::serializable_parse_cache_importer::{
    ImportResult, SerializableParseCacheImporter,
};
use crate::api::parsecache::{LoadedCompileCache, LoadedParseCache, SerializableParseCache};
use crate::api::ql_functional_varargs::QLFunctionalVarargs;
use crate::check_options::CheckOptions;
use crate::exception::ql_syntax_exception::QLSyntaxException;
use crate::exception::QLException;
use crate::init_options::InitOptions;
use crate::ql_options::QLOptions;
use crate::ql_precedences;
use crate::ql_result::QLResult;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::context::{
    ExpressContext, MapExpressContext, ObjectFieldExpressContext, QLAliasContext,
};
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::delegate_qcontext::DelegateQContext;
use crate::runtime::function::{CustomFunction, ExtensionFunction, QMethodFunction};
use crate::runtime::i_method::IMethod;
use crate::runtime::instruction::Instruction;
use crate::runtime::jvm_i_method::NativeIMethod;
use crate::runtime::member::NativeRegistry;
use crate::runtime::native_type::NativeType;
use crate::runtime::operator::custom_binary_operator::CustomBinaryOperator;
use crate::runtime::operator::operator_manager::OperatorManager;
use crate::runtime::parameters::Parameters;
use crate::runtime::q_runtime::QRuntime;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::qlambda_definition_inner::QLambdaDefinitionInner;
use crate::runtime::qlambda_trace::QLambdaTrace;
use crate::runtime::qvm_global_scope::QvmGlobalScope;
use crate::runtime::qvm_runtime::{current_time_millis, QvmRuntime};
use crate::runtime::reflect_loader::ReflectLoader;
use crate::runtime::scope::QScope;
use crate::runtime::trace::{ExpressionTrace, QTraces, TracePointTree};
use crate::runtime::value::{DataValue, QValue};
use crate::security::ql_security_strategy::QLSecurityStrategy;
use crate::security::{Capability, SandboxProfile};
use crate::utils::ql_function_util::QLFunctionUtil;

/// runner 身份令牌分配器(Java 以 `this` 引用相等判断 `LoadedParseCache`
/// 绑定关系;Rust 为每个 runner 分配唯一序号,见 [`LoadedParseCache`])。
static RUNNER_IDENTITY: AtomicUsize = AtomicUsize::new(1);
const JAVA_COMPATIBLE_CACHE_TENANT: &str = "__java_compatible__";
const UNBOUNDED_CACHE_CAPACITY: usize = usize::MAX;

/// Java `addExtendFunction(String, Class, QLFunctionalVarargs)` 中匿名
/// `ExtensionFunction` 的 Rust 对等适配器。
struct VarargsExtensionFunction<F> {
    name: String,
    binding_class: ClassRef,
    functional_varargs: F,
}

impl<F> ExtensionFunction for VarargsExtensionFunction<F>
where
    F: QLFunctionalVarargs,
{
    fn parameter_types(&self) -> Vec<ClassRef> {
        vec![ClassRef::array_of(ClassRef::Named(
            "java.lang.Object".to_string(),
        ))]
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn declaring_class(&self) -> ClassRef {
        self.binding_class.clone()
    }

    fn is_var_args(&self) -> bool {
        // Java 匿名 ExtensionFunction 明确覆盖 isVarArgs() 并返回 true；
        // 该元数据决定标准 IMethod 参数转换是否把多个脚本实参打包为 Object[]。
        true
    }

    fn invoke(
        &self,
        object: &DataValue,
        arguments: &[DataValue],
    ) -> Result<DataValue, QLException> {
        let [DataValue::Array(var_args)] = arguments else {
            return Err(QLException::for_test(
                crate::exception::QLExceptionKind::Runtime,
                format!("invoke method '{}' with wrong arguments", self.name),
                crate::exception::error_codes::INVOKE_METHOD_WITH_WRONG_ARGUMENTS,
            ));
        };
        let var_args = var_args.borrow();
        let mut extension_arguments = Vec::with_capacity(var_args.len() + 1);
        extension_arguments.push(object.clone());
        extension_arguments.extend(var_args.iter().cloned());
        self.functional_varargs.call(&extension_arguments)
    }
}

/// QlExpress Rust 的解析、编译、执行与宿主扩展统一门面。
///
/// 普通 `execute` 系列保持 Java QLExpress4 的兼容默认值；
/// [`Express4Runner::execute_checked`] 为 Rust 增加静态检查、有限预算、
/// capability 白名单和租户缓存隔离。字段与 Java 一一对应，见各字段注释。
/// 对应 Java: com.alibaba.qlexpress4.Express4Runner。
///
/// # Thread safety: intentionally not `Send` or `Sync`
///
/// `Express4Runner` uses [`Rc`] and [`RefCell`] internally (compile cache,
/// user-defined functions, global scope, registered capabilities). It is
/// **intentionally** neither [`Send`] nor [`Sync`].
///
/// The design model is **one runner per worker thread**: create and configure
/// an `Express4Runner` on the thread that will use it, then reuse that
/// instance for all script evaluations on that thread. Sharing or moving a
/// runner across thread boundaries will produce a compile-time error. This
/// is a deliberate safety guarantee, not a limitation — it lets the engine
/// avoid the overhead of `Arc<Mutex<...>>` while still preventing data
/// races at compile time.
///
/// Multi-threaded hosts should create one runner per worker thread.
/// Compilation artifacts can be shared across threads via
/// [`ConcurrentParseCache`](crate::api::parsecache::ConcurrentParseCache)
/// (pure data, no runtime state).
///
/// The following `compile_fail` doctests verify the non-`Send`/non-`Sync`
/// contracts at compile time:
///
/// ```compile_fail
/// use qlexpress::Express4Runner;
///
/// fn assert_send<T: Send>(_: &T) {}
///
/// let runner = Express4Runner::new();
/// assert_send(&runner); // Express4Runner is intentionally not Send
/// ```
///
/// ```compile_fail
/// use qlexpress::Express4Runner;
///
/// fn assert_sync<T: Sync>(_: &T) {}
///
/// let runner = Express4Runner::new();
/// assert_sync(&runner); // Express4Runner is intentionally not Sync
/// ```
pub struct Express4Runner {
    /// 操作符管理器。对应 Java 字段 `operatorManager`。
    operator_manager: OperatorManager,
    /// Java 兼容编译缓存(script → 编译产物)。对应 Java 字段
    /// `compileCache`；与 Java 的无界 `ConcurrentHashMap` 一样，在显式
    /// 清理前不淘汰条目。
    compile_cache: RefCell<CompileCacheStore>,
    /// `execute_checked` 专用的按租户有界 LRU；与 Java 兼容缓存物理隔离，
    /// 防止安全租户淘汰普通执行入口已经冻结的编译语义。
    secure_compile_cache: RefCell<CompileCacheStore>,
    /// 用户注册函数表。对应 Java 字段 `userDefineFunction`；共享给已创建
    /// Lambda 的全局作用域，使后续注册与 Java Map 引用语义一致。
    user_define_functions: Rc<RefCell<UserDefineFunctions>>,
    /// 编译期函数表。对应 Java 字段 `compileTimeFunctions`。
    compile_time_functions: RefCell<CompileTimeFunctions>,
    /// 全局宏作用域。对应 Java 字段 `globalScope`。
    global_scope: Rc<InstructionScope>,
    /// 原生类型/成员注册表。对应 Java 字段 `reflectLoader`
    /// (SPEC §4:显式注册表替代反射 + 安全策略检查点)。
    reflect_loader: ReflectLoader,
    /// 初始化选项。对应 Java 字段 `initOptions`。
    init_options: InitOptions,
    /// 宿主显式注册的扩展能力集合，供 `execute_checked` 统一白名单审计。
    registered_capabilities: RefCell<HashSet<Capability>>,
    /// runner 身份令牌(Java `this` 引用,用于 parse cache 绑定校验)。
    identity: usize,
}

impl Default for Express4Runner {
    /// 对应 Java `new Express4Runner(InitOptions.DEFAULT_OPTIONS)` 的
    /// 无参便捷构造。
    fn default() -> Self {
        Express4Runner::new()
    }
}

include!("express4_runner/execution.rs");
include!("express4_runner/compilation.rs");
include!("express4_runner/functions.rs");
include!("express4_runner/extensions.rs");
include!("express4_runner/security_and_analysis.rs");

/// `HashMap<String, DataValue>` → 脚本上下文的有序 map
/// (Java `new MapExpressContext(Map<String, Object>)`)。
fn map_to_index_map(context: HashMap<String, DataValue>) -> Rc<RefCell<IndexMap>> {
    let entries = context
        .into_iter()
        .map(|(key, value)| (DataValue::string(key), value))
        .collect();
    Rc::new(RefCell::new(IndexMap::from_entries(entries)))
}

fn sandbox_limit_error(code: &'static str, actual: usize, limit: usize) -> QLException {
    crate::runtime::execution_budget::budget_error(
        crate::exception::QLExceptionKind::Runtime,
        code,
        format!("sandbox limit exceeded: actual {actual}, limit {limit}"),
    )
}

fn validate_token_nesting(
    tokens: &[crate::aparser::token::Token],
    max_depth: usize,
) -> Result<(), QLException> {
    use crate::aparser::token;

    let mut depth = 0usize;
    let mut recursive_expression_ops = 0usize;
    for item in tokens {
        match item.token_type() {
            value
                if value == token::LPAREN as i32
                    || value == token::LBRACE as i32
                    || value == token::LBRACK as i32 =>
            {
                depth = depth.saturating_add(1);
                if depth > max_depth {
                    return Err(sandbox_limit_error(
                        "SANDBOX_AST_DEPTH_EXCEEDED",
                        depth,
                        max_depth,
                    ));
                }
            }
            value
                if value == token::RPAREN as i32
                    || value == token::RBRACE as i32
                    || value == token::RBRACK as i32 =>
            {
                depth = depth.saturating_sub(1);
            }
            value
                if value == token::EQ as i32
                    || value == token::QUESTION as i32
                    || value == token::ARROW as i32
                    || value == token::RIGHSHIFT_ASSGIN as i32
                    || value == token::URSHIFT_ASSGIN as i32
                    || value == token::LSHIFT_ASSGIN as i32
                    || value == token::ADD_ASSIGN as i32
                    || value == token::SUB_ASSIGN as i32
                    || value == token::AND_ASSIGN as i32
                    || value == token::OR_ASSIGN as i32
                    || value == token::MUL_ASSIGN as i32
                    || value == token::MOD_ASSIGN as i32
                    || value == token::DIV_ASSIGN as i32
                    || value == token::XOR_ASSIGN as i32 =>
            {
                recursive_expression_ops = recursive_expression_ops.saturating_add(1);
                if recursive_expression_ops > max_depth {
                    return Err(sandbox_limit_error(
                        "SANDBOX_AST_DEPTH_EXCEEDED",
                        recursive_expression_ops,
                        max_depth,
                    ));
                }
            }
            value if value == token::SEMI as i32 || value == token::NEWLINE as i32 => {
                recursive_expression_ops = 0;
            }
            _ => {}
        }
    }
    Ok(())
}

fn validate_ast_budget(tree: &Node, sandbox_profile: &SandboxProfile) -> Result<(), QLException> {
    use crate::aparser::rule_context::ChildRef;

    let mut node_count = 0usize;
    let mut stack = vec![(tree, 1usize)];
    while let Some((node, depth)) = stack.pop() {
        node_count = node_count.saturating_add(1);
        if node_count > sandbox_profile.limits.max_ast_nodes {
            return Err(sandbox_limit_error(
                "SANDBOX_AST_NODES_EXCEEDED",
                node_count,
                sandbox_profile.limits.max_ast_nodes,
            ));
        }
        if depth > sandbox_profile.limits.max_ast_depth {
            return Err(sandbox_limit_error(
                "SANDBOX_AST_DEPTH_EXCEEDED",
                depth,
                sandbox_profile.limits.max_ast_depth,
            ));
        }
        for child in node.children() {
            if let ChildRef::Node(child) = child {
                stack.push((child, depth.saturating_add(1)));
            }
        }
    }
    Ok(())
}

/// 模板包成动态字符串字面量。对应 Java 私有方法
/// `wrapAsDynamicString(String)`(`null` → `""`,转义双引号)。
fn wrap_as_dynamic_string(template: &str) -> String {
    format!("\"{}\"", template.replace('\"', "\\\""))
}

#[cfg(test)]
mod varargs_extension_function_tests {
    use super::*;
    use crate::runtime::i_method::IMethod;

    /// SOURCE_PARITY: Java `Express4Runner#addExtendFunction(String, Class,
    /// QLFunctionalVarargs)` 匿名类的五个覆盖方法必须共同保留：Object[]
    /// 签名、varargs=true、名称、声明类，以及 receiver 位于展开参数第 0 位。
    #[test]
    fn java_anonymous_varargs_extension_contract() {
        let extension = VarargsExtensionFunction {
            name: "describe".to_string(),
            binding_class: ClassRef::Named("java.lang.String".to_string()),
            functional_varargs: |arguments: &[DataValue]| {
                Ok(DataValue::string(
                    arguments
                        .iter()
                        .map(DataValue::string_value_of)
                        .collect::<Vec<_>>()
                        .join("|"),
                ))
            },
        };

        assert_eq!(
            ExtensionFunction::parameter_types(&extension),
            vec![ClassRef::array_of(ClassRef::Named(
                "java.lang.Object".to_string()
            ))]
        );
        assert!(ExtensionFunction::is_var_args(&extension));
        assert!(IMethod::is_var_args(&extension));
        assert_eq!(ExtensionFunction::name(&extension), "describe");
        assert_eq!(
            ExtensionFunction::declaring_class(&extension),
            ClassRef::Named("java.lang.String".to_string())
        );
        assert_eq!(
            ExtensionFunction::invoke(
                &extension,
                &DataValue::string("root"),
                &[DataValue::array(vec![
                    DataValue::Int(1),
                    DataValue::string("leaf")
                ])],
            )
            .expect("invoke varargs extension"),
            DataValue::string("root|1|leaf")
        );
    }
}
