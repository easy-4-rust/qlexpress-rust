//! Instruction-generation scope chain, mirroring Java `GeneratorScope`.
//!
//! Java links scopes via a parent pointer and mutates the current scope
//! (`defineMacro`) while compiling; the Rust port uses an `Rc` chain plus
//! `RefCell` interior mutability so shared sub-scopes can be defined into
//! through the same handle the visitor holds.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::macro_define::MacroDefine;

/// `GeneratorScope` 结构体的 Rust 实现，保留对应对象的领域职责与公开契约。
/// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/GeneratorScope.java`；具体对象路径见 `docs/对象级对照表.md`。
/// Java `GeneratorScope`. Macro lookup walks the parent chain.
#[derive(Debug)]
pub struct GeneratorScope<I> {
    parent: Option<Rc<GeneratorScope<I>>>,
    name: String,
    macro_define_map: RefCell<HashMap<String, MacroDefine<I>>>,
}

impl<I> GeneratorScope<I> {
    /// 创建对象实例。
    /// 参数：`name`、`parent`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/GeneratorScope.java`，构造器 `<init>`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new GeneratorScope(name, parent)`.
    pub fn new(name: impl Into<String>, parent: Option<Rc<GeneratorScope<I>>>) -> Self {
        GeneratorScope {
            parent,
            name: name.into(),
            macro_define_map: RefCell::new(HashMap::new()),
        }
    }

    /// 附加 macros 配置并返回新值。
    /// 参数：`parent`、`name`、`macro_define_map`；返回：`Self`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/GeneratorScope.java`，方法 `withMacros`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `new GeneratorScope(parent, name, macroDefineMap)`.
    pub fn with_macros(
        parent: Option<Rc<GeneratorScope<I>>>,
        name: impl Into<String>,
        macro_define_map: HashMap<String, MacroDefine<I>>,
    ) -> Self {
        GeneratorScope {
            parent,
            name: name.into(),
            macro_define_map: RefCell::new(macro_define_map),
        }
    }

    /// 添加或注册 macro if absent。
    /// 参数：`name`、`macro_define`；返回：`bool`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/GeneratorScope.java`，方法 `defineMacroIfAbsent`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `defineMacroIfAbsent`: true when defined, false if the macro
    /// name already exists in *this* scope.
    pub fn define_macro_if_absent(
        &self,
        name: impl Into<String>,
        macro_define: MacroDefine<I>,
    ) -> bool {
        match self.macro_define_map.borrow_mut().entry(name.into()) {
            std::collections::hash_map::Entry::Occupied(_) => false,
            std::collections::hash_map::Entry::Vacant(slot) => {
                slot.insert(macro_define);
                true
            }
        }
    }

    /// 添加或注册 macro。
    /// 参数：`name`、`macro_define`；返回：无。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/GeneratorScope.java`，方法 `defineMacro`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `defineMacro`.
    pub fn define_macro(&self, name: impl Into<String>, macro_define: MacroDefine<I>) {
        self.macro_define_map
            .borrow_mut()
            .insert(name.into(), macro_define);
    }

    /// 查询 macro instructions。
    /// 参数：`macro_name`；返回：`Option<MacroDefine<I>>`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/runtime/instruction/QLInstruction.java`，方法 `getMacroInstructions`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getMacroInstructions`: search this scope then parents.
    ///
    /// Returns a clone of the definition (Java returns the shared object;
    /// `MacroDefine<I>` is cheap to clone when `I` is reference-counted).
    pub fn get_macro_instructions(&self, macro_name: &str) -> Option<MacroDefine<I>>
    where
        I: Clone,
    {
        if let Some(define) = self.macro_define_map.borrow().get(macro_name) {
            return Some(define.clone());
        }
        self.parent
            .as_ref()
            .and_then(|parent| parent.get_macro_instructions(macro_name))
    }

    /// 处理 name 对应的领域职责。
    /// 无显式参数；返回：`&str`。
    /// 对应或承接 Java 源文件：`com/alibaba/qlexpress4/aparser/GeneratorScope.java`，方法 `name`；Rust 侧按所有权与 `Result` 语义适配。
    /// Java `getName`.
    pub fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn macro_lookup_walks_parent_chain() {
        let root = GeneratorScope::new("root", None);
        assert!(root.define_macro_if_absent("m", MacroDefine::new(vec![1], false)));
        assert!(!root.define_macro_if_absent("m", MacroDefine::new(vec![2], false)));
        let root = Rc::new(root);

        let child = GeneratorScope::<i32>::new("child", Some(Rc::clone(&root)));
        assert_eq!(
            child
                .get_macro_instructions("m")
                .map(|d| d.macro_instructions().to_vec()),
            Some(vec![1])
        );
        assert!(child.get_macro_instructions("missing").is_none());
        assert_eq!(child.name(), "child");
    }

    #[test]
    fn child_definition_shadows_parent() {
        let root = GeneratorScope::new("root", None);
        root.define_macro("m", MacroDefine::new(vec![1], false));
        let root = Rc::new(root);
        let child = GeneratorScope::<i32>::new("child", Some(root));
        child.define_macro("m", MacroDefine::new(vec![2], true));
        assert_eq!(
            child
                .get_macro_instructions("m")
                .map(|d| d.macro_instructions().to_vec()),
            Some(vec![2])
        );
    }
}
