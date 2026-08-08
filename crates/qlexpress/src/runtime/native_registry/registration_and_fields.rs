impl NativeRegistry {
    /// 预置内建类型的注册表(SPEC §4:String/List/Map/数值 常用方法子集)。
    /// 对应 Java: 无（Rust 原生适配）。
    ///
    /// # Returns
    ///
    /// 返回已注册 QLExpress 内建 Java 类型锚点的注册表。
    pub fn with_builtins() -> Self {
        let registry = NativeRegistry::new();
        registry.register_builtin_types();
        registry
    }

    /// 注册类型。对应 Java `ClassSupplier.addClass` 一类的类型供给。
    ///
    /// # Arguments
    ///
    /// * `native_type` - 包含规范类型名及显式构造器、字段和方法的描述。
    pub fn register_type(&self, native_type: NativeType) {
        self.types
            .borrow_mut()
            .insert(native_type.name.clone(), Rc::new(native_type));
    }

    /// 按名取注册类型。对应 Java `Class.forName` 命中已供给类型。
    ///
    /// # Arguments
    ///
    /// * `name` - Java 规范类型名。
    ///
    /// # Returns
    ///
    /// 类型已经显式注册时返回共享只读快照，否则返回 `None`。该快照不会
    /// 阻止宿主继续向同一注册表登记其他类型或替换同名类型。
    pub fn get_type(&self, name: &str) -> Option<Rc<NativeType>> {
        self.types.borrow().get(name).map(Rc::clone)
    }

    /// 为指定类型追加(或覆盖)一个扩展函数。
    ///
    /// 对应 Java 方法 `ReflectLoader#addExtendFunction`；扩展函数在
    /// `StrategyIsolation` 判断之前解析，不属于受反射沙箱约束的成员。
    ///
    /// # Arguments
    ///
    /// * `type_name` - 被扩展类型的 Java 规范名。
    /// * `method_name` - 脚本调用的方法名。
    /// * `method` - 接收目标值和实参数组的宿主实现。
    pub fn register_method(
        &self,
        type_name: impl Into<String>,
        method_name: impl Into<String>,
        method: NativeMethod,
    ) {
        self.extension_methods
            .borrow_mut()
            .insert((type_name.into(), method_name.into()), method);
    }

    /// 按 Java `ExtensionFunction` 签名追加一个扩展函数候选。
    ///
    /// 对应 Java `ReflectLoader#addExtendFunction` 向
    /// `CopyOnWriteArrayList` 追加元素的行为；同名候选不会覆盖，解析时同时
    /// 考虑声明类型可赋值关系、精确参数、数值转换和 varargs。
    ///
    /// # 参数
    ///
    /// - `declaring_class`：扩展函数声明的接收者类型。
    /// - `method_name`：脚本中的实例方法名。
    /// - `candidate`：形参签名、varargs 标志与调用实现。
    pub fn register_extension_candidate(
        &self,
        declaring_class: ClassRef,
        method_name: impl Into<String>,
        candidate: NativeMethodCandidate,
    ) {
        self.extension_method_candidates.borrow_mut().push((
            declaring_class,
            method_name.into(),
            candidate,
        ));
    }

    /// 按 Java `@QLAlias` 语义把脚本方法名解析为注册表中的真实方法名。
    ///
    /// Java `MethodHandler` 会先枚举真实方法，再匹配方法上的别名；Rust
    /// 将注解元数据拍平到 `NativeType.method_aliases`，这里同时服务静态
    /// 方法和实例方法分派。
    fn resolve_registered_method_name<'a>(
        native_type: &'a NativeType,
        method_name: &'a str,
        is_static: bool,
    ) -> &'a str {
        let contains_method = |name: &str| {
            if is_static {
                native_type.static_methods.contains_key(name)
                    || native_type.static_method_candidates.contains_key(name)
            } else {
                native_type.methods.contains_key(name)
                    || native_type.method_candidates.contains_key(name)
            }
        };
        if contains_method(method_name) {
            return method_name;
        }
        native_type
            .method_aliases
            .iter()
            .find_map(|(registered_name, aliases)| {
                (contains_method(registered_name)
                    && aliases.iter().any(|alias| alias == method_name))
                .then_some(registered_name.as_str())
            })
            .unwrap_or(method_name)
    }

    // ---- 对应 Java ReflectLoader.loadConstructor ----

    /// 对应 Java 方法 `loadConstructor(Class, Class[])`:取注册构造器;
    /// 参数匹配委托给构造器闭包自身(Java 由 `MemberResolver` 选重载,
    /// Rust 一个类型只注册一个构造入口)。
    ///
    /// # Arguments
    ///
    /// * `clz` - 待实例化的类型引用。
    ///
    /// # Returns
    ///
    /// 安全策略允许且类型注册了兼容构造器时返回调用闭包。
    pub fn load_constructor(&self, clz: &ClassRef) -> Option<NativeConstructor> {
        if !self.check_member(clz.java_name(), "<init>") {
            return None;
        }
        self.get_type(clz.java_name())
            .and_then(|native_type| native_type.constructor.as_ref().map(Rc::clone))
    }

    /// 按实参类型选择构造器候选。没有候选元数据时兼容旧的单构造器注册。
    /// 对应 Java: 无（Rust 原生适配）。
    ///
    /// # Arguments
    ///
    /// * `clz` - 待实例化类型。
    /// * `args` - 用于 Java 重载匹配的运行时实参。
    ///
    /// # Returns
    ///
    /// 返回完成必要参数转换的最佳构造器，未授权或无匹配项时返回 `None`。
    pub fn load_constructor_for_args(
        &self,
        clz: &ClassRef,
        args: &[DataValue],
    ) -> Option<NativeConstructor> {
        if !self.check_member(clz.java_name(), "<init>") {
            return None;
        }
        let native_type = self.get_type(clz.java_name())?;
        if let Some(candidate) =
            self.select_constructor_candidate(&native_type.constructor_candidates, args)
        {
            let constructor = Rc::clone(&candidate.constructor);
            let parameter_types = candidate.parameter_types.clone();
            let var_args = candidate.var_args;
            return Some(Rc::new(move |values| {
                let converted = convert_candidate_arguments(values, &parameter_types, var_args);
                constructor(&converted)
            }));
        }
        if native_type.constructor_candidates.is_empty() {
            return native_type.constructor.as_ref().map(Rc::clone);
        }
        None
    }

    // ---- 对应 Java ReflectLoader.loadField ----

    /// 对应 Java 方法 `loadField(Object bean, String fieldName, boolean
    /// skipSecurity, ErrorReporter)`:字段不存在时返回 `None`(Java 返回 `null`)。
    ///
    /// # Arguments
    ///
    /// * `bean` - 字段接收者。
    /// * `field_name` - 字段名、Map 键或内建 `length`/`class` 名称。
    ///
    /// # Returns
    ///
    /// 安全策略允许且字段存在时返回可读或可写 QVM 值。
    pub fn load_field(&self, bean: &DataValue, field_name: &str) -> Option<QValue> {
        self.load_field_with_security(bean, field_name, false)
    }

    /// 加载字段，并按 Java `skipSecurity` 参数决定是否跳过成员策略。
    ///
    /// 对应 Java 方法 `ReflectLoader#loadField(Object, String, boolean,
    /// ErrorReporter)`；脚本指令传 `false`，`Express4Runner#loadField`
    /// 宿主 API 传 `true`。
    ///
    /// # Arguments
    ///
    /// * `bean` - 字段接收者。
    /// * `field_name` - 待解析字段名。
    /// * `skip_security` - 仅宿主 API 可用；为真时跳过成员安全策略。
    ///
    /// # Returns
    ///
    /// 字段可解析时返回对应 QVM 值，否则返回 `None`。
    pub fn load_field_with_security(
        &self,
        bean: &DataValue,
        field_name: &str,
        skip_security: bool,
    ) -> Option<QValue> {
        // Java 通用语义:任何对象都有 `.class`(`obj.getClass()`)。
        // 内建值按 `data_type_name` 还原类引用(原语名经
        // `ClassRef::from_name` 归一到与类字面量 `int` 等一致的
        // Primitive 目标,使 `c.class == int` 之类的比较成立);
        // MetaClass/宿主对象的 `.class` 由下方 Object 分支处理。
        // (对齐测试 cast/cast_express.ql 发现。)
        if field_name == basic_util::CLASS && !matches!(bean, DataValue::Object(_)) {
            let class_ref = ClassRef::from_name(bean.data_type_name());
            return Some(QValue::Data(MetaClass::new(class_ref).into_data_value()));
        }
        match bean {
            // Java 特殊分支:数组 length。
            DataValue::Array(arr) if field_name == basic_util::LENGTH => {
                Some(QValue::Data(DataValue::Int(arr.borrow().len() as i32)))
            }
            // Java 特殊分支:List length。
            DataValue::List(list) if field_name == basic_util::LENGTH => {
                Some(QValue::Data(DataValue::Int(list.borrow().len() as i32)))
            }
            // Java 特殊分支:Map 的字段访问即按 key 取条目(可写左值)。
            DataValue::Map(map) => Some(QValue::Left(Rc::new(RefCell::new(MapItemValue::new(
                Rc::clone(map),
                DataValue::string(field_name),
            ))))),
            DataValue::Object(obj) => {
                // Java 的 MetaClass 分支:`.class` 与静态字段。
                let meta_clz = {
                    let borrowed = obj.borrow();
                    borrowed
                        .as_any()
                        .downcast_ref::<MetaClass>()
                        .map(|meta| meta.clz().clone())
                };
                match meta_clz {
                    Some(clz) => {
                        if field_name == basic_util::CLASS {
                            // Java 返回 Class 对象本身;栈上最接近的值即 MetaClass 数据。
                            return Some(QValue::Data(bean.clone()));
                        }
                        let name = clz.java_name();
                        let native_type = self.get_type(name)?;
                        let registered_name = PreferredFieldHandler::gather_field_recursive(
                            &native_type,
                            field_name,
                        )?;
                        // 安全策略接线点(Java ReflectLoader.check):
                        // 静态字段访问前过 QLSecurityStrategy。
                        if skip_security || self.check_member(name, &registered_name) {
                            if let Some(cell) = native_type.static_field_cells.get(&registered_name)
                            {
                                let getter_cell = Rc::clone(cell);
                                let setter_cell = Rc::clone(cell);
                                return Some(QValue::Left(Rc::new(RefCell::new(FieldValue::new(
                                    Box::new(move || getter_cell.borrow().clone()),
                                    Box::new(move |value| {
                                        *setter_cell.borrow_mut() = value;
                                        true
                                    }),
                                    None,
                                )))));
                            }
                            if let Some(value) = native_type.static_fields.get(&registered_name) {
                                return Some(QValue::Data(value.clone()));
                            }
                        }
                        None
                    }
                    // Java:bean 字段/getter 反射读取 → NativeObject 显式读取。
                    None => {
                        let type_name = obj.borrow().native_type_name().to_string();
                        let registered_name = self
                            .get_type(&type_name)
                            .and_then(|native_type| {
                                PreferredFieldHandler::gather_field_recursive(
                                    &native_type,
                                    field_name,
                                )
                            })
                            .unwrap_or_else(|| field_name.to_string());
                        if !skip_security && !self.check_member(&type_name, &registered_name) {
                            return None;
                        }
                        if let Some(native_type) = self.get_type(&type_name) {
                            if let (Some(getter), Some(setter)) = (
                                native_type.fields.get(&registered_name),
                                native_type.field_setters.get(&registered_name),
                            ) {
                                let getter = Rc::clone(getter);
                                let setter = Rc::clone(setter);
                                let getter_bean = bean.clone();
                                let setter_bean = bean.clone();
                                return Some(QValue::Left(Rc::new(RefCell::new(FieldValue::new(
                                    Box::new(move || {
                                        getter(&getter_bean).unwrap_or(DataValue::Null)
                                    }),
                                    Box::new(move |value| setter(&setter_bean, &value)),
                                    None,
                                )))));
                            }
                            if let Some(value) = native_type
                                .fields
                                .get(&registered_name)
                                .and_then(|getter| getter(bean))
                            {
                                return Some(QValue::Data(value));
                            }
                            // Java 允许通过实例读取/写入 static Field（例如
                            // `SampleEnum.NORMAL.testStaticField`）；反射最终仍以
                            // declaring class 的同一共享字段为准。
                            if let Some(cell) = native_type.static_field_cells.get(&registered_name)
                            {
                                let getter_cell = Rc::clone(cell);
                                let setter_cell = Rc::clone(cell);
                                return Some(QValue::Left(Rc::new(RefCell::new(FieldValue::new(
                                    Box::new(move || getter_cell.borrow().clone()),
                                    Box::new(move |value| {
                                        *setter_cell.borrow_mut() = value;
                                        true
                                    }),
                                    None,
                                )))));
                            }
                            if let Some(value) = native_type.static_fields.get(&registered_name) {
                                return Some(QValue::Data(value.clone()));
                            }
                            // Rust 的显式注册表就是 Java 反射可见性边界：类型已
                            // 注册但成员未注册时，不得绕过注册表直读对象字段。
                            return None;
                        }
                        obj.borrow().get_field(&registered_name).map(QValue::Data)
                    }
                }
            }
            _ => {
                // 注册的实例字段(按 Java 类型名)。
                // 安全策略接线点:实例字段访问前过 QLSecurityStrategy。
                let type_name = bean.data_type_name();
                let native_type = self.get_type(type_name)?;
                let registered_name =
                    PreferredFieldHandler::gather_field_recursive(&native_type, field_name)?;
                if !skip_security && !self.check_member(type_name, &registered_name) {
                    return None;
                }
                if let (Some(getter), Some(setter)) = (
                    native_type.fields.get(&registered_name),
                    native_type.field_setters.get(&registered_name),
                ) {
                    let getter = Rc::clone(getter);
                    let setter = Rc::clone(setter);
                    let getter_bean = bean.clone();
                    let setter_bean = bean.clone();
                    return Some(QValue::Left(Rc::new(RefCell::new(FieldValue::new(
                        Box::new(move || getter(&getter_bean).unwrap_or(DataValue::Null)),
                        Box::new(move |value| setter(&setter_bean, &value)),
                        None,
                    )))));
                }
                native_type
                    .fields
                    .get(&registered_name)
                    .and_then(|getter| getter(bean))
                    .map(QValue::Data)
            }
        }
    }
}
