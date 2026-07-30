//! 带初始化 new 实例指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.NewFilledInstanceInstruction`。
//! 职责:创建实例并以初始值填充。
//! 本文件由 `new_instance.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::QLException;
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::member::ClassRef;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::value::{DataValue, QValue};
use crate::utils::println_utils::PrintlnUtils;
use std::rc::Rc;

/// 带初始化 new 实例指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.NewFilledInstanceInstruction(职责:创建实例并以初始值填充)
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
    /// 构造指令,对应 Java 构造器 `NewFilledInstanceInstruction`。
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

    /// 对应 Java 方法 `newCls`。
    pub fn new_cls(&self) -> &ClassRef {
        &self.new_cls
    }

    /// 对应 Java 方法 `keys`。
    pub fn keys(&self) -> &[String] {
        &self.keys
    }

    /// Java `newInstance`: zero-arg constructor.
    fn new_instance(&self, q_context: &dyn QContext) -> Result<DataValue, QLException> {
        let Some(constructor) = q_context
            .registry()
            .load_constructor_for_args(&self.new_cls, &[])
        else {
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
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        _ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let instance = self.new_instance(q_context)?;
        let init_items = q_context.pop_n(self.keys.len());
        for (i, field_name) in self.keys.iter().enumerate() {
            let init_value = init_items.get_value(i);
            // Java `ReflectLoader.loadField` 返回可写 `FieldValue`。Rust
            // 宿主对象没有运行时反射，先通过显式 `NativeObject::set_field`
            // 复现字段写入；失败后继续走 load_field，以区分字段不存在
            // （Java 忽略）和字段存在但只读/类型不兼容（赋值错误）。
            if let DataValue::Object(object) = &instance {
                if object.borrow_mut().set_field(field_name, &init_value) {
                    continue;
                }
            }
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
