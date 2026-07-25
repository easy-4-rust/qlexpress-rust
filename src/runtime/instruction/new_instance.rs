//! Construction instructions, mirroring Java `NewInstanceInstruction`,
//! `NewFilledInstanceInstruction`, `NewArrayInstruction`,
//! `MultiNewArrayInstruction`, `NewListInstruction`, `NewMapInstruction`.

use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::ql_result::QResult;
use crate::runtime::data::convert::obj_type_convertor::{ObjTypeConvertor, TargetType};
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::member::ClassRef;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;

/// Operation: new an object of specified class
/// Input: ${argNum} + 1
/// Output: 1
///
/// Mirrors Java `NewInstanceInstruction`.
pub struct NewInstanceInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    new_clz: ClassRef,
    arg_num: usize,
}

impl NewInstanceInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, new_clz: ClassRef, arg_num: usize) -> Self {
        NewInstanceInstruction {
            error_reporter,
            new_clz,
            arg_num,
        }
    }

    pub fn new_clz(&self) -> &ClassRef {
        &self.new_clz
    }

    pub fn arg_num(&self) -> usize {
        self.arg_num
    }
}

impl QLInstruction for NewInstanceInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let objs: Vec<DataValue> = if self.arg_num == 0 {
            Vec::new()
        } else {
            q_context.pop_n(self.arg_num).values()
        };
        let Some(constructor) = q_context.registry().load_constructor(&self.new_clz) else {
            let param_types = objs
                .iter()
                .map(|o| o.data_type_name().to_string())
                .collect::<Vec<_>>()
                .join(", ");
            return Err(self.error_reporter.report_format(
                error_codes::NO_SUITABLE_CONSTRUCTOR,
                error_codes::error_msg(error_codes::NO_SUITABLE_CONSTRUCTOR),
                &[format!("[{param_types}]")],
            ));
        };
        // Java: InvocationTargetException → INVOKE_CONSTRUCTOR_INNER_ERROR,
        // other reflection failures → INVOKE_CONSTRUCTOR_UNKNOWN_ERROR. The
        // registry constructor reports `QLException` directly; an
        // uncoded inner failure is normalised to INVOKE_CONSTRUCTOR_INNER_ERROR.
        let new_object = constructor(&objs).map_err(|err| {
            if err.error_code() == error_codes::INVOKE_CONSTRUCTOR_INNER_ERROR
                || err.error_code() == error_codes::INVOKE_CONSTRUCTOR_UNKNOWN_ERROR
            {
                err
            } else {
                self.error_reporter.report_with_catch(
                    err.catch_obj().cloned(),
                    error_codes::INVOKE_CONSTRUCTOR_INNER_ERROR,
                    error_codes::error_msg(error_codes::INVOKE_CONSTRUCTOR_INNER_ERROR),
                )
            }
        })?;
        q_context.push(QValue::Data(new_object));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.arg_num as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!(
                "{}: New instance of cls {} with argNum {}",
                index,
                self.new_clz.simple_name(),
                self.arg_num
            ),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: new a instance with fields filled by top ${keys.length} stack
/// element
/// Input: ${keys.length}
/// Output: 1
///
/// Mirrors Java `NewFilledInstanceInstruction`.
pub struct NewFilledInstanceInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    new_cls: ClassRef,
    keys: Vec<String>,
}

impl NewFilledInstanceInstruction {
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        new_cls: ClassRef,
        keys: Vec<String>,
    ) -> Self {
        NewFilledInstanceInstruction {
            error_reporter,
            new_cls,
            keys,
        }
    }

    pub fn new_cls(&self) -> &ClassRef {
        &self.new_cls
    }

    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// Java `newInstance`: zero-arg constructor.
    fn new_instance(&self, q_context: &dyn QContext) -> Result<DataValue, QLException> {
        let Some(constructor) = q_context.registry().load_constructor(&self.new_cls) else {
            return Err(self.error_reporter.report(
                error_codes::INVOKE_CONSTRUCTOR_UNKNOWN_ERROR,
                error_codes::error_msg(error_codes::INVOKE_CONSTRUCTOR_UNKNOWN_ERROR),
            ));
        };
        constructor(&[]).map_err(|err| {
            self.error_reporter.report_with_catch(
                err.catch_obj().cloned(),
                error_codes::INVOKE_CONSTRUCTOR_INNER_ERROR,
                error_codes::error_msg(error_codes::INVOKE_CONSTRUCTOR_INNER_ERROR),
            )
        })
    }
}

impl QLInstruction for NewFilledInstanceInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let instance = self.new_instance(q_context)?;
        let init_items = q_context.pop_n(self.keys.len());
        for (i, field_name) in self.keys.iter().enumerate() {
            let init_value = init_items.get_value(i);
            let Some(field_value) = q_context.registry().load_field(&instance, field_name) else {
                // ignore field that don't exist
                continue;
            };
            let Some(left_value) = field_value.as_left() else {
                return Err(self.error_reporter.report_format(
                    error_codes::INVALID_ASSIGNMENT,
                    error_codes::error_msg(error_codes::INVALID_ASSIGNMENT),
                    &[format!("of field '{field_name}'")],
                ));
            };
            left_value
                .borrow_mut()
                .set(init_value, &*self.error_reporter)?;
        }
        q_context.push(QValue::Data(instance));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.keys.len() as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!(
                "{}: New instace of cls {} with fields [{}]",
                index,
                self.new_cls.simple_name(),
                self.keys.join(", ")
            ),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// new int[] {1,2,3}
/// Operation: new array with init items
/// Input: ${length}
/// Output: 1
///
/// Mirrors Java `NewArrayInstruction`.
pub struct NewArrayInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    clz: TargetType,
    length: usize,
}

impl NewArrayInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, clz: TargetType, length: usize) -> Self {
        NewArrayInstruction {
            error_reporter,
            clz,
            length,
        }
    }

    pub fn clz(&self) -> TargetType {
        self.clz
    }

    pub fn length(&self) -> usize {
        self.length
    }
}

impl QLInstruction for NewArrayInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        if !ql_options.check_arr_len(self.length as i32) {
            return Err(self.error_reporter.report_format(
                error_codes::EXCEED_MAX_ARR_LENGTH,
                error_codes::error_msg(error_codes::EXCEED_MAX_ARR_LENGTH),
                &[
                    self.length.to_string(),
                    ql_options.max_arr_length().to_string(),
                ],
            ));
        }
        let init_items = q_context.pop_n(self.length);
        let mut array = Vec::with_capacity(self.length);
        for i in 0..init_items.size() {
            let init_item_obj = init_items.get_value(i);
            let ql_convert_result = ObjTypeConvertor::cast(&init_item_obj, self.clz);
            if !ql_convert_result.is_convertible() {
                return Err(self.error_reporter.report_format(
                    error_codes::INCOMPATIBLE_ARRAY_ITEM_TYPE,
                    error_codes::error_msg(error_codes::INCOMPATIBLE_ARRAY_ITEM_TYPE),
                    &[
                        i.to_string(),
                        if init_item_obj.is_null() {
                            "null".to_string()
                        } else {
                            init_item_obj.data_type_name().to_string()
                        },
                        self.clz.java_name().to_string(),
                    ],
                ));
            }
            array.push(ql_convert_result.into_converted());
        }
        q_context.push(QValue::Data(DataValue::array(array)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.length as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: NewArray with length {}", index, self.length),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// new int[1][2][][]
/// Operation: new array with multi dims
/// Input: ${dims}
/// Output: 1
///
/// Mirrors Java `MultiNewArrayInstruction`. Rust arrays are untyped
/// `Vec<DataValue>`; extra dimensions become nested arrays filled with
/// `Null` (Java: zero-initialised multi-dimensional arrays).
pub struct MultiNewArrayInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    clz: TargetType,
    dims: usize,
}

impl MultiNewArrayInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, clz: TargetType, dims: usize) -> Self {
        MultiNewArrayInstruction {
            error_reporter,
            clz,
            dims,
        }
    }

    pub fn clz(&self) -> TargetType {
        self.clz
    }

    pub fn dims(&self) -> usize {
        self.dims
    }

    /// Java `Array.newInstance(clz, dims...)`: nested arrays, leaf elements
    /// zero-initialised (here `Null`, since script arrays are untyped).
    fn build_array(dims: &[i64]) -> DataValue {
        match dims.split_first() {
            None => DataValue::Null,
            Some((&len, rest)) => DataValue::array(
                (0..len.max(0)).map(|_| Self::build_array(rest)).collect(),
            ),
        }
    }
}

impl QLInstruction for MultiNewArrayInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let dim_values = q_context.pop_n(self.dims);
        let mut dim_array = Vec::with_capacity(self.dims);
        for i in 0..self.dims {
            let dim_value = dim_values.get_value(i);
            if !dim_value.is_number() {
                return Err(self.error_reporter.report(
                    error_codes::ARRAY_SIZE_NUM_REQUIRED,
                    error_codes::error_msg(error_codes::ARRAY_SIZE_NUM_REQUIRED),
                ));
            }
            let dim_len = crate::runtime::data::convert::to_i64(&dim_value);
            if !ql_options.check_arr_len(dim_len as i32) {
                return Err(self.error_reporter.report_format(
                    error_codes::EXCEED_MAX_ARR_LENGTH,
                    error_codes::error_msg(error_codes::EXCEED_MAX_ARR_LENGTH),
                    &[
                        dim_len.to_string(),
                        ql_options.max_arr_length().to_string(),
                    ],
                ));
            }
            dim_array.push(dim_len);
        }
        q_context.push(QValue::Data(Self::build_array(&dim_array)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.dims as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: MultiNewArray with dims {}", index, self.dims),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: new a List with top ${initLength} stack element
/// Input: ${initLength}
/// Output: 1
///
/// Mirrors Java `NewListInstruction`.
pub struct NewListInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    init_length: usize,
}

impl NewListInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, init_length: usize) -> Self {
        NewListInstruction {
            error_reporter,
            init_length,
        }
    }

    pub fn init_length(&self) -> usize {
        self.init_length
    }
}

impl QLInstruction for NewListInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let init_items = q_context.pop_n(self.init_length);
        let list = init_items.values();
        q_context.push(QValue::Data(DataValue::list(list)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.init_length as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: NewList {}", index, self.init_length),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

/// Operation: new a Map with top ${keys.length} stack element
/// Input: ${keys.length}
/// Output: 1
///
/// Mirrors Java `NewMapInstruction`.
pub struct NewMapInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    keys: Vec<String>,
}

impl NewMapInstruction {
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, keys: Vec<String>) -> Self {
        NewMapInstruction {
            error_reporter,
            keys,
        }
    }

    pub fn keys(&self) -> &[String] {
        &self.keys
    }
}

impl QLInstruction for NewMapInstruction {
    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let init_items = q_context.pop_n(self.keys.len());
        let mut map = IndexMap::new();
        for (i, key) in self.keys.iter().enumerate() {
            map.insert(DataValue::Str(key.clone()), init_items.get_value(i));
        }
        q_context.push(QValue::Data(DataValue::map(map)));
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        self.keys.len() as i32
    }

    fn stack_output(&self) -> i32 {
        1
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(
            depth as i32,
            &format!("{}: NewMap by keys [{}]", index, self.keys.join(", ")),
            debug,
        );
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}
