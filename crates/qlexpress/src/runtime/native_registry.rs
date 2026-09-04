//! 显式类型注册表,替代 Java 反射(SPEC §4)。
//! 对应 Java `com.alibaba.qlexpress4.runtime.ReflectLoader` 的
//! `loadConstructor`/`loadField`/`loadMethod` 职责,以及
//! `ClassSupplier`/`DefaultClassSupplier` 的类型供给职责(Rust 新增物,
//! Java 无同名类;内建方法子集对齐 Java 版脚本中 String/List/Map/数值
//! 的常用方法)。

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::exception::error_codes;
use crate::exception::ql_exception::QLExceptionKind;
use crate::exception::QLException;
use crate::member::field_handler::Preferred as PreferredFieldHandler;
use crate::number::big_decimal_math::BigDecimalMath;
use crate::proxy::q_lambda_invocation_handler::QLambdaInvocationHandler;
use crate::runtime::class_ref::ClassRef;
use crate::runtime::data::convert::number_compare;
use crate::runtime::data::convert::parameters_type_convertor::ParametersTypeConvertor;
use crate::runtime::data::index_map::IndexMap;
use crate::runtime::data::java_array_list::JavaArrayList;
use crate::runtime::data::java_string::JavaString;
use crate::runtime::data::{FieldValue, MapItemValue};
use crate::runtime::function::ExtensionFunction;
use crate::runtime::java_collector::JavaCollector;
use crate::runtime::java_map_entry::JavaMapEntry;
use crate::runtime::java_stream::JavaStream;
use crate::runtime::member_resolver::MemberResolver;
use crate::runtime::meta_class::{as_meta_class, MetaClass};
use crate::runtime::native_type::{
    NativeConstructor, NativeConstructorCandidate, NativeMethod, NativeMethodCandidate, NativeType,
};
use crate::runtime::opaque_native_object::OpaqueNativeObject;
use crate::runtime::qvm_runtime::current_time_millis;
use crate::runtime::value::{DataValue, QValue};
use crate::security::ql_security_strategy::{NativeMember, QLSecurityStrategy};
use crate::utils::basic_util;
use crate::utils::cache_util::CacheUtil;

/// 显式类型注册表。对应 Java: `ReflectLoader`(按类型加载构造器/字段/方法)
/// + `DefaultClassSupplier`(类型白名单式供给)。
///
/// Java 语义要点:Java `loadField`/`loadMethod` 通过反射现场解析成员,
/// 并对数组/List 的 `length`、Map 的 key、类的静态成员有特殊分支;
/// Rust 版逐一复现这些特殊分支,普通成员走注册表查询。
///
/// 安全策略接线点(Java `ReflectLoader.check(Member)`):
/// Java 在反射取得成员后过 `securityStrategy.check(member)`,不通过则按
/// 「成员不存在」(`null`)处理;Rust 在**注册类型成员**的解析点做同样
/// 判定(见 [`NativeRegistry::resolve_method`] / [`NativeRegistry::load_field`])。
/// 偏差:内建类型(String/List/Map/数值)的方法子集是 Rust 语言内核的
/// 一部分,不过策略(Java 中它们也走反射,`isolation` 默认下同样被拦);
/// 注册表自身默认策略为 `open`(Java `InitOptions` 默认 `isolation`,
/// 由 `Express4Runner` 构造时显式接线,见 `set_security_strategy`)。
/// 对应 Java: 无（Rust 原生适配）。
pub struct NativeRegistry {
    /// 类型名 -> 注册类型(Java 侧为 `ClassLoader` 可加载的所有类)。
    ///
    /// Java `ReflectLoader` 在已创建运行时或 Lambda 仍持有加载器时也允许
    /// 继续注册扩展能力。Rust 运行时通过 `Rc` 共享本注册表，因此类型表
    /// 使用内部可变性，并以 `Rc<NativeType>` 快照供一次解析过程稳定读取。
    types: RefCell<HashMap<String, Rc<NativeType>>>,
    /// 扩展函数表。Java `ReflectLoader.loadMethod` 在隔离策略判断之前解析
    /// `ExtensionFunction`，因此它必须与受安全策略约束的反射方法分开。
    extension_methods: RefCell<HashMap<(String, String), NativeMethod>>,
    /// 带 Java 形参签名的扩展函数候选，按注册顺序保存。对应 Java
    /// `CopyOnWriteArrayList<ExtensionFunction>`，允许同一声明类型和名称
    /// 注册多个重载并在调用现场交给 `MemberResolver` 选择。
    extension_method_candidates: RefCell<Vec<(ClassRef, String, NativeMethodCandidate)>>,
    /// 成员访问安全策略(Java `ReflectLoader.securityStrategy`)。
    /// `RefCell`:注册表经 `Rc` 共享给 QVM,策略需在 runner 层可改。
    security_strategy: RefCell<QLSecurityStrategy>,
    /// 函数式接口判定缓存。对应 Java `CacheUtil` 的 Class 级缓存；
    /// 每个注册表独立，避免租户/宿主模型之间的同名类型污染。
    function_interface_cache: CacheUtil,
}

impl NativeRegistry {
    /// 空注册表。对应 Java `new ReflectLoader()`(无任何已知类型)。
    ///
    /// # Returns
    ///
    /// 返回默认开放成员策略且不包含任何类型或扩展方法的注册表。
    pub fn new() -> Self {
        NativeRegistry {
            types: RefCell::new(HashMap::new()),
            extension_methods: RefCell::new(HashMap::new()),
            extension_method_candidates: RefCell::new(Vec::new()),
            // 注册表裸用时默认放行;Express4Runner 构造时按
            // `InitOptions.securityStrategy` 覆盖(Java 默认 `isolation`)。
            security_strategy: RefCell::new(QLSecurityStrategy::open()),
            function_interface_cache: CacheUtil::new(),
        }
    }

    /// 设置成员访问安全策略。对应 Java `ReflectLoader` 持有的
    /// `securityStrategy`(由 `InitOptions` 注入)。
    ///
    /// # Arguments
    ///
    /// * `security_strategy` - 后续构造器、字段和方法解析共同使用的策略。
    pub fn set_security_strategy(&self, security_strategy: QLSecurityStrategy) {
        *self.security_strategy.borrow_mut() = security_strategy;
    }

    /// 当前安全策略。对应 Java `InitOptions.getSecurityStrategy()` 经
    /// `ReflectLoader` 读取的值。
    ///
    /// # Returns
    ///
    /// 返回当前策略快照。
    pub fn security_strategy(&self) -> QLSecurityStrategy {
        self.security_strategy.borrow().clone()
    }

    /// Java `ReflectLoader.check(Member)`:`check` 不通过即视为成员不存在。
    fn check_member(&self, type_name: &str, member_name: &str) -> bool {
        self.security_strategy
            .borrow()
            .is_allowed(&NativeMember::new(type_name, member_name))
    }

    /// 判断原生成员是否被当前安全策略允许。
    ///
    /// 对应 Java 私有方法 `ReflectLoader#securityFilter(Member)`；供
    /// `NativeObject` 动态分派在调用前执行与反射成员一致的检查。
    ///
    /// # Arguments
    ///
    /// * `type_name` - 成员声明类型的 Java 规范名。
    /// * `member_name` - 构造器、字段或方法名。
    ///
    /// # Returns
    ///
    /// 当前安全策略允许访问该成员时返回 `true`。
    pub fn is_member_allowed(&self, type_name: &str, member_name: &str) -> bool {
        self.check_member(type_name, member_name)
    }

    /// 判断类型层次中是否声明了指定名称的方法候选。
    ///
    /// 候选存在但实参不匹配时，Java 反射会直接报告“方法不存在”，不能再
    /// 回退到 Rust [`NativeObject`](crate::runtime::native_object::NativeObject)
    /// 的动态分派入口。
    /// 对应 Java：`ReflectLoader#loadMethod` 在已发现同名方法后的重载解析语义。
    pub(crate) fn has_registered_method_candidates(
        &self,
        type_name: &str,
        method_name: &str,
    ) -> bool {
        self.has_registered_method_candidates_inner(type_name, method_name, &mut Vec::new())
    }

    fn has_registered_method_candidates_inner(
        &self,
        type_name: &str,
        method_name: &str,
        visited: &mut Vec<String>,
    ) -> bool {
        if visited.iter().any(|name| name == type_name) {
            return false;
        }
        visited.push(type_name.to_string());
        let Some(native_type) = self.get_type(type_name) else {
            return false;
        };
        let registered_name =
            Self::resolve_registered_method_name(&native_type, method_name, false);
        if native_type
            .method_candidates
            .get(registered_name)
            .is_some_and(|candidates| !candidates.is_empty())
        {
            return true;
        }
        native_type.supertypes.iter().any(|supertype| {
            self.has_registered_method_candidates_inner(supertype, method_name, visited)
        })
    }
}

impl Default for NativeRegistry {
    /// 与 [`NativeRegistry::new`] 一致(默认放行策略)。
    fn default() -> Self {
        NativeRegistry::new()
    }
}

include!("native_registry/registration_and_fields.rs");
include!("native_registry/method_resolution.rs");
include!("native_registry/builtin_types.rs");

include!("native_registry/candidate_helpers.rs");
include!("native_registry/string_methods.rs");
include!("native_registry/collection_methods.rs");
include!("native_registry/scalar_methods.rs");
