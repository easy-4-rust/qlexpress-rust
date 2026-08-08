impl Express4Runner {
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

    /// 注册谓词函数，取首个参数（缺省为 `Null`）并返回布尔值。
    ///
    /// 对应 Java `addFunction(String name, Predicate<T> predicate)`：额外参数
    /// 不参与调用，`Predicate#test` 的 `boolean` 结果包装为脚本布尔值；同名
    /// 函数已存在时保持 `putIfAbsent` 语义并返回 `false`。
    ///
    /// # 参数
    ///
    /// - `name`：脚本侧函数名；
    /// - `predicate`：接收第一个脚本参数并返回布尔判定的宿主闭包。
    pub fn add_function_predicate<F>(&self, name: impl Into<String>, predicate: F) -> bool
    where
        F: Fn(DataValue) -> bool + 'static,
    {
        self.add_function(
            name,
            move |_ctx: &mut dyn QContext, parameters: &Parameters| {
                Ok(DataValue::Bool(predicate(parameters.get_value(0))))
            },
        )
    }

    /// 注册无参副作用函数，忽略脚本实参并固定返回 `Null`。
    ///
    /// 对应 Java `addFunction(String name, Runnable runnable)`：调用
    /// `Runnable#run()` 后返回 Java `null`；同名函数已存在时不替换。
    ///
    /// # 参数
    ///
    /// - `name`：脚本侧函数名；
    /// - `runnable`：每次脚本调用时执行一次的宿主闭包。
    pub fn add_function_runnable<F>(&self, name: impl Into<String>, runnable: F) -> bool
    where
        F: Fn() + 'static,
    {
        self.add_function(
            name,
            move |_ctx: &mut dyn QContext, _parameters: &Parameters| {
                runnable();
                Ok(DataValue::Null)
            },
        )
    }

    /// 注册消费型副作用函数，取首个参数（缺省为 `Null`）并固定返回
    /// `Null`。
    ///
    /// 对应 Java `addFunction(String name, Consumer<T> consumer)`：额外参数
    /// 被忽略，`Consumer#accept` 的副作用完成后返回 Java `null`；同名函数
    /// 已存在时不替换。
    ///
    /// # 参数
    ///
    /// - `name`：脚本侧函数名；
    /// - `consumer`：接收第一个脚本参数的宿主闭包。
    pub fn add_function_consumer<F>(&self, name: impl Into<String>, consumer: F) -> bool
    where
        F: Fn(DataValue) + 'static,
    {
        self.add_function(
            name,
            move |_ctx: &mut dyn QContext, parameters: &Parameters| {
                consumer(parameters.get_value(0));
                Ok(DataValue::Null)
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

    /// 扫描并注册实例方法上的 `@QLFunction` 元数据。
    ///
    /// 对应 Java `addObjFunction(Object)`。Rust 由
    /// [`QLFunctionProvider`] 显式提供 `getDeclaredMethods()` 等价清单；
    /// 非公开方法先进入失败列表，公开且带注解的方法按每个别名尝试
    /// `putIfAbsent` 注册。结果列表记录宿主方法原名，与 Java 完全一致。
    pub fn add_obj_function<P>(&self, object: &P) -> BatchAddFunctionResult
    where
        P: QLFunctionProvider + ?Sized,
    {
        self.add_function_by_annotation(object.ql_object_function_methods())
    }

    /// 扫描并注册类型静态方法上的 `@QLFunction` 元数据。
    ///
    /// 对应 Java `addStaticFunction(Class<?>)`；Rust 以类型参数代替 JVM
    /// `Class<?>`，其余成功/失败与重复名称语义与实例入口相同。
    pub fn add_static_function<P>(&self) -> BatchAddFunctionResult
    where
        P: QLFunctionProvider,
    {
        self.add_function_by_annotation(P::ql_static_function_methods())
    }

    /// Java 私有方法 `addFunctionByAnnotation(Class<?>, Object)` 的 Rust
    /// 显式元数据适配。
    fn add_function_by_annotation(&self, methods: Vec<QLFunctionMethod>) -> BatchAddFunctionResult {
        let mut result = BatchAddFunctionResult::new();
        for method in methods {
            if !method.is_public() {
                result.add_fail(method.method_name());
                continue;
            }
            let function_names = method.function_names();
            if !QLFunctionUtil::contains_ql_function_for_method(function_names) {
                continue;
            }
            for function_name in
                QLFunctionUtil::get_ql_function_value(function_names).unwrap_or_default()
            {
                if self.add_function_shared(function_name, method.function()) {
                    result.add_succ(method.method_name());
                } else {
                    result.add_fail(method.method_name());
                }
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
        let global_scope = QvmGlobalScope::with_shared_context(
            context,
            Rc::clone(&self.user_define_functions),
            ql_options.shared_attachments(),
            ql_options.is_pollute_user_context(),
        );
        let runtime = Rc::new(QvmRuntime::new(
            QTraces::empty(),
            ql_options.shared_attachments(),
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
            vec![ClassRef::array_of(ClassRef::Named(
                "java.lang.Object".to_string(),
            ))],
            true,
            Rc::new(move |bean, args| {
                let flat: Vec<DataValue> = match args {
                    [DataValue::Array(items)] => items.borrow().to_vec(),
                    _ => args.to_vec(),
                };
                method(bean, &flat)
            }),
        ));
        self.add_function(name, QMethodFunction::new(None, i_method))
    }
}
