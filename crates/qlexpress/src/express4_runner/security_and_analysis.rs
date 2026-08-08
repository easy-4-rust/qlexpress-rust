impl Express4Runner {
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
            QLSecurityStrategy::SharedWhiteList(members) => {
                for member in members.borrow().iter() {
                    let capability = Capability::NativeMember {
                        type_name: member.type_name.clone(),
                        member_name: member.member_name.clone(),
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
            QLSecurityStrategy::Open
            | QLSecurityStrategy::BlackList(_)
            | QLSecurityStrategy::SharedBlackList(_)
            | QLSecurityStrategy::Custom(_) => Err(crate::runtime::execution_budget::budget_error(
                crate::exception::QLExceptionKind::Runtime,
                "SANDBOX_NATIVE_POLICY_UNSAFE",
                "execute_checked requires Isolation or an explicit enumerable native WhiteList",
            )),
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
