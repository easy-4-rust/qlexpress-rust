//! 成员(方法)重载解析,对应 Java `com.alibaba.qlexpress4.runtime.MemberResolver`。
//!
//! 适配说明(SPEC §4):Java 版输入是 `Class<?>[]`(反射类型);Rust 用
//! [`ClassRef`] 表达同一语义。继承体系解析(`isAssignableFrom`、接口默认
//! 方法、构造器反射枚举)无运行时类信息可用,按注册表策略裁剪:
//! 仅保留「精确/拆箱/数值提升/数值窄化/Lambda/扩展(Object)」优先级体系,
//! 与脚本实际可构造的类型组合一致。

use std::rc::Rc;

use crate::runtime::class_ref::ClassRef;
use crate::runtime::i_method::IMethod;
use crate::utils::basic_util::NumKind;

/// 匹配优先级。对应 Java: `MemberResolver.MatchPriority`(枚举,
/// `priority` 字段越大越优先)。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchPriority {
    /// 不匹配(Java `MISMATCH(-1)`)。
    Mismatch,
    /// 扩展匹配,如实参可赋给 `Object`(Java `EXTEND(0)`)。
    Extend,
    /// 数值窄化,如 BigDecimal -> int(Java `NUMBER_DEMOTION(9)`)。
    NumberDemotion,
    /// 数值提升,如 int -> long(Java `NUMBER_PROMOTION(8)`,实际优先级
    /// 按 `8 + argLevel - paramLevel` 计算)。
    NumberPromotion,
    /// 拆箱,如 Integer -> int(Java `UNBOX(9)`;Rust 中包装/原语同型,
    /// 该档与「精确」重合,保留以对齐 Java 优先级数值)。
    Unbox,
    /// Lambda 适配函数式接口(Java `LAMBDA(10)`)。
    Lambda,
    /// 类型完全相同(Java `EQUAL(11)`)。
    Equal,
}

impl MatchPriority {
    /// 对应 Java 字段 `priority`。
    pub fn priority(self) -> i32 {
        match self {
            MatchPriority::Mismatch => -1,
            MatchPriority::Extend => 0,
            MatchPriority::NumberDemotion => 9,
            MatchPriority::NumberPromotion => 8,
            MatchPriority::Unbox => 9,
            MatchPriority::Lambda => 10,
            MatchPriority::Equal => 11,
        }
    }
}

/// 方法重载解析器。对应 Java: com.alibaba.qlexpress4.runtime.MemberResolver
/// (职责:在候选方法签名中按实参类型挑选最优匹配)。
pub struct MemberResolver;

impl MemberResolver {
    /// 对应 Java 方法 `resolveMethod(List<? extends IMethod>, Class<?>[])`:
    /// 先做精确匹配,失败后对可变参数候选做 varargs 适配再匹配。
    pub fn resolve_method(
        methods: &[Rc<dyn IMethod>],
        arg_types: &[ClassRef],
    ) -> Option<Rc<dyn IMethod>> {
        // 简单匹配(Java: simple match)。
        let candidates: Vec<Vec<ClassRef>> = methods
            .iter()
            .map(|method| method.parameter_types())
            .collect();
        if let Some(best_index) = Self::resolve_best_match(&candidates, arg_types) {
            return Some(Rc::clone(&methods[best_index]));
        }

        // 可变参数匹配(Java: var args match)。
        let mut var_args_candidates: Vec<Vec<ClassRef>> = Vec::new();
        let mut var_args_method_i: Vec<usize> = Vec::new();
        for (i, method) in methods.iter().enumerate() {
            if !method.is_var_args() {
                continue;
            }
            var_args_candidates.push(Self::adapt_2_var_arg_types(
                &method.parameter_types(),
                arg_types.len(),
            ));
            var_args_method_i.push(i);
        }
        let var_arg_best_index = Self::resolve_best_match(&var_args_candidates, arg_types)?;
        Some(Rc::clone(&methods[var_args_method_i[var_arg_best_index]]))
    }

    /// 对应 Java 方法 `resolveBestMatch(Class<?>[][], Class<?>[])`:
    /// 选优先级最高(数值最大)的候选下标。
    pub fn resolve_best_match(
        candidates: &[Vec<ClassRef>],
        arg_types: &[ClassRef],
    ) -> Option<usize> {
        Self::resolve_best_match_with(candidates, arg_types, default_assignable)
    }

    /// 使用宿主注册表提供的继承关系选择最佳候选。
    pub fn resolve_best_match_with(
        candidates: &[Vec<ClassRef>],
        arg_types: &[ClassRef],
        is_assignable: impl Fn(&ClassRef, &ClassRef) -> bool,
    ) -> Option<usize> {
        let mut best_match_index = None;
        let mut best_priority = MatchPriority::Mismatch.priority();
        for (i, candidate) in candidates.iter().enumerate() {
            let priority = Self::resolve_priority_with(candidate, arg_types, &is_assignable);
            if priority > best_priority {
                best_priority = priority;
                best_match_index = Some(i);
            }
        }
        best_match_index
    }

    /// 对应 Java 方法 `resolvePriority(Class<?>[], Class<?>[])`:
    /// 方法优先级 = 各参数优先级的最小值;长度不等或任一参数不匹配即
    /// `MISMATCH`。
    pub fn resolve_priority(param_types: &[ClassRef], arg_types: &[ClassRef]) -> i32 {
        Self::resolve_priority_with(param_types, arg_types, &default_assignable)
    }

    /// 使用显式类型继承判断计算方法优先级。
    pub fn resolve_priority_with(
        param_types: &[ClassRef],
        arg_types: &[ClassRef],
        is_assignable: &dyn Fn(&ClassRef, &ClassRef) -> bool,
    ) -> i32 {
        if param_types.len() != arg_types.len() {
            return MatchPriority::Mismatch.priority();
        }
        let mut method_priority = MatchPriority::Equal.priority();
        for (param_type, arg_type) in param_types.iter().zip(arg_types.iter()) {
            let param_priority =
                Self::resolve_arg_priority_with(param_type, arg_type, is_assignable);
            if param_priority == MatchPriority::Mismatch.priority() {
                return param_priority;
            }
            if param_priority < method_priority {
                method_priority = param_priority;
            }
        }
        method_priority
    }

    /// 对应 Java 私有方法 `resolveArgPriority(Class<?>, Class<?>)`。
    fn resolve_arg_priority_with(
        param_type: &ClassRef,
        arg_type: &ClassRef,
        is_assignable: &dyn Fn(&ClassRef, &ClassRef) -> bool,
    ) -> i32 {
        if param_type == arg_type {
            return MatchPriority::Equal.priority();
        }
        // Java: CacheUtil.isFunctionInterface(paramType)
        //       && QLambda.class.isAssignableFrom(argType)。
        if is_function_interface(param_type) && is_qlambda(arg_type) {
            return MatchPriority::Lambda.priority();
        }

        // Java 的 UNBOX 分支:包装类与原语类互转。Rust 中
        // 常规解析会把包装类归一到 `Primitive`；显式注册的方法签名仍可
        // 以 `Named("java.lang.*")` 保留包装类型，此时必须维持 Java 的
        // UNBOX 优先级，避免与完全相同签名混淆。
        if is_boxing_pair(param_type, arg_type) {
            return MatchPriority::Unbox.priority();
        }

        // 数值提升/窄化(Java: BasicUtil.numberPromoteLevel 双侧可比)。
        if let (Some(param_level), Some(arg_level)) = (
            number_promote_level(param_type),
            number_promote_level(arg_type),
        ) {
            return if param_level >= arg_level {
                MatchPriority::NumberPromotion.priority() + arg_level as i32 - param_level as i32
            } else {
                MatchPriority::NumberDemotion.priority()
            };
        }

        // Java: 原语实参装箱为 Object(paramType == Object.class)。
        if matches!(arg_type, ClassRef::Primitive(_)) && param_type.is_java_object() {
            return MatchPriority::Extend.priority();
        }

        // Java: argType == Nothing.class(null 实参)或
        // paramType.isAssignableFrom(argType)。Rust 无继承信息,
        // 仅复现 Nothing 与 Object 两种可赋值情形。
        if is_nothing(arg_type)
            || param_type.is_java_object()
            || is_assignable(param_type, arg_type)
        {
            return MatchPriority::Extend.priority();
        }
        MatchPriority::Mismatch.priority()
    }

    /// 在带 `varargs` 标记的签名列表中选择最佳候选下标。
    /// 对应 Java: com.alibaba.qlexpress4.runtime.MemberResolver#resolveCandidateIndex。
    pub fn resolve_candidate_index(
        candidates: &[(Vec<ClassRef>, bool)],
        arg_types: &[ClassRef],
        is_assignable: impl Fn(&ClassRef, &ClassRef) -> bool,
    ) -> Option<usize> {
        let fixed_indices: Vec<usize> = candidates
            .iter()
            .enumerate()
            .filter_map(|(index, (_, var_args))| (!*var_args).then_some(index))
            .collect();
        let fixed_signatures: Vec<Vec<ClassRef>> = fixed_indices
            .iter()
            .map(|index| candidates[*index].0.clone())
            .collect();
        if let Some(index) =
            Self::resolve_best_match_with(&fixed_signatures, arg_types, &is_assignable)
        {
            return Some(fixed_indices[index]);
        }

        let var_indices: Vec<usize> = candidates
            .iter()
            .enumerate()
            .filter_map(|(index, (_, var_args))| (*var_args).then_some(index))
            .collect();
        let var_signatures: Vec<Vec<ClassRef>> = var_indices
            .iter()
            .map(|index| Self::adapt_2_var_arg_types(&candidates[*index].0, arg_types.len()))
            .collect();
        Self::resolve_best_match_with(&var_signatures, arg_types, is_assignable)
            .map(|index| var_indices[index])
    }

    /// 对应 Java 私有方法 `adapt2VarArgTypes`:把可变参数签名按实参个数
    /// 展开(末位参数以其组件类型重复)。
    fn adapt_2_var_arg_types(parameter_types: &[ClassRef], arg_length: usize) -> Vec<ClassRef> {
        let var_item_type = parameter_types
            .last()
            .cloned()
            .unwrap_or_else(|| ClassRef::Named("java.lang.Object".to_string()));
        let mut var_param_types: Vec<ClassRef> =
            parameter_types[..parameter_types.len().saturating_sub(1)].to_vec();
        while var_param_types.len() < arg_length {
            var_param_types.push(var_item_type.clone());
        }
        var_param_types
    }
}

fn default_assignable(param_type: &ClassRef, arg_type: &ClassRef) -> bool {
    param_type == arg_type || param_type.is_java_object()
}

fn is_boxing_pair(left: &ClassRef, right: &ClassRef) -> bool {
    fn primitive_wrapper(class_ref: &ClassRef) -> Option<&'static str> {
        match class_ref {
            ClassRef::Primitive(target) => Some(target.java_name()),
            ClassRef::Named(_) => None,
        }
    }

    match (left, right) {
        (ClassRef::Primitive(_), ClassRef::Named(right_name)) => {
            primitive_wrapper(left) == Some(right_name.as_str())
        }
        (ClassRef::Named(left_name), ClassRef::Primitive(_)) => {
            primitive_wrapper(right) == Some(left_name.as_str())
        }
        _ => false,
    }
}

/// 对应 Java `CacheUtil.isFunctionInterface`:是否为函数式接口类型。
/// Rust 按 Java 函数式接口全限定名前缀识别(`java.util.function.*`、
/// `java.lang.Runnable`)。
fn is_function_interface(class_ref: &ClassRef) -> bool {
    match class_ref {
        ClassRef::Named(name) => {
            name.starts_with("java.util.function.") || name == "java.lang.Runnable"
        }
        ClassRef::Primitive(_) => false,
    }
}

/// 对应 Java `QLambda.class.isAssignableFrom(argType)`:实参是否脚本 Lambda。
fn is_qlambda(class_ref: &ClassRef) -> bool {
    matches!(class_ref, ClassRef::Named(name) if name == "com.alibaba.qlexpress4.runtime.QLambda")
}

/// 对应 Java `argType == Nothing.class`:实参是否 `null` 字面量类型。
fn is_nothing(class_ref: &ClassRef) -> bool {
    matches!(class_ref, ClassRef::Named(name) if name == "com.alibaba.qlexpress4.runtime.Nothing")
}

/// 对应 Java `BasicUtil.numberPromoteLevel(Class)`。
fn number_promote_level(class_ref: &ClassRef) -> Option<u8> {
    let kind = match class_ref.java_name() {
        "java.lang.Byte" => NumKind::Byte,
        "java.lang.Short" => NumKind::Short,
        "java.lang.Integer" => NumKind::Int,
        "java.lang.Long" => NumKind::Long,
        "java.math.BigInteger" => NumKind::BigInteger,
        "java.lang.Float" => NumKind::Float,
        "java.lang.Double" => NumKind::Double,
        "java.math.BigDecimal" => NumKind::BigDecimal,
        _ => return None,
    };
    Some(kind.promote_level())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn named(name: &str) -> ClassRef {
        ClassRef::Named(name.to_string())
    }

    #[test]
    fn priority_numbers_align_with_java() {
        assert_eq!(MatchPriority::Mismatch.priority(), -1);
        assert_eq!(MatchPriority::Extend.priority(), 0);
        assert_eq!(MatchPriority::NumberPromotion.priority(), 8);
        assert_eq!(MatchPriority::Unbox.priority(), 9);
        assert_eq!(MatchPriority::NumberDemotion.priority(), 9);
        assert_eq!(MatchPriority::Lambda.priority(), 10);
        assert_eq!(MatchPriority::Equal.priority(), 11);
    }

    #[test]
    fn number_promotion_beats_demotion() {
        // int 实参 -> long 形参:提升(8 + 2 - 3 = 7)。
        let promotion = MemberResolver::resolve_priority(
            &[ClassRef::from_name("long")],
            &[ClassRef::from_name("int")],
        );
        assert_eq!(promotion, 7);
        // long 实参 -> int 形参:窄化(9)。
        let demotion = MemberResolver::resolve_priority(
            &[ClassRef::from_name("int")],
            &[ClassRef::from_name("long")],
        );
        assert_eq!(demotion, 9);
    }

    #[test]
    fn lambda_arg_matches_function_interface() {
        let priority = MemberResolver::resolve_priority(
            &[named("java.util.function.Function")],
            &[named("com.alibaba.qlexpress4.runtime.QLambda")],
        );
        assert_eq!(priority, MatchPriority::Lambda.priority());
    }

    #[test]
    fn null_arg_matches_any_reference_via_extend() {
        let priority = MemberResolver::resolve_priority(
            &[named("java.lang.String")],
            &[named("com.alibaba.qlexpress4.runtime.Nothing")],
        );
        assert_eq!(priority, MatchPriority::Extend.priority());
    }

    /// 逐项对应 Java `MemberResolverTest#resolvePriorityTest`。
    #[test]
    fn java_resolve_priority_contract() {
        let boolean_primitive = ClassRef::from_name("boolean");
        let boolean_wrapper = ClassRef::Named("java.lang.Boolean".to_string());
        assert_eq!(
            MemberResolver::resolve_priority(&[boolean_primitive], &[boolean_wrapper]),
            MatchPriority::Unbox.priority()
        );
        assert_eq!(
            MemberResolver::resolve_priority(&[], &[]),
            MatchPriority::Equal.priority()
        );
    }

    /// 逐项对应 Java `MemberResolverTest#resolveConstructorTest` 的候选
    /// 签名选择；Rust 构造器和方法共用同一签名解析器。
    #[test]
    fn java_constructor_candidate_priority_contract() {
        let candidates = [
            vec![named("java.lang.Number")],
            vec![ClassRef::from_name("long")],
            vec![named("java.lang.Long"), named("java.lang.Runnable")],
            vec![ClassRef::from_name("long"), named("java.lang.Runnable")],
        ];
        assert_eq!(
            MemberResolver::resolve_best_match(
                &candidates[..2],
                &[ClassRef::from_name("java.lang.Integer")],
            ),
            Some(1)
        );
        assert_eq!(
            MemberResolver::resolve_best_match(
                &candidates[2..],
                &[
                    named("java.lang.Long"),
                    named("com.alibaba.qlexpress4.runtime.QLambda"),
                ],
            ),
            Some(0)
        );
    }
}
