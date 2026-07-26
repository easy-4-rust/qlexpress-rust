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

/// Java `GeneratorScope`. Macro lookup walks the parent chain.
#[derive(Debug)]
pub struct GeneratorScope<I> {
    parent: Option<Rc<GeneratorScope<I>>>,
    name: String,
    macro_define_map: RefCell<HashMap<String, MacroDefine<I>>>,
}

impl<I> GeneratorScope<I> {
    /// Java `new GeneratorScope(name, parent)`.
    pub fn new(name: impl Into<String>, parent: Option<Rc<GeneratorScope<I>>>) -> Self {
        GeneratorScope {
            parent,
            name: name.into(),
            macro_define_map: RefCell::new(HashMap::new()),
        }
    }

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

    /// Java `defineMacro`.
    pub fn define_macro(&self, name: impl Into<String>, macro_define: MacroDefine<I>) {
        self.macro_define_map
            .borrow_mut()
            .insert(name.into(), macro_define);
    }

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
            child.get_macro_instructions("m").map(|d| d.macro_instructions().to_vec()),
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
            child.get_macro_instructions("m").map(|d| d.macro_instructions().to_vec()),
            Some(vec![2])
        );
    }
}
