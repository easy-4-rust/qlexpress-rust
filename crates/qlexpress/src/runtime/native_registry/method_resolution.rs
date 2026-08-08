impl NativeRegistry {
    // ---- 对应 Java ReflectLoader.loadMethod + member/MethodHandler ----

    /// 按名解析 `bean` 上的可调用方法。对应 Java `loadMethod` 返回 `null`
    /// 的语义:不存在时返回 `None`。
    ///
    /// # Arguments
    ///
    /// * `bean` - 实例接收者或表示静态类型的 MetaClass。
    /// * `method_name` - 待解析的方法名或 QL 别名。
    ///
    /// # Returns
    ///
    /// 扩展方法优先，其次为安全策略允许的内建或注册方法；均未命中时返回
    /// `None`。需要精确重载选择时应使用 [`NativeRegistry::resolve_method_for_args`]。
    pub fn resolve_method(&self, bean: &DataValue, method_name: &str) -> Option<NativeMethod> {
        // MetaClass 接收者 → 静态方法(Java `isStaticMethod` 分支)。
        if let Some(meta) = as_meta_class(bean) {
            if method_name == "getName" {
                let class_name = meta.java_name().to_string();
                return Some(Rc::new(move |_bean, args| {
                    if args.is_empty() {
                        Ok(DataValue::string(class_name.clone()))
                    } else {
                        Err(wrong_args("Class.getName"))
                    }
                }));
            }
            let name = meta.java_name();
            let native_type = self.get_type(name)?;
            let registered_name =
                Self::resolve_registered_method_name(&native_type, method_name, true);
            // 安全策略接线点:静态方法访问前过 QLSecurityStrategy。
            if !self.check_member(name, registered_name) {
                return None;
            }
            return native_type
                .static_methods
                .get(registered_name)
                .map(Rc::clone);
        }
        let type_name = native_type_name(bean);
        // Java 先解析扩展函数，再判断是否为隔离策略。
        if let Some(method) = self
            .resolve_extension_method(bean, method_name, &[])
            .or_else(|| builtin_extension_method(bean, method_name))
        {
            return Some(method);
        }
        let native_type = self.get_type(&type_name);
        let registered_name = native_type
            .as_ref()
            .map(|native_type| {
                Self::resolve_registered_method_name(native_type.as_ref(), method_name, false)
            })
            .unwrap_or(method_name);
        // Java 反射方法（含 Rust 内建 JDK 方法子集）统一通过安全策略；
        // 别名先还原为真实成员名，再执行与 Java 反射 Member 相同的检查。
        if !self.check_member(&type_name, registered_name) {
            return None;
        }
        if let Some(method) = builtin_method(bean, method_name) {
            return Some(method);
        }
        if let Some(method) = native_type
            .as_ref()
            .and_then(|native_type| native_type.methods.get(registered_name).map(Rc::clone))
        {
            return Some(method);
        }
        None
    }

    /// 按调用现场实参选择同名方法候选。对应 Java
    /// `ReflectLoader#loadMethod(bean, name, argTypes, ...)`。
    ///
    /// # Arguments
    ///
    /// * `bean` - 实例接收者或静态类型对象。
    /// * `method_name` - 脚本调用的方法名或别名。
    /// * `args` - 用于重载选择和类型转换的运行时实参。
    ///
    /// # Returns
    ///
    /// 返回安全策略允许的最佳方法闭包；无匹配项时返回 `None`。
    pub fn resolve_method_for_args(
        &self,
        bean: &DataValue,
        method_name: &str,
        args: &[DataValue],
    ) -> Option<NativeMethod> {
        if let Some(meta) = as_meta_class(bean) {
            if method_name == "getName" && args.is_empty() {
                let class_name = meta.java_name().to_string();
                return Some(Rc::new(move |_bean, _args| {
                    Ok(DataValue::string(class_name.clone()))
                }));
            }
            if method_name == "isArray" && args.is_empty() {
                let is_array = meta.component_type().is_some();
                return Some(Rc::new(move |_bean, _args| Ok(DataValue::Bool(is_array))));
            }
            return self.resolve_registered_candidate(
                meta.java_name(),
                method_name,
                args,
                true,
                bean,
            );
        }

        // Java 将赋给 SAM 接口的 QLambda 包装为动态 Proxy；抽象接口方法
        // 的调用必须原样转发全部实参并取 Lambda 结果。Rust 不具 JVM Proxy，
        // 但脚本值在运行时仍保留 Lambda，因此在方法解析边界提供同一分派。
        // 接口可赋值性已由声明/形参转换检查，宿主自定义 SAM 与 JDK
        // Runnable/Supplier/Consumer/Function 共用此路径。
        if let DataValue::Lambda(lambda) = bean {
            let handler = QLambdaInvocationHandler::new(Rc::clone(lambda));
            return Some(Rc::new(move |_bean, arguments| {
                handler.invoke_abstract(arguments)
            }));
        }

        let type_name = native_type_name(bean);
        // Java 扩展函数优先于反射成员。
        if let Some(method) = self
            .resolve_extension_method(bean, method_name, args)
            .or_else(|| builtin_extension_method(bean, method_name))
        {
            return Some(method);
        }

        if let Some(method) =
            self.resolve_registered_candidate(&type_name, method_name, args, false, bean)
        {
            return Some(method);
        }
        // 未显式登记候选时继续兼容内建方法表。
        if self.get_type(&type_name).is_none_or(|native_type| {
            native_type
                .method_candidates
                .get(method_name)
                .is_none_or(Vec::is_empty)
        }) && self.check_member(&type_name, method_name)
        {
            return builtin_method(bean, method_name);
        }
        None
    }

    /// 按 Java `declaringClass.isAssignableFrom(bean.getClass())` 解析扩展
    /// 函数；不能只用运行时类型名做精确 HashMap 命中，否则注册在
    /// `Number` / `List` 上的扩展无法用于 `Integer` / `ArrayList`。
    fn resolve_extension_method(
        &self,
        bean: &DataValue,
        method_name: &str,
        args: &[DataValue],
    ) -> Option<NativeMethod> {
        let argument_type = runtime_class_ref(bean);
        let signed_candidates = self
            .extension_method_candidates
            .borrow()
            .iter()
            .filter(|entry| entry.1 == method_name && self.is_assignable(&entry.0, &argument_type))
            .map(|entry| entry.2.clone())
            .collect::<Vec<_>>();
        if let Some(candidate) = self.select_method_candidate(&signed_candidates, args) {
            return Some(wrap_method_candidate(candidate));
        }

        // 早期 Rust API 的无签名注册项保留为兼容回退；Java 对应入口全部
        // 使用上面的签名候选，不会因同名注册而覆盖。
        let type_name = native_type_name(bean);
        if let Some(method) = self
            .extension_methods
            .borrow()
            .get(&(type_name.clone(), method_name.to_string()))
            .map(Rc::clone)
        {
            return Some(method);
        }
        let candidates = self
            .extension_methods
            .borrow()
            .iter()
            .map(|((declaring_type, registered_name), method)| {
                (
                    declaring_type.clone(),
                    registered_name.clone(),
                    Rc::clone(method),
                )
            })
            .collect::<Vec<_>>();
        candidates
            .into_iter()
            .find_map(|(declaring_type, registered_name, method)| {
                (registered_name == method_name
                    && self.is_assignable(&ClassRef::Named(declaring_type), &argument_type))
                .then_some(method)
            })
    }

    fn resolve_registered_candidate(
        &self,
        type_name: &str,
        method_name: &str,
        args: &[DataValue],
        is_static: bool,
        bean: &DataValue,
    ) -> Option<NativeMethod> {
        let native_type = self.get_type(type_name)?;
        let registered_name =
            Self::resolve_registered_method_name(&native_type, method_name, is_static);
        if !self.check_member(type_name, registered_name) {
            return None;
        }
        let candidates = if is_static {
            native_type.static_method_candidates.get(registered_name)
        } else {
            native_type.method_candidates.get(registered_name)
        };
        if let Some(candidates) = candidates {
            if let Some(candidate) = self.select_method_candidate(candidates, args) {
                return Some(wrap_method_candidate(candidate));
            }
        }

        let legacy = if is_static {
            native_type.static_methods.get(registered_name)
        } else {
            native_type.methods.get(registered_name)
        };
        if let Some(method) = legacy {
            return Some(Rc::clone(method));
        }

        // Java 从实际声明类开始逐层查找；当前类有同名候选但不匹配时，
        // 继续父类，保留 override/hiding 与 fallback 的组合语义。
        for supertype in &native_type.supertypes {
            if let Some(method) =
                self.resolve_registered_candidate(supertype, method_name, args, is_static, bean)
            {
                return Some(method);
            }
        }
        let _ = bean;
        None
    }

    fn select_method_candidate<'a>(
        &self,
        candidates: &'a [NativeMethodCandidate],
        args: &[DataValue],
    ) -> Option<&'a NativeMethodCandidate> {
        let signatures: Vec<(Vec<ClassRef>, bool)> = candidates
            .iter()
            .map(|candidate| (candidate.parameter_types.clone(), candidate.var_args))
            .collect();
        let arg_types = crate::utils::basic_util::BasicUtil::get_type_of_object(args);
        let index = MemberResolver::resolve_candidate_index_with_function_interface(
            &signatures,
            &arg_types,
            |param, arg| self.is_assignable(param, arg),
            |param| self.is_function_interface(param),
        )?;
        candidates.get(index)
    }

    fn select_constructor_candidate<'a>(
        &self,
        candidates: &'a [NativeConstructorCandidate],
        args: &[DataValue],
    ) -> Option<&'a NativeConstructorCandidate> {
        let signatures: Vec<(Vec<ClassRef>, bool)> = candidates
            .iter()
            .map(|candidate| (candidate.parameter_types.clone(), candidate.var_args))
            .collect();
        let arg_types = crate::utils::basic_util::BasicUtil::get_type_of_object(args);
        let index = MemberResolver::resolve_constructor(
            &signatures,
            &arg_types,
            |param, arg| self.is_assignable(param, arg),
            |param| self.is_function_interface(param),
        )?;
        candidates.get(index)
    }

    /// 判断形参类型是否为函数式接口。
    ///
    /// JDK 内建函数接口按规范名识别；宿主自定义接口由 [`NativeType`] 的
    /// `is_interface + abstract_methods` 元数据判定并通过 [`CacheUtil`]
    /// 缓存。对应 Java `CacheUtil.isFunctionInterface(Class<?>)`。
    fn is_function_interface(&self, class_ref: &ClassRef) -> bool {
        let name = class_ref.java_name();
        if name.starts_with("java.util.function.") || name == "java.lang.Runnable" {
            return true;
        }
        self.get_type(name).is_some_and(|native_type| {
            self.function_interface_cache
                .is_function_interface(&native_type)
        })
    }

    fn is_assignable(&self, param: &ClassRef, arg: &ClassRef) -> bool {
        if param == arg || param.is_java_object() {
            return true;
        }
        let param_name = param.java_name();
        let arg_name = arg.java_name();
        if builtin_assignable(param_name, arg_name) {
            return true;
        }
        if let Some(arg_item) = arg.component_type() {
            if matches!(param_name, "java.lang.Cloneable" | "java.io.Serializable") {
                return true;
            }
            if let Some(param_item) = param.component_type() {
                // Java 原语数组不协变；引用数组才递归执行组件可赋值判断。
                if matches!(param_item, ClassRef::Primitive(_))
                    || matches!(arg_item, ClassRef::Primitive(_))
                {
                    return param_item == arg_item;
                }
                return self.is_assignable(&param_item, &arg_item);
            }
        }
        self.type_extends(arg_name, param_name, &mut Vec::new())
    }

    /// 判断运行时值是否可赋给完整 Java 类型引用。
    ///
    /// 对应 Java `Class#isInstance`，并保留 Lambda 到函数式接口的代理适配。
    pub fn is_value_assignable(&self, target: &ClassRef, value: &DataValue) -> bool {
        if value.is_null() || target.is_java_object() {
            return true;
        }
        if matches!(value, DataValue::Lambda(_)) && self.is_function_interface(target) {
            return true;
        }
        self.is_assignable(target, &runtime_class_ref(value))
    }

    fn type_extends(
        &self,
        type_name: &str,
        expected_supertype: &str,
        visited: &mut Vec<String>,
    ) -> bool {
        if type_name == expected_supertype {
            return true;
        }
        if visited.iter().any(|visited_name| visited_name == type_name) {
            return false;
        }
        visited.push(type_name.to_string());
        self.get_type(type_name).is_some_and(|native_type| {
            native_type.supertypes.iter().any(|supertype| {
                supertype == expected_supertype
                    || self.type_extends(supertype, expected_supertype, visited)
            })
        })
    }
}
