//! for-each 循环指令,对应 Java `com.alibaba.qlexpress4.runtime.instruction.ForEachInstruction`。
//! 职责:遍历集合/数组的循环执行体。
//! 本文件由 `flow.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use crate::exception::error_codes;
use crate::exception::error_reporter::ErrorReporter;
use crate::exception::{QLException, QLExceptionKind};
use crate::ql_options::QLOptions;
use crate::runtime::instruction::QLInstruction;
use crate::runtime::data::{JavaArray, JavaArrayList};
use crate::runtime::member::ClassRef;
use crate::runtime::q_result::QResult;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda_definition::QLambdaDefinition;
use crate::runtime::value::DataValue;
use crate::utils::println_utils::PrintlnUtils;
use std::cell::RefCell;
use std::rc::Rc;

/// 反射数组的可迭代适配器。
///
/// 对应 Java：
/// `ForEachInstruction.ReflectArrayIterable`。Java 持有任意反射数组对象；
/// Rust 数组统一为共享 `Vec<DataValue>`。
#[derive(Clone)]
pub(crate) struct ReflectArrayIterable {
    arr_obj: Rc<RefCell<JavaArray>>,
}

impl ReflectArrayIterable {
    /// 创建数组可迭代适配器。
    ///
    /// 对应 Java 私有构造器 `ReflectArrayIterable(Object)`。
    pub(crate) fn new(arr_obj: Rc<RefCell<JavaArray>>) -> Self {
        Self { arr_obj }
    }

    /// 创建一个游标从零开始的数组迭代器。
    ///
    /// 对应 Java：`ReflectArrayIterable#iterator()`。
    pub fn iterator(&self) -> ReflectArrayIterator {
        ReflectArrayIterator {
            arr_obj: Rc::clone(&self.arr_obj),
            cursor: 0,
        }
    }
}

/// 反射数组迭代器。
///
/// 对应 Java：
/// `ForEachInstruction.ReflectArrayIterable.ReflectArrayIterator`。
pub(crate) struct ReflectArrayIterator {
    arr_obj: Rc<RefCell<JavaArray>>,
    cursor: usize,
}

impl ReflectArrayIterator {
    /// 判断游标是否仍小于数组长度。
    ///
    /// 对应 Java：`ReflectArrayIterator#hasNext()`。
    pub fn has_next(&self) -> bool {
        self.cursor < self.arr_obj.borrow().len()
    }
}

impl Iterator for ReflectArrayIterator {
    type Item = DataValue;

    /// 返回当前元素并将游标加一。
    ///
    /// 对应 Java：`ReflectArrayIterator#next()`。通过 Rust `Iterator`
    /// 契约在越界时返回 `None`，循环调用点与 Java 增强 for 的行为一致。
    fn next(&mut self) -> Option<Self::Item> {
        if !self.has_next() {
            return None;
        }
        let item = self.arr_obj.borrow().get(self.cursor).cloned();
        self.cursor += 1;
        item
    }
}

/// Java `ArrayList.Itr` 的 fail-fast 迭代器适配。
///
/// Java 的 `ForEachInstruction` 对普通 `Iterable` 直接使用其迭代器；列表
/// 不能克隆为快照，否则循环体中的结构修改不会产生
/// `ConcurrentModificationException`。
struct JavaArrayListIterator {
    list: Rc<RefCell<JavaArrayList>>,
    cursor: usize,
    expected_mod_count: u64,
}

impl JavaArrayListIterator {
    /// 从列表当前结构版本创建迭代器。
    fn new(list: Rc<RefCell<JavaArrayList>>) -> Self {
        let expected_mod_count = list.borrow().mod_count();
        Self {
            list,
            cursor: 0,
            expected_mod_count,
        }
    }

    /// 对应 Java `ArrayList.Itr#hasNext()` 的 `cursor != size`。
    fn has_next(&self) -> bool {
        self.cursor != self.list.borrow().len()
    }

    /// 对应 Java `ArrayList.Itr#next()`：先检查 `modCount`，再读取元素。
    fn next_item(&mut self) -> Result<DataValue, QLException> {
        let list = self.list.borrow();
        if list.mod_count() != self.expected_mod_count {
            return Err(QLException::host_error(
                QLExceptionKind::Runtime,
                "java.util.ConcurrentModificationException",
                "java.util.ConcurrentModificationException",
            ));
        }
        let Some(item) = list.get(self.cursor).cloned() else {
            return Err(QLException::host_error(
                QLExceptionKind::Runtime,
                "java.util.NoSuchElementException",
                "java.util.NoSuchElementException",
            ));
        };
        self.cursor += 1;
        Ok(item)
    }
}

/// 数组与列表共用的 Java foreach 迭代协议。
enum ForEachIterator {
    Array(ReflectArrayIterator),
    List(JavaArrayListIterator),
}

impl ForEachIterator {
    fn has_next(&self) -> bool {
        match self {
            Self::Array(iterator) => iterator.has_next(),
            Self::List(iterator) => iterator.has_next(),
        }
    }

    fn next_item(&mut self) -> Result<DataValue, QLException> {
        match self {
            Self::Array(iterator) => iterator.next().ok_or_else(|| {
                QLException::host_error(
                    QLExceptionKind::Runtime,
                    "java.util.NoSuchElementException",
                    "java.util.NoSuchElementException",
                )
            }),
            Self::List(iterator) => iterator.next_item(),
        }
    }
}

/// for-each 循环指令。对应 Java: com.alibaba.qlexpress4.runtime.instruction.ForEachInstruction(职责:遍历集合/数组的循环执行体)
/// Operation: process each element in iterable object on top of stack,
/// Input: 1
/// Output: 0
///
/// Mirrors Java `ForEachInstruction`.
///
/// Java 为反射数组额外创建两层适配器；Rust 对 `DataValue::Array` 保留同样
/// 的 iterable/iterator 两层结构，并按游标逐项读取共享数组：
///
/// - 对应 Java:
///   `com.alibaba.qlexpress4.runtime.instruction.ForEachInstruction.ReflectArrayIterable`
/// - 对应 Java:
///   `com.alibaba.qlexpress4.runtime.instruction.ForEachInstruction.ReflectArrayIterable.ReflectArrayIterator`
pub struct ForEachInstruction {
    error_reporter: Rc<dyn ErrorReporter>,
    body: Rc<dyn QLambdaDefinition>,
    target_error_reporter: Rc<dyn ErrorReporter>,
    it_cls: ClassRef,
}

impl ForEachInstruction {
    /// 构造指令,对应 Java 构造器 `ForEachInstruction`。
    pub fn new(
        error_reporter: Rc<dyn ErrorReporter>,
        body: Rc<dyn QLambdaDefinition>,
        it_cls: ClassRef,
        target_error_reporter: Rc<dyn ErrorReporter>,
    ) -> Self {
        ForEachInstruction {
            error_reporter,
            body,
            target_error_reporter,
            it_cls,
        }
    }

    /// 对应 Java 方法 `body`。
    pub fn body(&self) -> &Rc<dyn QLambdaDefinition> {
        &self.body
    }

    /// 对应 Java 方法 `targetErrorReporter`。
    pub fn target_error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.target_error_reporter
    }

    /// 对应 Java 方法 `itCls`。
    pub fn it_cls(&self) -> &ClassRef {
        &self.it_cls
    }
}

impl QLInstruction for ForEachInstruction {
    /// 向下转型支持(供 api/parsecache Exporter 的 Java `instanceof` 分派)。
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn execute(
        &self,
        q_context: &mut dyn QContext,
        ql_options: &QLOptions,
    ) -> Result<QResult, QLException> {
        let may_be_iterable = q_context.pop().get();
        // Java: array → ReflectArrayIterable; Iterable → as-is; else error.
        let mut items = match &may_be_iterable {
            DataValue::Array(arr) => ForEachIterator::Array(
                ReflectArrayIterable::new(Rc::clone(arr)).iterator(),
            ),
            DataValue::List(list) => {
                ForEachIterator::List(JavaArrayListIterator::new(Rc::clone(list)))
            }
            _ => {
                return Err(self.target_error_reporter.report(
                    error_codes::FOR_EACH_ITERABLE_REQUIRED,
                    error_codes::error_msg(error_codes::FOR_EACH_ITERABLE_REQUIRED),
                ))
            }
        };
        let body_lambda = Rc::clone(&self.body).to_lambda(q_context, ql_options, true);
        // forEachBody:
        while items.has_next() {
            // Java 增强 for 在进入循环体的 try/catch 之前调用
            // Iterator.next()，因此 fail-fast 异常不应被循环体异常映射吞掉。
            let item = items.next_item()?;
            match body_lambda.call(std::slice::from_ref(&item)) {
                Ok(body_result) => match body_result {
                    QResult::Return(_) => return Ok(body_result),
                    QResult::Break => break,
                    _ => {}
                },
                Err(err) => {
                    // Java: UserDefineException (lambda argument conversion)
                    // → FOR_EACH_TYPE_MISMATCH; QLRuntimeException → rethrow;
                    // else FOR_EACH_UNKNOWN_ERROR.
                    if err.error_code() == error_codes::INVALID_ARGUMENT {
                        return Err(self.error_reporter.report_format(
                            error_codes::FOR_EACH_TYPE_MISMATCH,
                            error_codes::error_msg(error_codes::FOR_EACH_TYPE_MISMATCH),
                            &[
                                self.it_cls.java_name().to_string(),
                                if item.is_null() {
                                    "null".to_string()
                                } else {
                                    item.runtime_type_name()
                                },
                            ],
                        ));
                    }
                    return Err(err);
                }
            }
        }
        Ok(QResult::NEXT_INSTRUCTION)
    }

    fn stack_input(&self) -> i32 {
        1
    }

    fn stack_output(&self) -> i32 {
        0
    }

    fn compiled_instruction_count(&self) -> usize {
        1usize.saturating_add(self.body.compiled_instruction_count())
    }

    fn println(&self, index: usize, depth: usize, debug: &mut dyn FnMut(String)) {
        PrintlnUtils::println_by_cur_depth(depth as i32, &format!("{index}: ForEach"), debug);
        self.body.println(depth + 1, debug);
    }

    fn error_reporter(&self) -> &Rc<dyn ErrorReporter> {
        &self.error_reporter
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// SOURCE_PARITY: ReflectArrayIterable#iterator 与
    /// ReflectArrayIterator#hasNext/next。
    #[test]
    fn reflect_array_iterator_advances_in_array_order() {
        let array = Rc::new(RefCell::new(JavaArray::object(vec![
            DataValue::Int(1),
            DataValue::Int(2),
        ])));
        let iterable = ReflectArrayIterable::new(array);
        let mut iterator = iterable.iterator();
        assert!(iterator.has_next());
        assert_eq!(iterator.next(), Some(DataValue::Int(1)));
        assert!(iterator.has_next());
        assert_eq!(iterator.next(), Some(DataValue::Int(2)));
        assert!(!iterator.has_next());
        assert_eq!(iterator.next(), None);
    }

    /// SOURCE_PARITY: Java `ArrayList.Itr#next` 在列表结构修改后抛出
    /// `ConcurrentModificationException`。
    #[test]
    fn java_list_iterator_rejects_structural_modification() {
        let list = Rc::new(RefCell::new(JavaArrayList::new(vec![
            DataValue::Int(1),
            DataValue::Int(2),
        ])));
        let mut iterator = JavaArrayListIterator::new(Rc::clone(&list));
        assert_eq!(iterator.next_item().unwrap(), DataValue::Int(1));
        list.borrow_mut().push(DataValue::Int(3));

        let error = iterator.next_item().unwrap_err();
        assert_eq!(
            error.error_code(),
            "java.util.ConcurrentModificationException"
        );
    }

    /// SOURCE_PARITY: Java `ArrayList#set` 不改变 `modCount`，已有迭代器
    /// 继续运行并读取替换后的元素。
    #[test]
    fn java_list_iterator_allows_non_structural_set() {
        let list = Rc::new(RefCell::new(JavaArrayList::new(vec![
            DataValue::Int(1),
            DataValue::Int(2),
        ])));
        let mut iterator = JavaArrayListIterator::new(Rc::clone(&list));
        assert_eq!(iterator.next_item().unwrap(), DataValue::Int(1));
        list.borrow_mut().set(1, DataValue::Int(20));

        assert_eq!(iterator.next_item().unwrap(), DataValue::Int(20));
        assert!(!iterator.has_next());
    }
}
