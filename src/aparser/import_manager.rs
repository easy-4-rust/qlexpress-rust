//! Import declarations and the import-aware class resolver, mirroring Java
//! `ImportManager`.
//!
//! Since Rust has no classpath, "loaded classes" are represented by the
//! canonical name `String` returned from [`ClassSupplier::load_cls`]
//! (`Some(name)` plays the role of Java's non-null `Class<?>`).

/// Java `ImportManager.ImportScope`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ImportScope {
    /// `import java.lang.*;`
    Pack,
    /// `import a.b.Cls.*;` (inner class)
    InnerCls,
    /// `import java.lang.String;`
    Cls,
    /// `import java.lang.String as Str;`
    Alias,
}

/// One import declaration, mirroring Java `ImportManager.QLImport`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QLImport {
    scope: ImportScope,
    /// Import target: package path for `Pack`, class path for `Cls`/
    /// `InnerCls`/`Alias`.
    target: String,
    /// Alias name; only meaningful for [`ImportScope::Alias`].
    alias: Option<String>,
}

impl QLImport {
    /// Java `ImportManager.importPack(String)`.
    pub fn import_pack(pack_path: impl Into<String>) -> Self {
        QLImport {
            scope: ImportScope::Pack,
            target: pack_path.into(),
            alias: None,
        }
    }

    /// Java `ImportManager.importInnerCls(String)`.
    pub fn import_inner_cls(cls_path: impl Into<String>) -> Self {
        QLImport {
            scope: ImportScope::InnerCls,
            target: cls_path.into(),
            alias: None,
        }
    }

    /// Java `ImportManager.importCls(String)`.
    pub fn import_cls(cls_path: impl Into<String>) -> Self {
        QLImport {
            scope: ImportScope::Cls,
            target: cls_path.into(),
            alias: None,
        }
    }

    /// Java `ImportManager.importClsAlias(Class, String)`.
    ///
    /// Java throws `IllegalArgumentException` on a null class; the Rust
    /// equivalent validates an empty class path.
    pub fn import_cls_alias(cls_path: impl Into<String>, alias: impl Into<String>) -> Self {
        let cls_path = cls_path.into();
        assert!(!cls_path.is_empty(), "Class cannot be null");
        QLImport {
            scope: ImportScope::Alias,
            target: cls_path,
            alias: Some(alias.into()),
        }
    }

    pub fn scope(&self) -> ImportScope {
        self.scope
    }

    pub fn target(&self) -> &str {
        &self.target
    }

    pub fn alias(&self) -> Option<&str> {
        self.alias.as_deref()
    }
}

// ---------------------------------------------------------------------------
// ImportManager (class resolution against a ClassSupplier)
// ---------------------------------------------------------------------------

use std::collections::HashMap;

use crate::class_supplier::ClassSupplier;

/// Java `ImportManager`: tracks imported packages/classes and resolves
/// (possibly partial) qualified names.
pub struct ImportManager<'a> {
    class_supplier: &'a dyn ClassSupplier,
    imported_packs: Vec<QLImport>,
    /// Simple name (or alias) -> loaded canonical class name.
    imported_clses: HashMap<String, String>,
}

/// Java `ImportManager.LoadPartQualifiedResult`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadPartQualifiedResult {
    /// The loaded class canonical name (`None` plays Java's null Class).
    cls: Option<String>,
    /// First field index that is not part of the class path.
    rest_index: usize,
}

impl LoadPartQualifiedResult {
    pub fn new(cls: Option<String>, rest_index: usize) -> Self {
        LoadPartQualifiedResult { cls, rest_index }
    }

    /// Java `getCls`.
    pub fn cls(&self) -> Option<&str> {
        self.cls.as_deref()
    }

    /// Java `getRestIndex`.
    pub fn rest_index(&self) -> usize {
        self.rest_index
    }
}

impl<'a> ImportManager<'a> {
    /// Java `new ImportManager(classSupplier, imports)`.
    pub fn new(class_supplier: &'a dyn ClassSupplier, imports: Vec<QLImport>) -> Self {
        let mut manager = ImportManager {
            class_supplier,
            imported_packs: Vec::new(),
            imported_clses: HashMap::new(),
        };
        for an_import in imports {
            manager.add_import(an_import);
        }
        manager
    }

    /// Java `new ImportManager(classSupplier, importedPacks, importedClses)`.
    pub fn from_parts(
        class_supplier: &'a dyn ClassSupplier,
        imported_packs: Vec<QLImport>,
        imported_clses: HashMap<String, String>,
    ) -> Self {
        ImportManager {
            class_supplier,
            imported_packs,
            imported_clses,
        }
    }

    /// Java `addImport`. Returns false when a `Cls` import cannot be loaded
    /// (Java returns false when `loadCls` yields null).
    pub fn add_import(&mut self, an_import: QLImport) -> bool {
        match an_import.scope() {
            ImportScope::Pack | ImportScope::InnerCls => {
                self.imported_packs.push(an_import);
                true
            }
            ImportScope::Cls => {
                let Some(loaded) = self.class_supplier.load_cls(an_import.target()) else {
                    return false;
                };
                let simple_name = an_import
                    .target()
                    .rsplit('.')
                    .next()
                    .unwrap_or(an_import.target())
                    .to_string();
                self.imported_clses.insert(simple_name, loaded);
                true
            }
            ImportScope::Alias => {
                if let Some(alias) = an_import.alias() {
                    self.imported_clses
                        .insert(alias.to_string(), an_import.target().to_string());
                    true
                } else {
                    false
                }
            }
        }
    }

    /// Java `loadQualified`.
    pub fn load_qualified(&self, qualified_cls: &str) -> Option<String> {
        self.class_supplier.load_cls(qualified_cls)
    }

    /// Java `loadPartQualified`: resolve the class-path prefix of
    /// `field_ids`, returning the loaded class and the index of the first
    /// non-class-path field.
    pub fn load_part_qualified(&self, field_ids: &[String]) -> LoadPartQualifiedResult {
        const INIT: u8 = 0;
        const CONTINUE: u8 = 1;
        const LOAD_CLS: u8 = 2;
        const LOAD_INNER_CLS: u8 = 3;
        const PRE_LOAD_INNER_CLS: u8 = 4;

        let mut qualified_cls: Option<String> = None;
        let mut qualified_path: Option<Vec<String>> = None;
        let mut inner_cls_id: Option<String> = None;
        let mut state = INIT;

        'next_field: for (i, field_id) in field_ids.iter().enumerate() {
            match state {
                INIT => {
                    // load from imported classes
                    if let Some(a_cls) = self.imported_clses.get(field_id) {
                        qualified_cls = Some(a_cls.clone());
                        state = PRE_LOAD_INNER_CLS;
                        continue;
                    }
                    // load from imported packs
                    if !starts_lower(field_id) {
                        for imported_pack in &self.imported_packs {
                            match imported_pack.scope() {
                                ImportScope::Pack => {
                                    let candidate =
                                        format!("{}.{}", imported_pack.target(), field_id);
                                    if let Some(pack_cls) = self.class_supplier.load_cls(&candidate)
                                    {
                                        qualified_cls = Some(pack_cls);
                                        state = PRE_LOAD_INNER_CLS;
                                        continue 'next_field;
                                    }
                                }
                                ImportScope::InnerCls => {
                                    let candidate =
                                        format!("{}${}", imported_pack.target(), field_id);
                                    if let Some(inner_cls) = self.class_supplier.load_cls(&candidate)
                                    {
                                        qualified_cls = Some(inner_cls);
                                        state = PRE_LOAD_INNER_CLS;
                                        continue 'next_field;
                                    }
                                }
                                _ => {}
                            }
                        }
                        return LoadPartQualifiedResult::new(None, 0);
                    }
                    state = CONTINUE;
                    qualified_path = Some(vec![field_id.clone()]);
                }
                PRE_LOAD_INNER_CLS => {
                    if !starts_lower(field_id) {
                        state = LOAD_INNER_CLS;
                        inner_cls_id = Some(field_id.clone());
                    } else {
                        return LoadPartQualifiedResult::new(qualified_cls, i);
                    }
                }
                CONTINUE => {
                    if let Some(path) = &mut qualified_path {
                        path.push(field_id.clone());
                    }
                    if !starts_lower(field_id) {
                        state = LOAD_CLS;
                    }
                }
                LOAD_CLS => {
                    let path = qualified_path.clone().unwrap_or_default();
                    qualified_cls = self.class_supplier.load_cls(&path.join("."));
                    if qualified_cls.is_none() {
                        return LoadPartQualifiedResult::new(None, 0);
                    }
                    if !starts_lower(field_id) {
                        qualified_path = None;
                        inner_cls_id = Some(field_id.clone());
                        state = LOAD_INNER_CLS;
                    } else {
                        return LoadPartQualifiedResult::new(qualified_cls, i);
                    }
                }
                LOAD_INNER_CLS => {
                    let base = qualified_cls.clone().unwrap_or_default();
                    let inner = self
                        .class_supplier
                        .load_cls(&format!("{}${}", base, inner_cls_id.clone().unwrap_or_default()));
                    let Some(inner) = inner else {
                        return LoadPartQualifiedResult::new(qualified_cls, i.saturating_sub(1));
                    };
                    if !starts_lower(field_id) {
                        qualified_cls = Some(inner);
                        inner_cls_id = Some(field_id.clone());
                    } else {
                        return LoadPartQualifiedResult::new(Some(inner), i);
                    }
                }
                _ => unreachable!("unknown state"),
            }
        }

        match state {
            CONTINUE => LoadPartQualifiedResult::new(None, 0),
            LOAD_CLS => {
                let path = qualified_path.unwrap_or_default();
                let loaded = self.class_supplier.load_cls(&path.join("."));
                match loaded {
                    None => LoadPartQualifiedResult::new(None, field_ids.len()),
                    Some(cls) => LoadPartQualifiedResult::new(Some(cls), field_ids.len()),
                }
            }
            PRE_LOAD_INNER_CLS => LoadPartQualifiedResult::new(qualified_cls, field_ids.len()),
            LOAD_INNER_CLS => {
                let base = qualified_cls.clone().unwrap_or_default();
                let inner = self
                    .class_supplier
                    .load_cls(&format!("{}${}", base, inner_cls_id.unwrap_or_default()));
                match inner {
                    None => LoadPartQualifiedResult::new(qualified_cls, field_ids.len() - 1),
                    Some(cls) => LoadPartQualifiedResult::new(Some(cls), field_ids.len()),
                }
            }
            _ => LoadPartQualifiedResult::new(None, 0),
        }
    }
}

/// Java `Character.isLowerCase(fieldId.charAt(0))`.
fn starts_lower(field_id: &str) -> bool {
    field_id
        .chars()
        .next()
        .map(char::is_lowercase)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::class_supplier::DefaultClassSupplier;

    #[test]
    fn factory_methods_match_java() {
        let pack = QLImport::import_pack("java.lang");
        assert_eq!(pack.scope(), ImportScope::Pack);
        assert_eq!(pack.target(), "java.lang");

        let cls = QLImport::import_cls("java.lang.String");
        assert_eq!(cls.scope(), ImportScope::Cls);

        let alias = QLImport::import_cls_alias("java.lang.String", "Str");
        assert_eq!(alias.scope(), ImportScope::Alias);
        assert_eq!(alias.alias(), Some("Str"));
    }

    #[test]
    #[should_panic(expected = "Class cannot be null")]
    fn alias_rejects_empty_class() {
        let _ = QLImport::import_cls_alias("", "Str");
    }

    fn supplier() -> DefaultClassSupplier {
        let mut s = DefaultClassSupplier::instance();
        s.register("java.lang.String");
        s.register("java.util.HashMap");
        s.register("java.util.HashMap$Entry");
        s
    }

    #[test]
    fn cls_import_registers_simple_name() {
        let s = supplier();
        let mut m = ImportManager::new(&s, vec![]);
        assert!(m.add_import(QLImport::import_cls("java.lang.String")));
        assert!(!m.add_import(QLImport::import_cls("a.b.Missing")));
        let result = m.load_part_qualified(&["String".to_string()]);
        assert_eq!(result.cls(), Some("java.lang.String"));
        assert_eq!(result.rest_index(), 1);
    }

    #[test]
    fn pack_import_resolves_upper_case_field() {
        let s = supplier();
        let mut m = ImportManager::new(&s, vec![]);
        m.add_import(QLImport::import_pack("java.util"));
        let result = m.load_part_qualified(&["HashMap".to_string(), "size".to_string()]);
        assert_eq!(result.cls(), Some("java.util.HashMap"));
        assert_eq!(result.rest_index(), 1);
    }

    #[test]
    fn inner_cls_loads_with_dollar() {
        let s = supplier();
        let m = ImportManager::new(&s, vec![QLImport::import_cls("java.util.HashMap")]);
        let result = m.load_part_qualified(&["HashMap".to_string(), "Entry".to_string()]);
        assert_eq!(result.cls(), Some("java.util.HashMap$Entry"));
        assert_eq!(result.rest_index(), 2);
    }

    #[test]
    fn qualified_path_builds_until_upper_case() {
        let s = supplier();
        let m = ImportManager::new(&s, vec![]);
        let result = m.load_part_qualified(&[
            "java".to_string(),
            "util".to_string(),
            "HashMap".to_string(),
        ]);
        assert_eq!(result.cls(), Some("java.util.HashMap"));
        assert_eq!(result.rest_index(), 3);
    }
}
