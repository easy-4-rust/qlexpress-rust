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
            secure_compile_cache: RefCell::new(CompileCacheStore::new()),
            user_define_functions: Rc::new(RefCell::new(HashMap::new())),
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
    /// 对应 Java：无（Rust 安全增强的不可绕过检查入口）。
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
    /// 对应 Java：无（Rust 安全增强的动态上下文检查入口）。
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
                self.secure_compile_cache
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
                self.secure_compile_cache.borrow_mut().insert(
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

    /// 以显式别名对象作为外部变量执行脚本。
    ///
    /// 对应 Java:
    /// `Express4Runner#executeWithAliasObjects(String,QLOptions,Object...)`。
    /// Rust 无运行时注解扫描，参数 `aliased_objects` 显式携带每个对象的
    /// `@QLAlias.value()` 等价元数据；对象顺序、别名覆盖及上下文写回语义
    /// 与 Java `QLAliasContext` 一致。
    ///
    /// 参数：
    /// - `script`：待执行脚本；
    /// - `ql_options`：本次执行选项；
    /// - `aliased_objects`：按 Java 可变参数顺序排列的别名对象。
    ///
    /// 返回：脚本执行结果；解析或运行失败时返回 [`QLException`]。
    pub fn execute_with_alias_objects(
        &self,
        script: &str,
        ql_options: &QLOptions,
        aliased_objects: &[(&[&str], DataValue)],
    ) -> Result<QLResult, QLException> {
        self.execute_with_context(
            script,
            Rc::new(QLAliasContext::new(aliased_objects)),
            ql_options,
        )
    }

    /// 兼容早期 Rust API 的名称。
    ///
    /// 新代码应使用 [`Self::execute_with_alias_objects`]，以保持与 Java
    /// `executeWithAliasObjects` 的对象名称一致性。
    /// 对应 Java：`Express4Runner#executeWithAliasObjects`（Rust 早期兼容别名）。
    pub fn execute_with_alias_values(
        &self,
        script: &str,
        ql_options: &QLOptions,
        aliased_values: &[(&[&str], DataValue)],
    ) -> Result<QLResult, QLException> {
        self.execute_with_alias_objects(script, ql_options, aliased_values)
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

    /// 以 Map 上下文加载并执行可序列化 parse cache。
    ///
    /// 对应 Java：`Express4Runner#execute(SerializableParseCache,
    /// Map<String, Object>, QLOptions)`。该重载与字符串脚本的 Map 入口一样，
    /// 先构造 [`MapExpressContext`]，再复用 [`Self::execute_with_cache`]；
    /// `polluteUserContext`、attachments 和 trace 等执行选项保持同一语义。
    ///
    /// # 参数
    ///
    /// - `cache`：待导入并执行的可序列化编译缓存；
    /// - `context`：变量名到脚本值的 Map 上下文；
    /// - `ql_options`：本次执行选项。
    ///
    /// # 返回值
    ///
    /// 返回脚本结果及可选表达式 trace。
    ///
    /// # 错误
    ///
    /// 缓存模型无效、导入失败或脚本运行失败时返回 [`QLException`]。
    pub fn execute_with_cache_map(
        &self,
        cache: &SerializableParseCache,
        context: HashMap<String, DataValue>,
        ql_options: &QLOptions,
    ) -> Result<QLResult, QLException> {
        self.execute_with_cache(
            cache,
            Rc::new(MapExpressContext::new(map_to_index_map(context))),
            ql_options,
        )
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
        let global_scope = QvmGlobalScope::with_shared_context(
            context,
            Rc::clone(&self.user_define_functions),
            ql_options.shared_attachments(),
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
            ql_options.shared_attachments(),
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
        let global_scope = QvmGlobalScope::with_shared_context(
            context,
            Rc::clone(&self.user_define_functions),
            ql_options.shared_attachments(),
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
            ql_options.shared_attachments(),
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
}
