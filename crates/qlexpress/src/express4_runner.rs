//! 引擎门面,对应 Java `com.alibaba.qlexpress4.Express4Runner`。
//!
//! 职责:脚本执行(`execute` 系列)、编译(`parse_to_syntax_tree`/
//! `parse_to_instructions`/`parse_to_definition_with_cache`)、函数/宏/
//! 编译期函数/操作符注册、外部变量与函数收集、安全策略与 parse cache
//! 接线。
//!
//! Rust 适配要点(对照 Java 逐条):
//! - Java 的编译缓存为 `ConcurrentHashMap<String, Future<QCompileCache>>`
//!   (并发去重编译);Rust 单线程 `Rc` 体系下退化为
//!   `RefCell<HashMap<String, Rc<_>>>`,命中语义一致(同一 script 只编译一次)。
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
use crate::runtime::function::{
    as_native_method, CustomFunction, ExtensionFunction, QMethodFunction,
};
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
use crate::runtime::qvm_global_scope::QvmGlobalScope;
use crate::runtime::qvm_runtime::{current_time_millis, QvmRuntime};
use crate::runtime::reflect_loader::ReflectLoader;
use crate::runtime::scope::QScope;
use crate::runtime::trace::{ExpressionTrace, QTraces, TracePointTree};
use crate::runtime::value::{DataValue, QValue};
use crate::security::ql_security_strategy::QLSecurityStrategy;
use crate::security::{Capability, SandboxProfile};

/// runner 身份令牌分配器(Java 以 `this` 引用相等判断 `LoadedParseCache`
/// 绑定关系;Rust 为每个 runner 分配唯一序号,见 [`LoadedParseCache`])。
static RUNNER_IDENTITY: AtomicUsize = AtomicUsize::new(1);

/// QlExpress Rust 的解析、编译、执行与宿主扩展统一门面。
///
/// 普通 `execute` 系列保持 Java QLExpress4 的兼容默认值；
/// [`Express4Runner::execute_checked`] 为 Rust 增加静态检查、有限预算、
/// capability 白名单和租户缓存隔离。字段与 Java 一一对应，见各字段注释。
/// 对应 Java: com.alibaba.qlexpress4.Express4Runner。
pub struct Express4Runner {
    /// 操作符管理器。对应 Java 字段 `operatorManager`。
    operator_manager: OperatorManager,
    /// 编译缓存(script → 编译产物)。对应 Java 字段 `compileCache`
    /// (Java 值为 `Future<QCompileCache>`,Rust 直接缓存产物,见文件头)。
    compile_cache: RefCell<CompileCacheStore>,
    /// 用户注册函数表。对应 Java 字段 `userDefineFunction`。
    user_define_functions: RefCell<UserDefineFunctions>,
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

impl Express4Runner {
    /// 无参构造(默认 `InitOptions`)。对应 Java 以默认 `InitOptions`
    /// 构造 `Express4Runner` 的用法。
    ///
    /// # Returns
    ///
    /// 返回采用 Java 兼容初始化选项和默认隔离成员策略的新 runner。
    pub fn new() -> Self {
        Self::with_init_options(InitOptions::default())
    }

    /// 对应 Java 构造器 `Express4Runner(InitOptions)`:
    /// 按 `InitOptions.securityStrategy` 接线注册表安全检查
    /// (Java: `new ReflectLoader(securityStrategy, allowPrivateAccess)`)。
    ///
    /// # Arguments
    ///
    /// * `init_options` - runner 生命周期内固定的解析、调试和原生成员策略。
    ///
    /// # Returns
    ///
    /// 返回拥有独立函数表、宏作用域和编译缓存的 runner。
    pub fn with_init_options(init_options: InitOptions) -> Self {
        let reflect_loader = ReflectLoader::new(
            init_options.security_strategy().clone(),
            init_options.is_allow_private_access(),
        );
        Express4Runner {
            operator_manager: OperatorManager::new(),
            compile_cache: RefCell::new(CompileCacheStore::new()),
            user_define_functions: RefCell::new(HashMap::new()),
            compile_time_functions: RefCell::new(HashMap::new()),
            global_scope: Rc::new(GeneratorScope::new("global", None)),
            reflect_loader,
            init_options,
            registered_capabilities: RefCell::new(HashSet::new()),
            identity: RUNNER_IDENTITY.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// runner 身份令牌(Java `this`)。
    /// 对应 Java: com.alibaba.qlexpress4.Express4Runner#identity。
    pub fn identity(&self) -> usize {
        self.identity
    }

    /// 取注册表(只读)。对应 Java 内部对 `reflectLoader` 的访问。
    pub fn registry(&self) -> &Rc<NativeRegistry> {
        self.reflect_loader.registry()
    }

    /// 注册表可变访问(`&mut self` 且注册表未被 QVM 共享时)。
    fn registry_mut(&mut self) -> &mut NativeRegistry {
        self.reflect_loader
            .registry_mut()
            .expect("Express4Runner registry must not be shared outside execute")
    }

    // ------------------------------------------------------------------
    // 执行(Java execute 系列)
    // ------------------------------------------------------------------

    /// 以 Map 上下文执行脚本。对应 Java 方法
    /// `execute(String script, Map<String, Object> context, QLOptions)`:
    /// map 的 key 即脚本引用的变量名(内部包装为 `MapExpressContext`)。
    ///
    /// # Arguments
    ///
    /// * `script` - 待解析和执行的 QLExpress 脚本。
    /// * `context` - 以变量名为键的外部数据；是否写回由 `ql_options` 决定。
    /// * `ql_options` - 本次执行的 Java 兼容选项。
    ///
    /// # Returns
    ///
    /// 返回脚本结果以及可选表达式追踪。
    ///
    /// # Errors
    ///
    /// 词法、语法、编译或 QVM 执行失败时返回 [`QLException`]。
    pub fn execute(
        &self,
        script: &str,
        context: HashMap<String, DataValue>,
        ql_options: &QLOptions,
    ) -> Result<QLResult, QLException> {
        self.execute_with_context(
            script,
            Rc::new(MapExpressContext::new(map_to_index_map(context))),
            ql_options,
        )
    }

    /// 执行模板字符串(包成动态字符串字面量)。对应 Java 方法
    /// `executeTemplate(String, Map, QLOptions)`;模板内不支持换行。
    ///
    /// # Arguments
    ///
    /// * `template` - 包含 QLExpress 插值选择器的单行模板。
    /// * `context` - 模板插值可读取的外部变量。
    /// * `ql_options` - 本次执行选项。
    ///
    /// # Returns
    ///
    /// 返回完成插值后的字符串结果。
    ///
    /// # Errors
    ///
    /// 模板包装、解析或执行失败时返回 [`QLException`]。
    pub fn execute_template(
        &self,
        template: &str,
        context: HashMap<String, DataValue>,
        ql_options: &QLOptions,
    ) -> Result<QLResult, QLException> {
        let script = wrap_as_dynamic_string(template);
        self.execute(&script, context, ql_options)
    }

    /// 以 [`ExpressContext`] 上下文执行脚本。对应 Java 方法
    /// `execute(String script, ExpressContext context, QLOptions)`。
    ///
    /// # Arguments
    ///
    /// * `script` - 待执行脚本。
    /// * `context` - 可自定义变量读取和写回行为的上下文。
    /// * `ql_options` - 本次执行选项。
    ///
    /// # Returns
    ///
    /// 返回脚本结果以及可选表达式追踪。
    ///
    /// # Errors
    ///
    /// 解析、编译、上下文访问或 QVM 执行失败时返回 [`QLException`]。
    pub fn execute_with_context(
        &self,
        script: &str,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<QLResult, QLException> {
        // Java: qlOptions.isCache() ? parseToDefinitionWithCache : parseDefinition
        let compile_cache = if ql_options.is_cache() {
            self.parse_to_definition_with_cache(script)
        } else {
            self.parse_definition(script).map(Rc::new)
        }
        .map_err(QLSyntaxException::into_exception)?;
        self.execute_definition(&compile_cache, true, context, ql_options)
    }

    /// 使用有限预算、静态检查、统一 capability 白名单和租户缓存执行脚本。
    ///
    /// 该入口是 Rust 安全增强；原 `execute` 系列继续保持 Java 兼容默认值。
    /// 对不可信脚本，宿主应只暴露本方法或进程级 Worker API。
    ///
    /// # Arguments
    ///
    /// * `script` - 来源不可信或由业务用户配置的规则脚本。
    /// * `context` - 执行前会进行字符串、集合和输出规模校验的外部数据。
    /// * `ql_options` - Java 兼容执行行为；安全上限由 `sandbox_profile` 决定。
    /// * `sandbox_profile` - 静态检查、资源预算、能力白名单和租户缓存策略。
    ///
    /// # Returns
    ///
    /// 全部检查通过并执行完成时返回有界的 QL 结果。
    ///
    /// # Errors
    ///
    /// 配置无界、静态检查失败、能力未授权、任一资源预算耗尽、取消、超时、
    /// 编译或 QVM 执行失败时返回带稳定错误码的 [`QLException`]。
    pub fn execute_checked(
        &self,
        script: &str,
        context: HashMap<String, DataValue>,
        ql_options: &QLOptions,
        sandbox_profile: &SandboxProfile,
    ) -> Result<QLResult, QLException> {
        let validation_budget = crate::runtime::execution_budget::ExecutionBudget::new(
            sandbox_profile.limits.clone(),
            sandbox_profile.cancellation_token.clone(),
        );
        for value in context.values() {
            validation_budget.charge_external_value(value)?;
        }
        self.execute_checked_with_context(
            script,
            Rc::new(MapExpressContext::new(map_to_index_map(context))),
            ql_options,
            sandbox_profile,
        )
    }

    /// `execute_checked` 的 [`ExpressContext`] 版本。
    ///
    /// 动态上下文返回的值会在进入 QVM 栈后受字符串与集合预算校验。
    ///
    /// # Arguments
    ///
    /// * `script` - 来源不可信的规则脚本。
    /// * `context` - 动态变量上下文；返回值在进入 QVM 后继续接受预算校验。
    /// * `ql_options` - Java 兼容执行行为。
    /// * `sandbox_profile` - 不可绕过的安全检查与资源限制。
    ///
    /// # Returns
    ///
    /// 返回通过静态与运行时安全检查的有界结果。
    ///
    /// # Errors
    ///
    /// 与 [`Express4Runner::execute_checked`] 相同；此外，无界表达式追踪配置会
    /// 以 `SANDBOX_TRACE_DISABLED` 被拒绝。
    pub fn execute_checked_with_context(
        &self,
        script: &str,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
        sandbox_profile: &SandboxProfile,
    ) -> Result<QLResult, QLException> {
        sandbox_profile.validate().map_err(|reason| {
            crate::runtime::execution_budget::budget_error(
                crate::exception::QLExceptionKind::Runtime,
                "SANDBOX_INVALID_PROFILE",
                reason,
            )
        })?;
        if self.init_options.is_trace_expression() && ql_options.is_trace_expression() {
            return Err(crate::runtime::execution_budget::budget_error(
                crate::exception::QLExceptionKind::Runtime,
                "SANDBOX_TRACE_DISABLED",
                "execute_checked disables expression tracing because trace retention is not bounded",
            ));
        }
        let started = Instant::now();
        self.validate_capabilities(sandbox_profile)?;
        self.validate_source_budget(script, sandbox_profile)?;

        let tokens = crate::aparser::qlexer::tokenize_with_limit(
            script,
            Some(&self.operator_manager),
            self.init_options.interpolation_mode(),
            self.init_options.selector_start(),
            self.init_options.selector_end(),
            self.init_options.is_strict_new_lines(),
            Some(sandbox_profile.limits.max_tokens),
        )
        .map_err(QLSyntaxException::into_exception)?;
        if tokens.len() > sandbox_profile.limits.max_tokens {
            return Err(sandbox_limit_error(
                "SANDBOX_TOKENS_EXCEEDED",
                tokens.len(),
                sandbox_profile.limits.max_tokens,
            ));
        }
        validate_token_nesting(&tokens, sandbox_profile.limits.max_ast_depth)?;
        self.check_sandbox_deadline(started, sandbox_profile)?;

        let consumer = Rc::clone(self.init_options.debug_info_consumer());
        let tree = crate::aparser::qlparser::build_tree_from_tokens(
            script,
            &tokens,
            Some(&self.operator_manager),
            self.init_options.is_debug(),
            move |info| consumer(info),
            self.init_options.is_strict_new_lines(),
        )
        .map_err(QLSyntaxException::into_exception)?;
        validate_ast_budget(&tree, sandbox_profile)?;
        let mut check_visitor = CheckVisitor::new(&sandbox_profile.check_options, script);
        check_visitor
            .check(&tree)
            .map_err(QLSyntaxException::into_exception)?;
        self.check_sandbox_deadline(started, sandbox_profile)?;

        let compile_cache = if sandbox_profile.compile_cache.enabled {
            let cached = {
                self.compile_cache
                    .borrow_mut()
                    .get(&sandbox_profile.tenant_id, script)
            };
            if let Some(cached) = cached {
                cached
            } else {
                let compiled = Rc::new(
                    self.compile_tree(script, &tree)
                        .map_err(QLSyntaxException::into_exception)?,
                );
                self.validate_instruction_budget(&compiled, sandbox_profile)?;
                self.compile_cache.borrow_mut().insert(
                    &sandbox_profile.tenant_id,
                    script.to_string(),
                    Rc::clone(&compiled),
                    sandbox_profile.compile_cache.max_entries,
                    sandbox_profile.compile_cache.max_entries_per_tenant,
                );
                compiled
            }
        } else {
            let compiled = Rc::new(
                self.compile_tree(script, &tree)
                    .map_err(QLSyntaxException::into_exception)?,
            );
            self.validate_instruction_budget(&compiled, sandbox_profile)?;
            compiled
        };
        self.validate_instruction_budget(&compile_cache, sandbox_profile)?;
        self.check_sandbox_deadline(started, sandbox_profile)?;

        let elapsed_millis = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        let mut runtime_limits = sandbox_profile.limits.clone();
        runtime_limits.timeout_millis = runtime_limits
            .timeout_millis
            .saturating_sub(elapsed_millis)
            .max(1);
        let mut runtime_profile = sandbox_profile.clone();
        runtime_profile.limits = runtime_limits;
        self.execute_definition_sandboxed(
            &compile_cache,
            true,
            context,
            ql_options,
            &runtime_profile,
        )
    }

    /// 以宿主对象的公开字段/getter 作为外部变量执行脚本。对应 Java
    /// `execute(String, Object, QLOptions)` 与 `ObjectFieldExpressContext`。
    pub fn execute_with_object(
        &self,
        script: &str,
        object: DataValue,
        ql_options: &QLOptions,
    ) -> Result<QLResult, QLException> {
        self.execute_with_context(
            script,
            Rc::new(ObjectFieldExpressContext::new(
                object,
                Rc::clone(self.reflect_loader.registry()),
            )),
            ql_options,
        )
    }

    /// 以显式别名对象作为外部变量执行脚本。对应 Java
    /// `executeWithAliasObjects`；Rust 无运行时注解扫描，调用方传入
    /// `(别名列表, 对象值)`，其余上下文与执行语义完全一致。
    pub fn execute_with_alias_values(
        &self,
        script: &str,
        ql_options: &QLOptions,
        aliased_values: &[(&[&str], DataValue)],
    ) -> Result<QLResult, QLException> {
        self.execute_with_context(
            script,
            Rc::new(QLAliasContext::new(aliased_values)),
            ql_options,
        )
    }

    /// 执行已加载的 parse cache。对应 Java 方法
    /// `execute(LoadedParseCache, ExpressContext, QLOptions)`:
    /// 绑定到其它 runner 的 cache 抛出
    /// `SERIALIZABLE_PARSE_CACHE_INVALID_MODEL`。
    pub fn execute_with_loaded_cache(
        &self,
        cache: &LoadedParseCache,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<QLResult, QLException> {
        if !cache.is_bound_to(self.identity) {
            return Err(QLException::for_test(
                crate::exception::ql_exception::QLExceptionKind::Runtime,
                format!(
                    "LoadedParseCache is bound to another Express4Runner: {}",
                    cache.get_script().unwrap_or_default()
                ),
                crate::exception::error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL,
            ));
        }
        self.execute_definition(
            cache.get_compile_cache(),
            cache.has_trace_points(),
            context,
            ql_options,
        )
    }

    /// 加载并执行可序列化 parse cache。对应 Java 方法
    /// `execute(SerializableParseCache, ExpressContext, QLOptions)`。
    pub fn execute_with_cache(
        &self,
        cache: &SerializableParseCache,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<QLResult, QLException> {
        let loaded = self
            .import_parse_cache(cache)
            .map_err(|err| err.into_ql_exception())?;
        self.execute_with_loaded_cache(&loaded, context, ql_options)
    }

    /// 编译并执行,同时返回主 Lambda 的指令序列(调试用途)。
    /// SPEC §3.6 契约名 `execute_to_last_instruction`;现版 Java
    /// `Express4Runner` 无同名方法(旧版调试入口),Rust 以
    /// 「执行 + 返回指令」对齐其语义。
    /// 对应 Java: com.alibaba.qlexpress4.Express4Runner#executeToLastInstruction。
    pub fn execute_to_last_instruction(
        &self,
        script: &str,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<(QLResult, Vec<Instruction>), QLException> {
        let instructions = self
            .parse_to_instructions(script)
            .map_err(QLSyntaxException::into_exception)?;
        let compile_cache = self
            .parse_definition(script)
            .map_err(QLSyntaxException::into_exception)?;
        let result = self.execute_definition(&compile_cache, true, context, ql_options)?;
        Ok((result, instructions))
    }

    /// 编译产物驱动一次执行,对应 Java 私有方法
    /// `executeLambdaTrace(QLambdaTrace)`(含 debug 计时输出)。
    fn execute_definition(
        &self,
        compile_cache: &LoadedCompileCache,
        trace_points_available: bool,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<QLResult, QLException> {
        let debug = self.init_options.is_debug();
        let start = current_time_millis();
        let global_scope = QvmGlobalScope::with_context(
            context,
            self.user_define_functions.borrow().clone(),
            ql_options.attachments().clone(),
            ql_options.is_pollute_user_context(),
        );
        let traces = if self.init_options.is_trace_expression()
            && ql_options.is_trace_expression()
            && trace_points_available
        {
            QTraces::from_trace_points(compile_cache.expression_trace_points())
        } else {
            QTraces::empty()
        };
        let runtime = Rc::new(QvmRuntime::new(
            traces,
            ql_options.attachments().clone(),
            Rc::clone(self.reflect_loader.registry()),
            current_time_millis(),
        ));
        let definition = Rc::clone(compile_cache.q_lambda_definition());
        let result = runtime.execute(global_scope, definition, ql_options)?;
        if debug {
            (self.init_options.debug_info_consumer())(format!(
                "Execute consume time: {} ms",
                current_time_millis() - start
            ));
        }
        // Java: new QLResult(result, traces.getExpressionTraces())。
        Ok(QLResult::new(result.value(), runtime.traces().snapshot()))
    }

    fn execute_definition_sandboxed(
        &self,
        compile_cache: &LoadedCompileCache,
        trace_points_available: bool,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
        sandbox_profile: &SandboxProfile,
    ) -> Result<QLResult, QLException> {
        let global_scope = QvmGlobalScope::with_context(
            context,
            self.user_define_functions.borrow().clone(),
            ql_options.attachments().clone(),
            ql_options.is_pollute_user_context(),
        );
        let traces = if self.init_options.is_trace_expression()
            && ql_options.is_trace_expression()
            && trace_points_available
        {
            QTraces::from_trace_points(compile_cache.expression_trace_points())
        } else {
            QTraces::empty()
        };
        let runtime = Rc::new(QvmRuntime::new_sandboxed(
            traces,
            ql_options.attachments().clone(),
            Rc::clone(self.reflect_loader.registry()),
            current_time_millis(),
            sandbox_profile.limits.clone(),
            sandbox_profile.cancellation_token.clone(),
            sandbox_profile.capability_policy.clone(),
        ));
        let definition = Rc::clone(compile_cache.q_lambda_definition());
        let result = runtime.execute(global_scope, definition, ql_options)?;
        let value = result.value();
        if let Some(budget) = runtime.execution_budget() {
            budget.validate_output(&value)?;
        }
        Ok(QLResult::new(value, runtime.traces().snapshot()))
    }

    // ------------------------------------------------------------------
    // 编译(Java parseTo* 系列)
    // ------------------------------------------------------------------

    /// 解析脚本为语法树。对应 Java 方法 `parseToSyntaxTree(String)`。
    pub fn parse_to_syntax_tree(&self, script: &str) -> Result<Node, QLSyntaxException> {
        let debug = self.init_options.is_debug();
        let consumer = Rc::clone(self.init_options.debug_info_consumer());
        build_tree(
            script,
            Some(&self.operator_manager),
            debug,
            move |info| consumer(info),
            self.init_options.interpolation_mode(),
            self.init_options.selector_start(),
            self.init_options.selector_end(),
            self.init_options.is_strict_new_lines(),
        )
    }

    /// 解析并编译脚本,返回主 Lambda 指令序列(Stage 3b 编译入口接线)。
    /// 对应 Java `parseDefinition` 的指令部分。
    pub fn parse_to_instructions(
        &self,
        script: &str,
    ) -> Result<Vec<Instruction>, QLSyntaxException> {
        let tree = self.parse_to_syntax_tree(script)?;
        let import_manager = RefCell::new(self.inherit_default_import());
        let user_define_functions = self.user_define_functions.borrow();
        let compile_time_functions = self.compile_time_functions.borrow();
        let (instructions, _max_stack) = compile_script(
            script,
            &tree,
            &import_manager,
            Some(Rc::clone(&self.global_scope)),
            &self.operator_manager,
            &compile_time_functions,
            &user_define_functions,
            &self.init_options,
        )?;
        Ok(instructions)
    }

    /// 编译脚本为可缓存的编译产物(不走缓存)。对应 Java 私有方法
    /// `parseDefinition(String)`。
    pub fn parse_definition(&self, script: &str) -> Result<LoadedCompileCache, QLSyntaxException> {
        let debug = self.init_options.is_debug();
        let start = current_time_millis();
        let tree = self.parse_to_syntax_tree(script)?;
        let compiled = self.compile_tree(script, &tree)?;
        if debug {
            (self.init_options.debug_info_consumer())(format!(
                "Compile consume time: {} ms",
                current_time_millis() - start
            ));
        }
        Ok(compiled)
    }

    fn compile_tree(
        &self,
        script: &str,
        tree: &Node,
    ) -> Result<LoadedCompileCache, QLSyntaxException> {
        let import_manager = RefCell::new(self.inherit_default_import());
        let user_define_functions = self.user_define_functions.borrow();
        let compile_time_functions = self.compile_time_functions.borrow();
        let (instructions, max_stack) = compile_script(
            script,
            tree,
            &import_manager,
            Some(Rc::clone(&self.global_scope)),
            &self.operator_manager,
            &compile_time_functions,
            &user_define_functions,
            &self.init_options,
        )?;
        let definition: Rc<dyn QLambdaDefinition> = Rc::new(QLambdaDefinitionInner::new(
            "main",
            instructions,
            vec![],
            max_stack,
        ));
        let trace_points = if self.init_options.is_trace_expression() {
            let mut visitor = TraceExpressionVisitor::new();
            visitor.visit(tree)
        } else {
            Vec::new()
        };
        Ok(QCompileCache::new(definition, trace_points))
    }

    /// 带缓存的编译(同一 script 只编译一次)。对应 Java 方法
    /// `parseToDefinitionWithCache(String)`。
    pub fn parse_to_definition_with_cache(
        &self,
        script: &str,
    ) -> Result<Rc<LoadedCompileCache>, QLSyntaxException> {
        const LEGACY_TENANT: &str = "__java_compatible__";
        const LEGACY_CACHE_CAPACITY: usize = 1024;
        if let Some(cached) = self.compile_cache.borrow_mut().get(LEGACY_TENANT, script) {
            return Ok(cached);
        }
        let compiled = Rc::new(self.parse_definition(script)?);
        self.compile_cache.borrow_mut().insert(
            LEGACY_TENANT,
            script.to_string(),
            Rc::clone(&compiled),
            LEGACY_CACHE_CAPACITY,
            LEGACY_CACHE_CAPACITY,
        );
        Ok(compiled)
    }

    /// 清空编译缓存。对应 Java 方法 `clearCompileCache()`。
    pub fn clear_compile_cache(&self) {
        self.compile_cache.borrow_mut().clear();
    }

    /// 返回 Runner 编译缓存统计。该统计覆盖兼容入口和安全租户入口。
    pub fn compile_cache_stats(&self) -> crate::security::CacheStats {
        self.compile_cache.borrow().stats()
    }

    /// 默认 import 管理器。对应 Java 私有方法 `inheritDefaultImport()`。
    fn inherit_default_import(&self) -> ImportManager<'_> {
        ImportManager::new(
            self.init_options.class_supplier().as_ref(),
            self.init_options.default_import().to_vec(),
        )
    }

    /// 脚本静态检查(操作符黑白名单等)。对应 Java 方法
    /// `check(String, CheckOptions)`:解析后经 `CheckVisitor` 校验。
    pub fn check(
        &self,
        script: &str,
        check_options: &CheckOptions,
    ) -> Result<(), QLSyntaxException> {
        let tree = self.parse_to_syntax_tree(script)?;
        let mut check_visitor = CheckVisitor::new(check_options, script);
        check_visitor.check(&tree)
    }

    /// 以默认检查配置校验脚本。对应 Java 方法 `check(String)`。
    pub fn check_default(&self, script: &str) -> Result<(), QLSyntaxException> {
        self.check(script, &CheckOptions::default())
    }

    // ------------------------------------------------------------------
    // parse cache(Java parseToSerializableCache / loadSerializableCache)
    // ------------------------------------------------------------------

    /// 导出脚本的可序列化 parse cache。对应 Java 方法
    /// `parseToSerializableCache(String)`(Java 抛 `QLSyntaxException`/
    /// `SerializableParseCacheException`;Rust 统一为 [`QLException`])。
    pub fn export_parse_cache(&self, script: &str) -> Result<SerializableParseCache, QLException> {
        let compile_cache = self
            .parse_definition(script)
            .map_err(QLSyntaxException::into_exception)?;
        let exporter = SerializableParseCacheExporter::new(
            script,
            &self.operator_manager,
            self.init_options.is_trace_expression(),
        );
        exporter
            .export(&compile_cache)
            .map_err(|err| err.into_ql_exception())
    }

    /// 导入(加载)可序列化 parse cache,绑定到本 runner。对应 Java 方法
    /// `loadSerializableCache(SerializableParseCache)`。
    pub fn import_parse_cache(
        &self,
        cache: &SerializableParseCache,
    ) -> ImportResult<LoadedParseCache> {
        let mut importer = SerializableParseCacheImporter::new(
            &self.operator_manager,
            self.init_options.class_supplier().as_ref(),
        );
        importer.load(cache, self.identity)
    }

    /// 把可序列化 parse cache 加载后放入编译缓存(后续
    /// `execute(..., cache=true)` 直接命中)。Java 无同名方法,等价于
    /// 「预热 `compileCache`」的宿主操作。
    /// 对应 Java: com.alibaba.qlexpress4.Express4Runner#setParseCache。
    pub fn set_parse_cache(&self, cache: &SerializableParseCache) -> ImportResult<()> {
        let loaded = self.import_parse_cache(cache)?;
        let script = loaded.get_script().unwrap_or_default().to_string();
        self.compile_cache.borrow_mut().insert(
            "__java_compatible__",
            script,
            Rc::new(loaded.get_compile_cache().clone()),
            1024,
            1024,
        );
        Ok(())
    }

    // ------------------------------------------------------------------
    // 函数注册(Java addFunction 系列)
    // ------------------------------------------------------------------

    /// 注册脚本函数(同名已存在则失败)。对应 Java 方法
    /// `addFunction(String name, CustomFunction function)`
    /// (`putIfAbsent` 语义)。
    pub fn add_function<F>(&self, name: impl Into<String>, function: F) -> bool
    where
        F: CustomFunction + 'static,
    {
        self.add_function_shared(name, Rc::new(function))
    }

    /// 以共享句柄注册脚本函数(`putIfAbsent` 语义;Java 无同名重载,
    /// 对应 `addFunction` 的 `Rc` 形态,便于同一函数多处注册/取回)。
    /// 对应 Java: com.alibaba.qlexpress4.Express4Runner#addFunctionShared。
    pub fn add_function_shared(
        &self,
        name: impl Into<String>,
        function: Rc<dyn CustomFunction>,
    ) -> bool {
        let name = name.into();
        let mut functions = self.user_define_functions.borrow_mut();
        match functions.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => false,
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(function);
                self.registered_capabilities
                    .borrow_mut()
                    .insert(Capability::Function(name));
                true
            }
        }
    }

    /// 取已注册函数。对应 Java 方法 `getFunction(String)`。
    pub fn get_function(&self, function_name: &str) -> Option<Rc<dyn CustomFunction>> {
        self.user_define_functions
            .borrow()
            .get(function_name)
            .map(Rc::clone)
    }

    /// 注册一元闭包函数(取首个参数,缺省为 `Null`)。对应 Java 方法
    /// `addFunction(String name, Function<T, R> function)`。
    pub fn add_function_unary<F>(&self, name: impl Into<String>, function: F) -> bool
    where
        F: Fn(DataValue) -> DataValue + 'static,
    {
        self.add_function(
            name,
            move |_ctx: &mut dyn QContext, parameters: &Parameters| {
                let arg = parameters.get_value(0);
                Ok(function(arg))
            },
        )
    }

    /// 注册二元闭包函数(取前两个参数)。对应 Java
    /// `addOperatorBiFunction` 的函数形态(Java `addFunction` 无
    /// BiFunction 重载;Rust 便利变体,语义同 Java lambda 包装:
    /// 参数经 `parameters.get(i).get()` 取出)。
    pub fn add_function_bi<F>(&self, name: impl Into<String>, function: F) -> bool
    where
        F: Fn(DataValue, DataValue) -> DataValue + 'static,
    {
        self.add_function(
            name,
            move |_ctx: &mut dyn QContext, parameters: &Parameters| {
                Ok(function(parameters.get_value(0), parameters.get_value(1)))
            },
        )
    }

    /// 注册变参函数。对应 Java 方法
    /// `addVarArgsFunction(String name, QLFunctionalVarargs)`:
    /// 参数逐个 `get()` 后交给 `functionalVarargs.call(...)`。
    pub fn add_varargs_function<F>(&self, name: impl Into<String>, functional_varargs: F) -> bool
    where
        F: QLFunctionalVarargs + 'static,
    {
        self.add_function(
            name,
            move |_ctx: &mut dyn QContext, parameters: &Parameters| {
                functional_varargs.call(&parameters.values())
            },
        )
    }

    /// 批量注册函数,逐名汇报成功/失败(同名冲突进 fail)。对应 Java
    /// `BatchAddFunctionResult` 的部分失败语义
    /// (Java 入口为 `addFunctionsDefinedInScript`;Rust 另提供本直接批量入口)。
    pub fn batch_add_function(
        &self,
        functions: Vec<(String, Rc<dyn CustomFunction>)>,
    ) -> BatchAddFunctionResult {
        let mut result = BatchAddFunctionResult::new();
        for (name, function) in functions {
            if self.add_function_shared(&name, function) {
                result.add_succ(name);
            } else {
                result.add_fail(name);
            }
        }
        result
    }

    /// 执行脚本并把其中 `function` 定义的函数注册进引擎。对应 Java 方法
    /// `addFunctionsDefinedInScript(String, ExpressContext, QLOptions)`。
    pub fn add_functions_defined_in_script(
        &self,
        script_with_function_define: &str,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<BatchAddFunctionResult, QLException> {
        let compile_cache = self
            .parse_definition(script_with_function_define)
            .map_err(QLSyntaxException::into_exception)?;
        self.add_functions_from_compile_cache(&compile_cache, context, ql_options)
    }

    /// 从可序列化编译缓存注册其中定义的函数。对应 Java 方法
    /// `addFunctionsDefinedInScript(SerializableParseCache, ExpressContext, QLOptions)`。
    pub fn add_functions_defined_in_cache(
        &self,
        cache: &SerializableParseCache,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<BatchAddFunctionResult, QLException> {
        let loaded = self
            .import_parse_cache(cache)
            .map_err(|error| error.into_ql_exception())?;
        self.add_functions_defined_in_loaded_cache(&loaded, context, ql_options)
    }

    /// 从已加载编译缓存注册其中定义的函数。对应 Java 方法
    /// `addFunctionsDefinedInScript(LoadedParseCache, ExpressContext, QLOptions)`。
    pub fn add_functions_defined_in_loaded_cache(
        &self,
        cache: &LoadedParseCache,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<BatchAddFunctionResult, QLException> {
        if !cache.is_bound_to(self.identity) {
            return Err(QLException::for_test(
                crate::exception::ql_exception::QLExceptionKind::Runtime,
                format!(
                    "LoadedParseCache is bound to another Express4Runner: {}",
                    cache.get_script().unwrap_or_default()
                ),
                crate::exception::error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL,
            ));
        }
        self.add_functions_from_compile_cache(cache.get_compile_cache(), context, ql_options)
    }

    /// 执行主 Lambda 的定义阶段并把函数表逐项注册。对应 Java 私有重载
    /// `addFunctionsDefinedInScript(QCompileCache, ExpressContext, QLOptions)`。
    fn add_functions_from_compile_cache(
        &self,
        compile_cache: &LoadedCompileCache,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<BatchAddFunctionResult, QLException> {
        let global_scope = QvmGlobalScope::with_context(
            context,
            self.user_define_functions.borrow().clone(),
            ql_options.attachments().clone(),
            ql_options.is_pollute_user_context(),
        );
        let runtime = Rc::new(QvmRuntime::new(
            QTraces::empty(),
            ql_options.attachments().clone(),
            Rc::clone(self.reflect_loader.registry()),
            current_time_millis(),
        ));
        // Java: mainLambdaTrace.getqLambda().getFunctionDefined()
        let mut root_context = crate::runtime::delegate_qcontext::DelegateQContext::new(
            Rc::clone(&runtime),
            QScope::global(global_scope),
        );
        let root_lambda = compile_cache.q_lambda_definition().clone().to_lambda(
            &mut root_context,
            ql_options,
            true,
        );
        let function_table = root_lambda.function_defined(&[])?;
        let mut result = BatchAddFunctionResult::new();
        for (name, function) in function_table {
            if self.add_function_shared(&name, function) {
                result.add_succ(name);
            } else {
                result.add_fail(name);
            }
        }
        Ok(result)
    }

    /// 把(静态/实例)原生方法注册为脚本函数。对应 Java 方法
    /// `addFunctionOfServiceMethod(String, Object, String, Class[])`:
    /// Java 以反射查方法并包成 `QMethodFunction`;Rust 由宿主显式给出
    /// [`IMethod`](SPEC §4),`object` 为接收者(静态方法传 `None`)。
    pub fn add_function_of_class_method(
        &self,
        name: impl Into<String>,
        object: Option<DataValue>,
        method: Rc<dyn IMethod>,
    ) -> bool {
        self.add_function(name, QMethodFunction::new(object, method))
    }

    /// 把注册表中某类型的静态方法注册为脚本函数。对应 Java 方法
    /// `addFunctionOfServiceMethod` 的静态形态 + `addStaticFunction`
    /// 的单方法版;方法未注册时返回 false(Java 抛
    /// `IllegalArgumentException`,Rust 以 false 表示查找失败)。
    pub fn add_static_method(
        &self,
        name: impl Into<String>,
        class_name: &str,
        method_name: &str,
    ) -> bool {
        let Some(method) = self
            .registry()
            .get_type(class_name)
            .and_then(|t| t.static_methods.get(method_name).map(Rc::clone))
        else {
            return false;
        };
        // 注册表中的静态方法不携带参数签名(Java 由反射取得精确
        // `Class[]`);按 `Object...` varargs 包装(实参全部匹配),
        // 调用前把 vararg 打包的数组还原为平铺参数列表。
        let i_method = Rc::new(NativeIMethod::new(
            method_name,
            ClassRef::Named(class_name.to_string()),
            vec![ClassRef::Named("java.lang.Object".to_string())],
            true,
            Rc::new(move |bean, args| {
                let flat: Vec<DataValue> = match args {
                    [DataValue::Array(items)] => items.borrow().clone(),
                    _ => args.to_vec(),
                };
                method(bean, &flat)
            }),
        ));
        self.add_function(name, QMethodFunction::new(None, i_method))
    }

    /// 注册编译期函数。对应 Java 方法
    /// `addCompileTimeFunction(String, CompileTimeFunction)`
    /// (`putIfAbsent` 语义)。
    pub fn add_compile_time_function(
        &self,
        name: impl Into<String>,
        compile_time_function: Rc<dyn CompileTimeFunction>,
    ) -> bool {
        let name = name.into();
        let mut functions = self.compile_time_functions.borrow_mut();
        match functions.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(_) => false,
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(compile_time_function);
                self.registered_capabilities
                    .borrow_mut()
                    .insert(Capability::CompileTimeFunction(name));
                true
            }
        }
    }

    /// 取已注册编译期函数。对应 Java 方法 `getCompileTimeFunction(String)`。
    pub fn get_compile_time_function(
        &self,
        function_name: &str,
    ) -> Option<Rc<dyn CompileTimeFunction>> {
        self.compile_time_functions
            .borrow()
            .get(function_name)
            .map(Rc::clone)
    }

    /// 注册扩展函数(为既有类型追加实例方法)。对应 Java 方法
    /// `addExtendFunction(ExtensionFunction)`:Java 注册进
    /// `ReflectLoader`;Rust 注册进 [`NativeRegistry`] 的方法表
    /// (声明类取 `ExtensionFunction.declaring_class` 的类名)。
    pub fn add_extend_function<F>(&mut self, extension_function: F)
    where
        F: ExtensionFunction + 'static,
    {
        let method_name = extension_function.name().to_string();
        let type_name = extension_function.declaring_class().java_name().to_string();
        self.registry_mut().register_method(
            type_name.clone(),
            method_name.clone(),
            as_native_method(extension_function),
        );
        self.registered_capabilities
            .borrow_mut()
            .insert(Capability::ExtensionMethod {
                type_name,
                method_name,
            });
    }

    /// 注册原生类型(SPEC §4 宿主 API;Java 无同名方法,对应
    /// 「让类对脚本可见」的显式注册)。
    /// 对应 Java: com.alibaba.qlexpress4.Express4Runner#registerNativeType。
    pub fn register_native_type(&mut self, native_type: NativeType) {
        self.registry_mut().register_type(native_type);
    }

    /// 通过 `#[derive(QLExpressType)]` 宏注册原生类型。对应
    /// Java `ReflectLoader.register` 的 Rust 形态。
    /// 对应 Java: com.alibaba.qlexpress4.Express4Runner#registerQlexpressType。
    pub fn register_qlexpress_type<T>(&mut self)
    where
        T: crate::runtime::member::QLExpressNativeType,
    {
        let native_type = T::build_native_type();
        self.register_native_type(native_type);
    }

    /// 读取对象字段。对应 Java 方法 `loadField(Object, String)`。
    pub fn load_field(&self, object: &DataValue, field_name: &str) -> Option<QValue> {
        self.reflect_loader.load_field(object, field_name)
    }

    // ------------------------------------------------------------------
    // 宏(Java addMacro / addOrReplaceMacro)
    // ------------------------------------------------------------------

    /// 注册全局宏(同名已存在则失败)。对应 Java 方法
    /// `addMacro(String, String)`(`defineMacroIfAbsent` 语义)。
    pub fn add_macro(&self, name: &str, macro_script: &str) -> Result<bool, QLSyntaxException> {
        let define = self.parse_macro_define(name, macro_script)?;
        let inserted = self.global_scope.define_macro_if_absent(name, define);
        if inserted {
            self.registered_capabilities
                .borrow_mut()
                .insert(Capability::Macro(name.to_string()));
        }
        Ok(inserted)
    }

    /// 注册或替换全局宏。对应 Java 方法
    /// `addOrReplaceMacro(String, String)`。
    pub fn add_or_replace_macro(
        &self,
        name: &str,
        macro_script: &str,
    ) -> Result<(), QLSyntaxException> {
        let define = self.parse_macro_define(name, macro_script)?;
        self.global_scope.define_macro(name, define);
        self.registered_capabilities
            .borrow_mut()
            .insert(Capability::Macro(name.to_string()));
        Ok(())
    }

    /// 编译宏定义。对应 Java 私有方法 `parseMacroDefine(String, String)`:
    /// 以 `MACRO_<name>` 子作用域 + `Context.MACRO` 编译,
    /// 并判定最后一条语句是否为表达式语句。
    fn parse_macro_define(
        &self,
        name: &str,
        macro_script: &str,
    ) -> Result<
        MacroDefine<crate::aparser::qvm_instruction_visitor::SharedInstruction>,
        QLSyntaxException,
    > {
        let tree = self.parse_to_syntax_tree(macro_script)?;
        let import_manager = RefCell::new(self.inherit_default_import());
        let user_define_functions = self.user_define_functions.borrow();
        let compile_time_functions = self.compile_time_functions.borrow();
        let macro_scope = Rc::new(GeneratorScope::new(
            format!("MACRO_{name}"),
            Some(Rc::clone(&self.global_scope)),
        ));
        let visitor = QvmInstructionVisitor::with_context(
            macro_script,
            &import_manager,
            macro_scope,
            &self.operator_manager,
            VisitorContext::Macro,
            &compile_time_functions,
            &user_define_functions,
            &self.init_options,
        );
        let (instructions, _max_stack) = visitor.compile(&tree)?;
        // Java 宏指令与主编译共享指令对象;Rust 编译产出 `Box`,宏表
        // 共享语义对应 `Rc`(与 visitor 内部 `get_macro_instructions`
        // 的 `Rc::from` 转换一致)。
        let instructions = instructions.into_iter().map(Rc::from).collect();
        // Java: 最后一条 blockStatement 是 ExpressionStatement 则
        // lastStmtExpress = true。
        let last_stmt_express = match &tree {
            Node::Program(program) => match &program.block_statements {
                Some(block) => match block.as_ref() {
                    Node::BlockStatements(statements) => statements
                        .statements
                        .last()
                        .map(|last| matches!(last, Node::ExpressionStatement(_)))
                        .unwrap_or(false),
                    _ => false,
                },
                None => false,
            },
            _ => false,
        };
        Ok(MacroDefine::new(instructions, last_stmt_express))
    }

    // ------------------------------------------------------------------
    // 操作符(Java addOperator / replaceDefaultOperator / addAlias)
    // ------------------------------------------------------------------

    /// 注册自定义二元操作符(默认优先级 `QLPrecedences.MULTI`)。
    /// 对应 Java 方法 `addOperator(String, CustomBinaryOperator)`。
    pub fn add_operator(
        &mut self,
        operator: impl Into<String>,
        custom_binary_operator: Rc<dyn CustomBinaryOperator>,
    ) -> bool {
        let operator = operator.into();
        let inserted = self.operator_manager.add_binary_operator(
            operator.clone(),
            custom_binary_operator,
            ql_precedences::MULTI,
        );
        if inserted {
            self.registered_capabilities
                .borrow_mut()
                .insert(Capability::Operator(operator));
        }
        inserted
    }

    /// 注册自定义二元操作符(指定优先级)。对应 Java 方法
    /// `addOperator(String, CustomBinaryOperator, int precedence)`。
    pub fn add_operator_with_precedence(
        &mut self,
        operator: impl Into<String>,
        custom_binary_operator: Rc<dyn CustomBinaryOperator>,
        precedence: i32,
    ) -> bool {
        let operator = operator.into();
        let inserted = self.operator_manager.add_binary_operator(
            operator.clone(),
            custom_binary_operator,
            precedence,
        );
        if inserted {
            self.registered_capabilities
                .borrow_mut()
                .insert(Capability::Operator(operator));
        }
        inserted
    }

    /// 以二元闭包注册操作符。对应 Java 方法
    /// `addOperatorBiFunction(String, BiFunction)`(默认 MULTI 优先级,
    /// 左右操作数取 `get()` 后的值)。
    pub fn add_operator_bi<F>(&mut self, operator: impl Into<String>, bi_function: F) -> bool
    where
        F: Fn(DataValue, DataValue) -> DataValue + 'static,
    {
        self.add_operator(
            operator,
            Rc::new(move |left: &QValue, right: &QValue| Ok(bi_function(left.get(), right.get()))),
        )
    }

    /// 以变参闭包注册操作符。对应 Java 方法
    /// `addOperator(String, QLFunctionalVarargs)`。
    pub fn add_operator_varargs<F>(
        &mut self,
        operator: impl Into<String>,
        functional_varargs: F,
    ) -> bool
    where
        F: QLFunctionalVarargs + 'static,
    {
        self.add_operator(
            operator,
            Rc::new(move |left: &QValue, right: &QValue| {
                functional_varargs.call(&[left.get(), right.get()])
            }),
        )
    }

    /// 替换内建操作符实现。对应 Java 方法
    /// `replaceDefaultOperator(String, CustomBinaryOperator)`。
    pub fn replace_operator(
        &mut self,
        operator: &str,
        custom_binary_operator: Rc<dyn CustomBinaryOperator>,
    ) -> bool {
        let replaced = self
            .operator_manager
            .replace_default_operator(operator, custom_binary_operator);
        if replaced {
            self.registered_capabilities
                .borrow_mut()
                .insert(Capability::Operator(operator.to_string()));
        }
        replaced
    }

    /// 为既有操作符添加别名。对应 Java `OperatorManager.addOperatorAlias`
    /// (`Express4Runner.addAlias` 中的操作符分支)。
    pub fn add_operator_alias(&mut self, alias: impl Into<String>, origin_token: &str) -> bool {
        let alias = alias.into();
        let inserted = self
            .operator_manager
            .add_operator_alias(alias.clone(), origin_token);
        if inserted {
            self.registered_capabilities
                .borrow_mut()
                .insert(Capability::Operator(alias));
        }
        inserted
    }

    /// 为关键字/操作符/函数添加别名(任一分支成功即 true)。对应 Java
    /// 方法 `addAlias(String alias, String originToken)`。
    pub fn add_alias(&mut self, alias: impl Into<String>, origin_token: &str) -> bool {
        let alias = alias.into();
        let key_word_result = self
            .operator_manager
            .add_key_word_alias(alias.clone(), origin_token);
        let operator_result = self
            .operator_manager
            .add_operator_alias(alias.clone(), origin_token);
        if operator_result {
            self.registered_capabilities
                .borrow_mut()
                .insert(Capability::Operator(alias.clone()));
        }
        // Java addFunctionAlias 分支:函数别名指向同一 CustomFunction。
        let function_result = {
            let function = self
                .user_define_functions
                .borrow()
                .get(origin_token)
                .map(Rc::clone);
            match function {
                Some(function) => self.add_function_shared(alias, function),
                None => false,
            }
        };
        key_word_result || operator_result || function_result
    }

    // ------------------------------------------------------------------
    // 安全策略(Java InitOptions.securityStrategy 的运行期接线)
    // ------------------------------------------------------------------

    /// 设置成员访问安全策略(白/黑名单、开放、隔离),作用于脚本经
    /// 注册表进行的方法/字段分派。对应 Java `ReflectLoader` 持有的
    /// `securityStrategy`(`InitOptions.Builder.securityStrategy`)。
    pub fn set_security_strategy(&self, security_strategy: QLSecurityStrategy) {
        self.reflect_loader
            .registry()
            .set_security_strategy(security_strategy);
    }

    /// 当前安全策略。对应 Java `InitOptions.getSecurityStrategy()`。
    pub fn security_strategy(&self) -> QLSecurityStrategy {
        self.reflect_loader.security_strategy()
    }

    fn validate_capabilities(&self, sandbox_profile: &SandboxProfile) -> Result<(), QLException> {
        for capability in self.registered_capabilities.borrow().iter() {
            if !sandbox_profile.capability_policy.is_allowed(capability) {
                return Err(crate::runtime::execution_budget::budget_error(
                    crate::exception::QLExceptionKind::Runtime,
                    "SANDBOX_CAPABILITY_DENIED",
                    format!("sandbox capability is not allowed: {capability:?}"),
                ));
            }
        }
        match self.security_strategy() {
            QLSecurityStrategy::Isolation => Ok(()),
            QLSecurityStrategy::WhiteList(members) => {
                for member in members {
                    let capability = Capability::NativeMember {
                        type_name: member.type_name,
                        member_name: member.member_name,
                    };
                    if !sandbox_profile.capability_policy.is_allowed(&capability) {
                        return Err(crate::runtime::execution_budget::budget_error(
                            crate::exception::QLExceptionKind::Runtime,
                            "SANDBOX_CAPABILITY_DENIED",
                            format!("native member is not allowed: {capability:?}"),
                        ));
                    }
                }
                Ok(())
            }
            QLSecurityStrategy::Open | QLSecurityStrategy::BlackList(_) => {
                Err(crate::runtime::execution_budget::budget_error(
                    crate::exception::QLExceptionKind::Runtime,
                    "SANDBOX_NATIVE_POLICY_UNSAFE",
                    "execute_checked requires Isolation or an explicit native WhiteList",
                ))
            }
        }
    }

    fn validate_source_budget(
        &self,
        script: &str,
        sandbox_profile: &SandboxProfile,
    ) -> Result<(), QLException> {
        if script.len() > sandbox_profile.limits.max_source_bytes {
            return Err(sandbox_limit_error(
                "SANDBOX_SOURCE_BYTES_EXCEEDED",
                script.len(),
                sandbox_profile.limits.max_source_bytes,
            ));
        }
        Ok(())
    }

    fn validate_instruction_budget(
        &self,
        compile_cache: &LoadedCompileCache,
        sandbox_profile: &SandboxProfile,
    ) -> Result<(), QLException> {
        let instruction_count = compile_cache
            .q_lambda_definition()
            .compiled_instruction_count();
        if instruction_count > sandbox_profile.limits.max_instructions {
            return Err(sandbox_limit_error(
                "SANDBOX_INSTRUCTIONS_EXCEEDED",
                instruction_count,
                sandbox_profile.limits.max_instructions,
            ));
        }
        Ok(())
    }

    fn check_sandbox_deadline(
        &self,
        started: Instant,
        sandbox_profile: &SandboxProfile,
    ) -> Result<(), QLException> {
        if sandbox_profile.cancellation_token.is_cancelled() {
            return Err(crate::runtime::execution_budget::budget_error(
                crate::exception::QLExceptionKind::Timeout,
                "SANDBOX_CANCELLED",
                "sandbox execution was cancelled",
            ));
        }
        if started.elapsed().as_millis() >= u128::from(sandbox_profile.limits.timeout_millis) {
            return Err(crate::runtime::execution_budget::budget_error(
                crate::exception::QLExceptionKind::Timeout,
                "SANDBOX_DEADLINE_EXCEEDED",
                "sandbox deadline exceeded during validation or compilation",
            ));
        }
        Ok(())
    }

    // ------------------------------------------------------------------
    // 外部变量/函数收集(Java getOutVarNames / getOutVarAttrs /
    // getOutFunctions)
    // ------------------------------------------------------------------

    /// 收集脚本引用的外部变量名(需经上下文提供)。对应 Java 方法
    /// `getOutVarNames(String)`。
    pub fn get_out_var_names(&self, script: &str) -> Result<HashSet<String>, QLSyntaxException> {
        let tree = self.parse_to_syntax_tree(script)?;
        let mut visitor = OutVarNamesVisitor::new(self.inherit_default_import());
        tree.accept(&mut visitor);
        Ok(visitor.out_vars().clone())
    }

    /// 收集脚本对外部变量的属性访问路径。对应 Java 方法
    /// `getOutVarAttrs(String)`。
    pub fn get_out_var_attrs(
        &self,
        script: &str,
    ) -> Result<HashSet<Vec<String>>, QLSyntaxException> {
        let tree = self.parse_to_syntax_tree(script)?;
        let mut visitor = OutVarAttrsVisitor::new(self.inherit_default_import());
        tree.accept(&mut visitor);
        Ok(visitor.out_var_attrs().clone())
    }

    /// 收集脚本引用的外部函数名。对应 Java 方法 `getOutFunctions(String)`。
    pub fn get_out_function_names(
        &self,
        script: &str,
    ) -> Result<HashSet<String>, QLSyntaxException> {
        let tree = self.parse_to_syntax_tree(script)?;
        let mut visitor = OutFunctionVisitor::new();
        tree.accept(&mut visitor);
        Ok(visitor.out_functions().clone())
    }

    /// 静态解析脚本并返回表达式追踪点树。
    ///
    /// 无论初始化时是否开启运行时追踪，本方法都会执行静态访问器；
    /// 对应 Java 方法 `getExpressionTracePoints(String)`。
    pub fn get_expression_trace_points(
        &self,
        script: &str,
    ) -> Result<Vec<TracePointTree>, QLSyntaxException> {
        let tree = self.parse_to_syntax_tree(script)?;
        let mut visitor = TraceExpressionVisitor::new();
        Ok(visitor.visit(&tree))
    }

    /// 执行脚本并取表达式 trace 列表。只有初始化选项与本次执行选项
    /// 都开启 trace 时由 QVM 填充。对应 Java `QLResult.getExpressionTraces`
    /// 的独立取 trace 用法。
    pub fn get_expression_trace(
        &self,
        script: &str,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<Vec<ExpressionTrace>, QLException> {
        Ok(self
            .execute_with_context(script, context, ql_options)?
            .expression_traces()
            .to_vec())
    }
}

/// `HashMap<String, DataValue>` → 脚本上下文的有序 map
/// (Java `new MapExpressContext(Map<String, Object>)`)。
fn map_to_index_map(context: HashMap<String, DataValue>) -> Rc<RefCell<IndexMap>> {
    let entries = context
        .into_iter()
        .map(|(key, value)| (DataValue::Str(key), value))
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
