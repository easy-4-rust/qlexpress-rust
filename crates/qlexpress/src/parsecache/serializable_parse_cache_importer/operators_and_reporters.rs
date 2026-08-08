impl<'a> SerializableParseCacheImporter<'a> {
    /// 对应 Java `binaryOperator`。
    fn binary_operator(
        &self,
        operator: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Rc<dyn crate::runtime::operator::BinaryOperator>> {
        self.operator_manager
            .get_binary_operator(operator)
            .ok_or_else(|| self.operator_not_found(owner, operator))
    }

    /// 对应 Java `prefixUnaryOperator`。
    fn prefix_unary_operator(
        &self,
        operator: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Rc<dyn crate::runtime::operator::UnaryOperator>> {
        self.operator_manager
            .get_prefix_unary_operator(operator)
            .ok_or_else(|| self.operator_not_found(owner, operator))
    }

    /// 对应 Java `suffixUnaryOperator`。
    fn suffix_unary_operator(
        &self,
        operator: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<Rc<dyn crate::runtime::operator::UnaryOperator>> {
        self.operator_manager
            .get_suffix_unary_operator(operator)
            .ok_or_else(|| self.operator_not_found(owner, operator))
    }

    /// 对应 Java `loadClass` + `primitiveClass`:原始类型名直接命中,
    /// 其余委托 [`ClassSupplier`](找不到即 `CLASS_NOT_FOUND`)。
    fn load_class(
        &self,
        class_name: &str,
        owner: Option<&SerializableInstruction>,
    ) -> ImportResult<ClassRef> {
        let parsed = ClassRef::from_name(class_name);
        if let Some(component) = parsed.component_type() {
            let loaded_component = self.load_class(component.java_name(), owner)?;
            return Ok(ClassRef::array_of(loaded_component));
        }
        // Java primitiveClass:boolean/byte/char/short/int/long/float/double/void
        match class_name {
            "boolean" | "byte" | "char" | "short" | "int" | "long" | "float" | "double" => {
                return Ok(ClassRef::from_name(class_name));
            }
            "void" => return Ok(ClassRef::Named("void".to_string())),
            _ => {}
        }
        // Java 包装类与 BigInteger/BigDecimal 可在无宿主 classpath 时确定。
        if let ClassRef::Boxed(_) = ClassRef::from_name(class_name) {
            return Ok(ClassRef::from_name(class_name));
        }
        // `java.lang.Object`:Java 的 Class.forName 恒可加载(Rust 无 classpath,
        // 编译器自身会为无类型参数/局部变量导出此名,故内建放行)
        if class_name == "java.lang.Object" {
            return Ok(ClassRef::Named(class_name.to_string()));
        }
        match self.class_supplier.load_cls(class_name) {
            Some(canonical) => Ok(ClassRef::from_name(&canonical)),
            None => Err(SerializableParseCacheException::new(
                Some(&self.script),
                owner.and_then(|inst| inst.source.as_ref()),
                error_codes::SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND,
                &error_codes::format_msg(
                    error_codes::error_msg(error_codes::SERIALIZABLE_PARSE_CACHE_CLASS_NOT_FOUND),
                    &[class_name.to_string()],
                ),
            )),
        }
    }

    /// 对应 Java `reporter(SerializableSource)`:
    /// line <= 0 归一为 1;col 取 max(0, col) + 1(转回 1 基)。
    fn reporter(&self, source: Option<&SerializableSource>) -> Rc<dyn ErrorReporter> {
        let default_source = SerializableSource::default();
        let normalized = source.unwrap_or(&default_source);
        let line = if normalized.line <= 0 {
            1
        } else {
            normalized.line
        };
        let col = normalized.col.max(0) + 1;
        Rc::new(DefaultErrReporter::new(
            self.script.clone(),
            normalized.start.max(0),
            line,
            col,
            normalized.lexeme.clone().unwrap_or_default(),
        ))
    }
}
