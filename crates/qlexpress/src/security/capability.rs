//! 脚本可调用宿主能力的统一标识。

/// 宿主授予脚本的能力。
///
/// 函数、编译期函数、扩展函数、操作符、宏和原生成员统一进入同一策略，
/// 避免只审计 `QLSecurityStrategy` 而遗漏其它扩展入口。
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Capability {
    /// 运行期宿主函数名。
    Function(String),
    /// 编译期函数名。
    CompileTimeFunction(String),
    /// 自定义操作符文本。
    Operator(String),
    /// 宏名。
    Macro(String),
    /// 扩展函数的“声明类型 + 方法名”。
    ExtensionMethod {
        /// 注册时声明的类型名。
        type_name: String,
        /// 方法名。
        method_name: String,
    },
    /// 原生构造器、字段或方法。
    NativeMember {
        /// 注册类型名。
        type_name: String,
        /// 成员名；构造器使用 `<init>`。
        member_name: String,
    },
}
