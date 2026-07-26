//! 方法处理器,对应 Java `com.alibaba.qlexpress4.member.MethodHandler`。
//!
//! 适配说明(SPEC §4):Java 版基于反射枚举 `clazz.getMethods()` 查找
//! getter/setter;Rust 在 [`NativeType`] 注册的方法表内按同名规则匹配。
//! `hasOnlyOneAbstractMethod` 依赖 JVM 修饰符,无 Rust 对应物,未迁移
//! (函数式接口判定改由 `MemberResolver` 按接口名识别,见偏差说明)。

use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::i_method::IMethod;
use crate::runtime::native_type::{NativeMethod, NativeType};
use crate::runtime::value::DataValue;
use crate::utils::basic_util::BasicUtil;

/// 方法处理器。对应 Java: com.alibaba.qlexpress4.member.MethodHandler
/// (职责:getter/setter 查找与带访问控制的方法调用)。
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
        let mut candidate = native_type.methods.get(&getter).map(Rc::clone);
        // 优先级 2:isX()(boolean 返回由注册闭包保证,同 Java 的返回类型检查)。
        if let Some(is_method) = native_type.methods.get(&is_get) {
            // Java isPreferredGetter: after.priority >= before.priority 时替换。
            candidate = Some(Rc::clone(is_method));
        }
        candidate
    }

    /// 对应 Java 方法 `getSetter(Class<?>, String)`:
    /// 查找单参数 `setX(...)` 方法,找不到返回 `None`。
    /// (单参数约束由注册方保证;Java 反射检查参数个数。)
    pub fn get_setter(native_type: &NativeType, property: &str) -> Option<NativeMethod> {
        let setter = BasicUtil::get_setter(property);
        native_type.methods.get(&setter).map(Rc::clone)
    }
}

/// 带访问控制的方法调用。对应 Java: `MethodHandler.Access`(静态嵌套类)。
pub struct Access;

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
