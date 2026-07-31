//! 方法处理器,对应 Java `com.alibaba.qlexpress4.member.MethodHandler`。
//!
//! 适配说明(SPEC §4):Java 版基于反射枚举 `clazz.getMethods()` 查找
//! getter/setter;Rust 在 [`NativeType`] 注册的方法表内按同名规则匹配。
//! Java `Method` 的抽象修饰符由 [`NativeType::abstract_methods`] 显式承载，
//! 因而自定义单抽象方法接口也能参与 Lambda 重载选择。

use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::i_method::IMethod;
use crate::runtime::native_type::{NativeMethod, NativeType};
use crate::runtime::value::DataValue;
use crate::utils::basic_util::BasicUtil;

pub use super::getter_candidate_method::GetterCandidateMethod;

/// 方法处理器。对应 Java: com.alibaba.qlexpress4.member.MethodHandler
/// (职责:getter/setter 查找与带访问控制的方法调用)。
///
/// Java 内部 `GetterCandidateMethod` 只临时保存 `(method, priority)`；
/// Rust 在 [`MethodHandler::get_getter`] 中以 `Option<NativeMethod>` 和固定
/// `getX`/`isX` 检查顺序表达同一选择状态。对应 Java:
/// `com.alibaba.qlexpress4.member.MethodHandler.GetterCandidateMethod`。
pub struct MethodHandler;

impl MethodHandler {
    /// 对应 Java 方法 `getGetter(Class<?>, String)`:
    /// 按 `isX()`(返回 boolean,优先级 2)/ `getX()`(优先级 1)规则
    /// 在注册方法表中查找 getter,找不到返回 `None`(Java 返回 `null`)。
    ///
    /// Java 语义要点:`isPreferredGetter` 以后出现且优先级不低于前者的
    /// 候选覆盖前者;注册表无序,Rust 按「先 getX 后 isX」的顺序应用同一
    /// 覆盖规则,结果一致(isX 最终胜出)。
    pub fn get_getter(native_type: &NativeType, property: &str) -> Option<NativeMethod> {
        let getter = BasicUtil::get_getter(property);
        let is_get = BasicUtil::get_is_getter(property);
        // 优先级 1:getX()。
        let mut candidate = native_type
            .methods
            .get(&getter)
            .map(|method| GetterCandidateMethod::new(Rc::clone(method), 1));
        // 优先级 2:isX()(boolean 返回由注册闭包保证,同 Java 的返回类型检查)。
        if let Some(is_method) = native_type.methods.get(&is_get) {
            let after = GetterCandidateMethod::new(Rc::clone(is_method), 2);
            if Self::is_preferred_getter(candidate.as_ref(), &after) {
                candidate = Some(after);
            }
        }
        candidate.map(|candidate| candidate.get_method())
    }

    /// 对应 Java 方法 `getSetter(Class<?>, String)`:
    /// 查找单参数 `setX(...)` 方法,找不到返回 `None`。
    /// (单参数约束由注册方保证;Java 反射检查参数个数。)
    pub fn get_setter(native_type: &NativeType, property: &str) -> Option<NativeMethod> {
        let setter = BasicUtil::get_setter(property);
        native_type.methods.get(&setter).map(Rc::clone)
    }

    /// 判断新 getter 候选是否应覆盖已有候选。
    ///
    /// 对应 Java：`MethodHandler#isPreferredGetter(GetterCandidateMethod,
    /// GetterCandidateMethod)`。已有候选为空时总是接受；否则新候选优先级
    /// 大于或等于已有候选时接受，因此同优先级下后出现者胜出。
    ///
    /// # 参数
    ///
    /// - `before`：当前候选；`None` 对应 Java `null`。
    /// - `after`：新扫描到的候选。
    pub fn is_preferred_getter(
        before: Option<&GetterCandidateMethod>,
        after: &GetterCandidateMethod,
    ) -> bool {
        before.is_none_or(|before| after.get_priority() >= before.get_priority())
    }

    /// 判断方法集合是否恰好包含一个抽象方法。
    ///
    /// 对应 Java：`MethodHandler#hasOnlyOneAbstractMethod(Method[])`。Rust
    /// 显式注册层只需传入各方法的 `is_abstract` 标记；发现第二个抽象方法
    /// 时立即返回 `false`。
    ///
    /// # 参数
    ///
    /// - `abstract_flags`：与 Java `Method[]` 顺序一致的抽象修饰符标记。
    pub fn has_only_one_abstract_method(abstract_flags: &[bool]) -> bool {
        let mut count = 0usize;
        for is_abstract in abstract_flags {
            if *is_abstract {
                count += 1;
                if count > 1 {
                    return false;
                }
            }
        }
        count == 1
    }
}

pub use super::access::Access;

impl Access {
    /// 对应 Java 方法 `Access.accessMethodValue(IMethod, Object, Object[])`:
    /// 方法不可访问时先 `setAccessible(true)` 再调用。
    pub fn access_method_value(
        method: &Rc<dyn IMethod>,
        bean: &DataValue,
        args: &[DataValue],
    ) -> Result<DataValue, QLException> {
        if !method.is_access() {
            method.set_accessible(true);
        }
        method.invoke(bean, args)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn method(value: i32) -> NativeMethod {
        Rc::new(move |_bean, _args| Ok(DataValue::Int(value)))
    }

    /// SOURCE_PARITY: MethodHandler#isPreferredGetter。
    #[test]
    fn preferred_getter_matches_priority_and_last_wins_rules() {
        let low = GetterCandidateMethod::new(method(1), 1);
        let same = GetterCandidateMethod::new(method(2), 1);
        let high = GetterCandidateMethod::new(method(3), 2);
        assert!(MethodHandler::is_preferred_getter(None, &low));
        assert!(MethodHandler::is_preferred_getter(Some(&low), &same));
        assert!(MethodHandler::is_preferred_getter(Some(&low), &high));
        assert!(!MethodHandler::is_preferred_getter(Some(&high), &low));
    }

    /// SOURCE_PARITY: MethodHandler#hasOnlyOneAbstractMethod。
    #[test]
    fn detects_exactly_one_abstract_method() {
        assert!(!MethodHandler::has_only_one_abstract_method(&[]));
        assert!(!MethodHandler::has_only_one_abstract_method(&[
            false, false
        ]));
        assert!(MethodHandler::has_only_one_abstract_method(&[
            false, true, false
        ]));
        assert!(!MethodHandler::has_only_one_abstract_method(&[
            true, false, true
        ]));
    }

    /// SOURCE_PARITY: GetterCandidateMethod getter/setter。
    #[test]
    fn getter_candidate_accessors_mutate_both_fields() {
        let mut candidate = GetterCandidateMethod::new(method(1), 1);
        candidate.set_method(method(2));
        candidate.set_priority(9);
        assert_eq!(candidate.get_priority(), 9);
        assert_eq!(
            candidate.get_method()(&DataValue::Null, &[]).expect("invoke candidate"),
            DataValue::Int(2)
        );
    }
}
