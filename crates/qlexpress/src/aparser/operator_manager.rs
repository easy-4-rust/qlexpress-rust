//! 编译期操作符、别名与优先级注册表。

use std::collections::HashMap;
use std::rc::Rc;

use crate::operator::BinaryOperator;
use crate::unary::UnaryOperator;

/// 管理内建、自定义操作符和关键字别名。
///
/// 对应 Java: `com.alibaba.qlexpress4.aparser.OperatorManager`。
#[derive(Default)]
pub struct OperatorManager {
    pub(crate) default_binary_operator_map: HashMap<String, Rc<dyn BinaryOperator>>,
    pub(crate) default_prefix_unary_operator_map: HashMap<String, Rc<dyn UnaryOperator>>,
    pub(crate) default_suffix_unary_operator_map: HashMap<String, Rc<dyn UnaryOperator>>,
    pub(crate) custom_binary_operator_map: HashMap<String, Rc<dyn BinaryOperator>>,
    pub(crate) key_word_aliases: HashMap<String, i32>,
}
