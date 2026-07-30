//! 运行时服务契约,对应 Java `com.alibaba.qlexpress4.runtime.QRuntime`。
//! 职责:一次脚本执行中所有上下文共享的运行时服务(起始时间戳、附件、
//! 类加载/本地注册表、trace 集合)。
//! 本文件由 `qvm_runtime.rs` 拆分而来(SPEC §5.5 一类一文件),仅移动代码与补充中文注释,行为完全一致。

use std::cell::Ref;
use std::rc::Rc;

use crate::ql_options::Attachments;
use crate::runtime::member::NativeRegistry;
use crate::runtime::trace::QTraces;

/// 运行时服务契约。对应 Java: com.alibaba.qlexpress4.runtime.QRuntime
///
/// Runtime services shared by every context of one script execution,
/// mirroring Java `QRuntime`.
pub trait QRuntime {
    /// 脚本起始时间戳(毫秒)。对应 Java 方法 `scriptStartTimeStamp()`。
    /// Java `scriptStartTimeStamp()`: script start time (millis).
    fn script_start_time_stamp(&self) -> i64;

    /// 执行附件。对应 Java 方法 `attachment()`。
    /// Java `attachment()`.
    fn attachment(&self) -> Ref<'_, Attachments>;

    /// 本地注册表。对应 Java 方法 `getReflectLoader()`——由显式的本地注册表
    /// 替代(SPEC §4)。
    /// Java `getReflectLoader()` — replaced by the explicit native registry
    /// (SPEC §4).
    fn registry(&self) -> &Rc<NativeRegistry>;

    /// trace 集合。对应 Java 方法 `getTraces()`。
    /// Java `getTraces()`.
    fn traces(&self) -> &QTraces;
}
