//! 多维数组创建指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.MultiNewArrayInstruction`。
//! 职责:创建多维数组。
//! 本文件由 `new_instance.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::convert::obj_type_convertor::TargetType;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 多维数组创建指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.MultiNewArrayInstruction(职责:创建多维数组)
/// 示例：`new int[1][2][][]`。
/// 操作：创建多维数组；输入栈元素数为 `dims`，输出一个数组值。
///
/// Mirrors Java `MultiNewArrayInstruction`，每一层数组保存其声明组件类型，
/// 叶子元素按 Java 原语/引用默认值初始化。
pub struct MultiNewArrayInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    clz: ClassRef,
    dims: usize,
}

impl MultiNewArrayInstruction {
    /// 构造指令,对应 Java 构造器 `MultiNewArrayInstruction`。
    pub fn new(error_reporter: Rc<dyn ErrorReporter>, clz: ClassRef, dims: usize) -> Self {
        MultiNewArrayInstruction {
            error_reporter,
            clz,
            dims,
        }
    }

    /// 对应 Java 方法 `clz`。
    pub fn clz(&self) -> &ClassRef {
        &self.clz
    }

    /// 对应 Java 方法 `dims`。
    pub fn dims(&self) -> usize {
        self.dims
    }

    /// Java `Array.newInstance(clz, dims...)`。
    fn build_array(
        clz: &ClassRef,
        dims: &[i64],
        registry: Rc<crate::runtime::native_registry::NativeRegistry>,
    ) -> DataValue {
        let (&len, rest) = dims
            .split_first()
            .expect("MultiNewArrayInstruction requires at least one dimension");
        let component_type = wrap_array_type(clz, rest.len());
        let values = if rest.is_empty() {
            (0..len)
                .map(|_| default_array_item(clz))
                .collect::<Vec<_>>()
        } else {
            (0..len)
                .map(|_| Self::build_array(clz, rest, Rc::clone(&registry)))
                .collect::<Vec<_>>()
        };
        DataValue::array_with_type(values, component_type, registry)
    }
}

impl QLInstruction for MultiNewArrayInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        if self.dims == 0 {
            return Err(QLException::host_error(
                crate::exception::QLExceptionKind::Runtime,
                "java.lang.IllegalArgumentException",
                "java.lang.IllegalArgumentException",
            ));
        }
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
            if dim_len < 0 {
                return Err(QLException::host_error(
                    crate::exception::QLExceptionKind::Runtime,
                    "java.lang.NegativeArraySizeException",
                    "java.lang.NegativeArraySizeException",
                ));
            }
            if !ql_options.check_arr_len(dim_len as i32) {
                return Err(self.error_reporter.report_format(
                    error_codes::EXCEED_MAX_ARR_LENGTH,
                    error_codes::error_msg(error_codes::EXCEED_MAX_ARR_LENGTH),
                    &[dim_len.to_string(), ql_options.max_arr_length().to_string()],
                ));
            }
            dim_array.push(dim_len);
        }
        if let Some(budget) = q_context.q_runtime().execution_budget() {
            let mut total_items = 0usize;
            let mut level_items = 1usize;
            for dim in &dim_array {
                let dim = usize::try_from(*dim).unwrap_or(usize::MAX);
                level_items = level_items.saturating_mul(dim);
                total_items = total_items.saturating_add(level_items);
            }
            budget.charge_collection_items(total_items)?;
        }
        q_context.push(QValue::Data(Self::build_array(
            &self.clz,
            &dim_array,
            Rc::clone(q_context.registry()),
        )));
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

fn wrap_array_type(component_type: &ClassRef, dimensions: usize) -> ClassRef {
    let mut result = component_type.clone();
    for _ in 0..dimensions {
        result = ClassRef::array_of(result);
    }
    result
}

fn default_array_item(component_type: &ClassRef) -> DataValue {
    match component_type {
        ClassRef::Primitive(TargetType::Boolean) => DataValue::Bool(false),
        ClassRef::Primitive(TargetType::Byte) => DataValue::Byte(0),
        ClassRef::Primitive(TargetType::Short) => DataValue::Short(0),
        ClassRef::Primitive(TargetType::Int) => DataValue::Int(0),
        ClassRef::Primitive(TargetType::Long) => DataValue::Long(0),
        ClassRef::Primitive(TargetType::Float) => DataValue::Float(0.0),
        ClassRef::Primitive(TargetType::Double) => DataValue::Double(0.0),
        ClassRef::Primitive(TargetType::Character) => DataValue::Char(0),
        // BigInteger/BigDecimal 是引用类型；Object/具名类同样初始化为 null。
        ClassRef::Primitive(TargetType::BigInteger)
        | ClassRef::Primitive(TargetType::BigDecimal)
        | ClassRef::Primitive(TargetType::Any)
        | ClassRef::Boxed(_)
        | ClassRef::Named(_) => DataValue::Null,
    }
}
