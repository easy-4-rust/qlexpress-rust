impl Express4Runner {
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
        self.reflect_loader.add_extend_function(extension_function);
        self.registered_capabilities
            .borrow_mut()
            .insert(Capability::ExtensionMethod {
                type_name,
                method_name,
            });
    }

    /// 注册可变参数扩展函数。对应 Java 方法
    /// `addExtendFunction(String name, Class<?> bindingClass,
    /// QLFunctionalVarargs functionalVarargs)`。
    ///
    /// 参数转换器先把脚本实参收集为 `Object[]`；调用闭包时再按 Java 匿名
    /// `ExtensionFunction` 的实现展开，并把接收者置于参数 0。
    ///
    /// # 参数
    ///
    /// - `name`：脚本中调用的扩展方法名。
    /// - `binding_class`：允许接收该扩展方法的声明类型。
    /// - `functional_varargs`：接收 `[接收者, 脚本参数...]` 的宿主逻辑。
    pub fn add_extend_function_varargs<F>(
        &mut self,
        name: impl Into<String>,
        binding_class: ClassRef,
        functional_varargs: F,
    ) where
        F: QLFunctionalVarargs + 'static,
    {
        self.add_extend_function(VarargsExtensionFunction {
            name: name.into(),
            binding_class,
            functional_varargs,
        });
    }

    /// 注册原生类型(SPEC §4 宿主 API;Java 无同名方法,对应
    /// 「让类对脚本可见」的显式注册)。
    /// 对应 Java: com.alibaba.qlexpress4.Express4Runner#registerNativeType。
    pub fn register_native_type(&mut self, native_type: NativeType) {
        self.registry().register_type(native_type);
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
        // Java `Express4Runner#loadField` 是宿主 API，固定传
        // `skipSecurity=true`；脚本内部字段访问仍由 QVM 传 false。
        self.reflect_loader.load_field(object, field_name, true)
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
}
