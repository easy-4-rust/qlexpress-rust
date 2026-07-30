//! Type-conversion 子包，对应 Java `runtime/data/convert/`。
//!
//! 仅做模块声明与 re-export，符合"mod.rs 不定义类型/逻辑"规范（SPEC §2）：
//! - [`obj_type_convertor`]：单值类型转换（Java `ObjTypeConvertor`）
//! - [`parameters_type_convertor`]：参数列表转换（Java `ParametersTypeConvertor`）
//! - [`number_math_helpers`]：🆕 Rust 化新增（数域提升/比较/转换辅助）

pub mod number_math_helpers;
pub mod obj_type_convertor;
pub mod parameters_type_convertor;
pub mod q_converted;
pub mod target_type;

pub use number_math_helpers::{
    big_dec_compare, big_dec_to_big_int, big_dec_to_i128, math_domain, num_kind, number_compare,
    java_f32_to_string, java_f64_to_string, promote, to_big_dec_string, to_big_int, to_f64,
    to_i128, to_i64, MathDomain,
};
pub use obj_type_convertor::{ObjTypeConvertor, QConverted, TargetType};
pub use parameters_type_convertor::ParametersTypeConvertor;
