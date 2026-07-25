//! Global scope, mirroring Java `com.alibaba.qlexpress4.runtime.QvmGlobalScope`.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::runtime::data::index_map::IndexMap;
use crate::runtime::data::{AssignableDataValue, MapItemValue};
use crate::runtime::function::CustomFunction;
use crate::runtime::left_value::LeftValue;
use crate::runtime::value::DataValue;

/// Root scope holding external variables/functions and variables created at
/// global level, mirroring Java `QvmGlobalScope`.
///
/// The Java version delegates external lookup to `ExpressContext`
/// (`context/` is Stage 5); here the external context is represented by an
/// ordered map shared by `Rc`, which behaves like Java's
/// `MapExpressContext` (external entries are assignable `MapItemValue`s).
pub struct QvmGlobalScope {
    /// External variables (Java `ExpressContext externalVariable`).
    external_variables: Rc<RefCell<IndexMap>>,
    /// Variables first mentioned in the script (Java `newVariables`).
    new_variables: HashMap<String, Rc<RefCell<dyn LeftValue>>>,
    /// External (host-registered) functions (Java `externalFunction`).
    external_functions: HashMap<String, Rc<dyn CustomFunction>>,
    /// Java `qlOptions.isPolluteUserContext()`, consulted per lookup.
    pollute_user_context: bool,
}

impl QvmGlobalScope {
    pub fn new(
        external_variables: Rc<RefCell<IndexMap>>,
        external_functions: HashMap<String, Rc<dyn CustomFunction>>,
        pollute_user_context: bool,
    ) -> Self {
        QvmGlobalScope {
            external_variables,
            new_variables: HashMap::new(),
            external_functions,
            pollute_user_context,
        }
    }

    /// An empty global scope (no external variables/functions).
    pub fn empty() -> Self {
        Self::new(
            Rc::new(RefCell::new(IndexMap::new())),
            HashMap::new(),
            false,
        )
    }

    /// Java `getSymbol`: script-defined variables win; otherwise the
    /// external variable is returned directly when `polluteUserContext`,
    /// else its current value is copied into a new script variable.
    pub fn get_symbol(&mut self, var_name: &str) -> Rc<RefCell<dyn LeftValue>> {
        if let Some(new_variable) = self.new_variables.get(var_name) {
            return Rc::clone(new_variable);
        }
        let has_external = self
            .external_variables
            .borrow()
            .contains_key(&DataValue::Str(var_name.to_string()));
        let external_value = if has_external {
            let map_item: Rc<RefCell<dyn LeftValue>> = Rc::new(RefCell::new(MapItemValue::new(
                Rc::clone(&self.external_variables),
                DataValue::Str(var_name.to_string()),
            )));
            Some(map_item)
        } else {
            None
        };
        if let Some(external) = external_value {
            if self.pollute_user_context {
                return external;
            }
            let initial = external.borrow().get();
            let new_variable: Rc<RefCell<dyn LeftValue>> =
                Rc::new(RefCell::new(AssignableDataValue::new(var_name, initial)));
            self.new_variables
                .insert(var_name.to_string(), Rc::clone(&new_variable));
            new_variable
        } else {
            let new_variable: Rc<RefCell<dyn LeftValue>> = Rc::new(RefCell::new(
                AssignableDataValue::new(var_name, DataValue::Null),
            ));
            self.new_variables
                .insert(var_name.to_string(), Rc::clone(&new_variable));
            new_variable
        }
    }

    /// Java `defineLocalSymbol`: unsupported at global scope.
    pub fn define_local_symbol(&mut self, _var_name: &str) -> ! {
        panic!("UnsupportedOperationException: defineLocalSymbol on QvmGlobalScope")
    }

    /// Java `defineFunction`: unsupported at global scope.
    pub fn define_function(&mut self, _function_name: &str) -> ! {
        panic!("UnsupportedOperationException: defineFunction on QvmGlobalScope")
    }

    /// Java `getFunction`: only external functions are visible here.
    pub fn get_function(&self, function_name: &str) -> Option<Rc<dyn CustomFunction>> {
        self.external_functions.get(function_name).cloned()
    }

    /// Java `getFunctionTable`.
    pub fn function_table(&self) -> &HashMap<String, Rc<dyn CustomFunction>> {
        &self.external_functions
    }

    /// Script-created variables (Java `newVariables`).
    pub fn new_variables(&self) -> &HashMap<String, Rc<RefCell<dyn LeftValue>>> {
        &self.new_variables
    }

    /// The shared external variable context.
    pub fn external_variables(&self) -> &Rc<RefCell<IndexMap>> {
        &self.external_variables
    }
}
