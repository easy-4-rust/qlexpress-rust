//! 脚本 Lambda 的函数包装,对应 Java `com.alibaba.qlexpress4.runtime.function.QLambdaFunction`。

use std::rc::Rc;

use crate::exception::QLException;
use crate::runtime::function::custom_function::CustomFunction;
use crate::runtime::parameters::Parameters;
use crate::runtime::qcontext::QContext;
use crate::runtime::qlambda::QLambda;
use crate::runtime::value::DataValue;

/// 把脚本内 `function` 定义的 Lambda 包装为 [`CustomFunction`]。
/// 对应 Java: com.alibaba.qlexpress4.runtime.function.QLambdaFunction
/// (`DefineFunctionInstruction` 用它把脚本函数注册进函数表)。
pub struct QLambdaFunction {
    /// 被包装的脚本 Lambda(Java `QLambda qLambda`)。
    q_lambda: Rc<QLambda>,
}

impl QLambdaFunction {
    /// 对应 Java 构造器 `QLambdaFunction(QLambda qLambda)`。
    pub fn new(q_lambda: Rc<QLambda>) -> Self {
        QLambdaFunction { q_lambda }
    }
}

impl CustomFunction for QLambdaFunction {
    /// 对应 Java 方法 `call(QContext, Parameters)`:
    /// 逐个取出参数值(`parameters.get(i).get()`)后调用 Lambda,
    /// 返回 `qLambda.call(...).getResult().get()`。
    fn call(
        &self,
        _q_context: &mut dyn QContext,
        parameters: &Parameters,
    ) -> Result<DataValue, QLException> {
        let params_arr = parameters.values();
        Ok(self.q_lambda.call(&params_arr)?.value())
    }
}
