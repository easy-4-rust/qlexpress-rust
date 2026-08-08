impl Express4Runner {
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
        if let Some(cached) = self
            .compile_cache
            .borrow_mut()
            .get(JAVA_COMPATIBLE_CACHE_TENANT, script)
        {
            return Ok(cached);
        }
        let compiled = Rc::new(self.parse_definition(script)?);
        self.compile_cache.borrow_mut().insert(
            JAVA_COMPATIBLE_CACHE_TENANT,
            script.to_string(),
            Rc::clone(&compiled),
            UNBOUNDED_CACHE_CAPACITY,
            UNBOUNDED_CACHE_CAPACITY,
        );
        Ok(compiled)
    }

    /// 将脚本编译并物化为尚未执行的主 Lambda。
    ///
    /// 对应 Java：`Express4Runner#parseToLambda(String, ExpressContext,
    /// QLOptions)`。当 `ql_options.cache` 为 `true` 时复用编译缓存，否则每次
    /// 重新编译；这里只创建运行时、全局作用域和 Lambda，不执行任何指令。
    ///
    /// # 参数
    ///
    /// - `script`：待编译脚本。
    /// - `context`：Lambda 捕获的外部变量上下文。
    /// - `ql_options`：缓存、附件、上下文污染和 trace 等执行选项。
    ///
    /// # 返回值
    ///
    /// 返回 Lambda 与其本次独立 trace 注册表。
    ///
    /// # 错误
    ///
    /// 脚本词法、语法或编译失败时返回原始 [`QLSyntaxException`]。
    pub fn parse_to_lambda(
        &self,
        script: &str,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> Result<QLambdaTrace, QLSyntaxException> {
        if ql_options.is_cache() {
            let compile_cache = self.parse_to_definition_with_cache(script)?;
            Ok(self.compile_cache_to_lambda(compile_cache.as_ref(), context, ql_options, true))
        } else {
            let compile_cache = self.parse_definition(script)?;
            Ok(self.compile_cache_to_lambda(&compile_cache, context, ql_options, true))
        }
    }

    /// 将已加载且绑定到当前 runner 的 parse cache 物化为主 Lambda。
    ///
    /// 对应 Java：`Express4Runner#parseToLambda(LoadedParseCache,
    /// ExpressContext, QLOptions)`。与 Java 一样，来自其它 runner 的缓存
    /// 必须拒绝，不能绕过缓存中 operator、类型供应器和宿主模型的绑定关系。
    ///
    /// # 参数
    ///
    /// - `cache`：由当前 runner 的 [`Self::import_parse_cache`] 生成的缓存。
    /// - `context`：Lambda 捕获的外部变量上下文。
    /// - `ql_options`：本次 Lambda 的执行选项。
    ///
    /// # 返回值
    ///
    /// 返回尚未执行的 Lambda 与 trace 注册表。
    ///
    /// # 错误
    ///
    /// 缓存绑定到其它 runner 时返回
    /// `SERIALIZABLE_PARSE_CACHE_INVALID_MODEL`。
    pub fn parse_loaded_cache_to_lambda(
        &self,
        cache: &LoadedParseCache,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> ImportResult<QLambdaTrace> {
        if !cache.is_bound_to(self.identity) {
            return Err(
                crate::api::parsecache::SerializableParseCacheException::new(
                    cache.get_script(),
                    None,
                    crate::exception::error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL,
                    &crate::exception::error_codes::format_msg(
                        crate::exception::error_codes::error_msg(
                            crate::exception::error_codes::SERIALIZABLE_PARSE_CACHE_INVALID_MODEL,
                        ),
                        &["LoadedParseCache is bound to another Express4Runner".to_string()],
                    ),
                ),
            );
        }
        Ok(self.compile_cache_to_lambda(
            cache.get_compile_cache(),
            context,
            ql_options,
            cache.has_trace_points(),
        ))
    }

    /// 加载可序列化 parse cache 后物化为主 Lambda。
    ///
    /// 对应 Java：`Express4Runner#parseToLambda(SerializableParseCache,
    /// ExpressContext, QLOptions)`；先执行与 `loadSerializableCache` 完全相同的
    /// 模型校验及 runner 绑定，再复用 loaded-cache 重载。
    ///
    /// # 参数
    ///
    /// - `cache`：待校验和导入的可序列化缓存。
    /// - `context`：Lambda 捕获的外部变量上下文。
    /// - `ql_options`：本次 Lambda 的执行选项。
    ///
    /// # 返回值
    ///
    /// 返回尚未执行的 Lambda 与 trace 注册表。
    ///
    /// # 错误
    ///
    /// 返回导入、模型校验或 runner 绑定错误。
    pub fn parse_serializable_cache_to_lambda(
        &self,
        cache: &SerializableParseCache,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
    ) -> ImportResult<QLambdaTrace> {
        let loaded = self.import_parse_cache(cache)?;
        self.parse_loaded_cache_to_lambda(&loaded, context, ql_options)
    }

    /// 由编译产物创建 Java `QLambdaTrace` 对应对象。
    ///
    /// 对应 Java 私有方法
    /// `Express4Runner#parseToLambda(QCompileCache, ExpressContext,
    /// QLOptions, boolean)`。
    fn compile_cache_to_lambda(
        &self,
        compile_cache: &LoadedCompileCache,
        context: Rc<dyn ExpressContext>,
        ql_options: &QLOptions,
        trace_points_available: bool,
    ) -> QLambdaTrace {
        if self.init_options.is_debug() {
            (self.init_options.debug_info_consumer())("\nInstructions:".to_string());
            compile_cache.q_lambda_definition().println(0, &mut |line| {
                (self.init_options.debug_info_consumer())(line)
            });
        }

        let traces = if self.init_options.is_trace_expression()
            && ql_options.is_trace_expression()
            && trace_points_available
        {
            QTraces::from_trace_points(compile_cache.expression_trace_points())
        } else {
            QTraces::empty()
        };
        let runtime = Rc::new(QvmRuntime::new(
            traces.clone(),
            ql_options.shared_attachments(),
            Rc::clone(self.reflect_loader.registry()),
            current_time_millis(),
        ));
        let global_scope = QvmGlobalScope::with_shared_context(
            context,
            Rc::clone(&self.user_define_functions),
            ql_options.shared_attachments(),
            ql_options.is_pollute_user_context(),
        );
        let mut root_context = DelegateQContext::new(runtime, QScope::global(global_scope));
        let q_lambda = Rc::clone(compile_cache.q_lambda_definition()).to_lambda(
            &mut root_context,
            ql_options,
            true,
        );
        QLambdaTrace::new(q_lambda, traces)
    }

    /// 清空 Java 兼容缓存及安全租户缓存。对应 Java 方法
    /// `clearCompileCache()`；安全租户缓存是 Rust 增强，因此同时清理，避免
    /// 宿主请求清空后仍命中旧的安全编译产物。
    pub fn clear_compile_cache(&self) {
        self.compile_cache.borrow_mut().clear();
        self.secure_compile_cache.borrow_mut().clear();
    }

    /// 返回 Runner 编译缓存统计。该快照汇总互相隔离的 Java 兼容缓存与
    /// 安全租户缓存。
    /// 对应 Java：无（Rust 安全增强的缓存可观测性）。
    pub fn compile_cache_stats(&self) -> crate::security::CacheStats {
        let compatible = self.compile_cache.borrow().stats();
        let secure = self.secure_compile_cache.borrow().stats();
        crate::security::CacheStats {
            entries: compatible.entries.saturating_add(secure.entries),
            hits: compatible.hits.saturating_add(secure.hits),
            misses: compatible.misses.saturating_add(secure.misses),
            evictions: compatible.evictions.saturating_add(secure.evictions),
        }
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
            JAVA_COMPATIBLE_CACHE_TENANT,
            script,
            Rc::new(loaded.get_compile_cache().clone()),
            UNBOUNDED_CACHE_CAPACITY,
            UNBOUNDED_CACHE_CAPACITY,
        );
        Ok(())
    }
}
