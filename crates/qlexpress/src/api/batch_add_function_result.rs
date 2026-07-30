//! 批量注册函数结果,对应 Java `com.alibaba.qlexpress4.api.BatchAddFunctionResult`。
//! 职责:记录批量 `addFunction` 中每个条目的注册成败(部分失败语义)。

/// 批量注册函数结果。对应 Java: com.alibaba.qlexpress4.api.BatchAddFunctionResult
///
/// Java 语义:批量注册时单个条目失败不会中断整体。脚本定义批量入口记录
/// 函数名；`addObjFunction/addStaticFunction` 按 Java 原实现记录宿主方法原名
/// （同一方法有多个别名时可重复出现）。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BatchAddFunctionResult {
    /// 注册成功的条目标识列表。对应 Java 字段 `succ`。
    succ: Vec<String>,
    /// 注册失败的条目标识列表。对应 Java 字段 `fail`。
    fail: Vec<String>,
}

impl BatchAddFunctionResult {
    /// 构造空结果。对应 Java 构造器 `BatchAddFunctionResult()`。
    pub fn new() -> Self {
        BatchAddFunctionResult {
            succ: Vec::new(),
            fail: Vec::new(),
        }
    }

    /// 成功函数名列表。对应 Java 方法 `getSucc()`。
    pub fn get_succ(&self) -> &Vec<String> {
        &self.succ
    }

    /// 失败函数名列表。对应 Java 方法 `getFail()`。
    pub fn get_fail(&self) -> &Vec<String> {
        &self.fail
    }

    /// 记录一个成功函数名(Java 侧由 Express4Runner 内部 `succ.add(name)`)。
    pub fn add_succ(&mut self, name: impl Into<String>) {
        self.succ.push(name.into());
    }

    /// 记录一个失败函数名(Java 侧 `fail.add(name)`)。
    /// 对应 Java: com.alibaba.qlexpress4.api.BatchAddFunctionResult#addFail。
    pub fn add_fail(&mut self, name: impl Into<String>) {
        self.fail.push(name.into());
    }

    /// 是否全部成功(便捷方法,Java 无对应,等价 `getFail().isEmpty()`)。
    /// 对应 Java: com.alibaba.qlexpress4.api.BatchAddFunctionResult#isAllSucc。
    pub fn is_all_succ(&self) -> bool {
        self.fail.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_failure_semantics() {
        // 部分失败语义:成功与失败分别归集,互不影响
        let mut result = BatchAddFunctionResult::new();
        assert!(result.is_all_succ());
        result.add_succ("f1");
        result.add_fail("f2");
        result.add_succ("f3");
        assert_eq!(result.get_succ(), &vec!["f1".to_string(), "f3".to_string()]);
        assert_eq!(result.get_fail(), &vec!["f2".to_string()]);
        assert!(!result.is_all_succ());
    }
}
