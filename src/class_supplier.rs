//! Native type supply, replacing Java `ClassSupplier`/`DefaultClassSupplier`
//! with the explicit-registration approach of SPEC §4/§6 (`NativeRegistry`).
//!
//! The executable registry (constructors/methods/fields, used by
//! GetField/GetMethod/NewInstance/Cast) lives in `runtime/member.rs`;
//! re-exported here so host integrations have a single door.

use std::collections::HashSet;

pub use crate::runtime::member::{
    ClassRef, MetaClass, NativeConstructor, NativeFieldGetter, NativeMethod, NativeRegistry,
    NativeType,
};

/// How scripts resolve type names, mirroring Java `ClassSupplier`
/// (`loadCls`). Rust has no classpath; a supplier answers whether a type
/// name is known to the host.
pub trait ClassSupplier {
    /// Java `loadCls(String)`: the canonical name when the class is known,
    /// `None` otherwise.
    fn load_cls(&self, qualified_name: &str) -> Option<String>;
}

/// Default supplier, mirroring Java `DefaultClassSupplier`: backed by an
/// explicit set of registered type names instead of `Class.forName`.
#[derive(Clone, Debug, Default)]
pub struct DefaultClassSupplier {
    registered: HashSet<String>,
}

impl DefaultClassSupplier {
    /// Java `DefaultClassSupplier.getInstance()` — an empty supplier.
    pub fn instance() -> Self {
        DefaultClassSupplier::default()
    }

    /// Register a loadable type name (Rust replacement for classpath
    /// visibility).
    pub fn register(&mut self, qualified_name: impl Into<String>) {
        self.registered.insert(qualified_name.into());
    }
}

impl ClassSupplier for DefaultClassSupplier {
    fn load_cls(&self, qualified_name: &str) -> Option<String> {
        self.registered.get(qualified_name).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registered_names_load() {
        let mut supplier = DefaultClassSupplier::instance();
        assert_eq!(supplier.load_cls("java.lang.String"), None);
        supplier.register("java.lang.String");
        assert_eq!(
            supplier.load_cls("java.lang.String"),
            Some("java.lang.String".to_string())
        );
    }
}
