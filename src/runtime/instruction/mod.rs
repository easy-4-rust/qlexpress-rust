//! QVM instructions, mirroring Java
//! `com.alibaba.qlexpress4.runtime.instruction` (all 42 classes).
//!
//! Every instruction keeps the Java javadoc *Specification* (Operation /
//! Input / Output) as its doc comment, reports through its own
//! `ErrorReporter`, and reproduces the Java debug `println` output.
//!
//! 一类一文件(SPEC §5.5):每个指令一个文件,文件名 = Java 类名 snake_case;
//! 本模块只做 mod 声明 + re-export,外部 `use` 路径保持不变。

pub mod break_continue_instruction;
pub mod call_const_instruction;
pub mod call_function_instruction;
pub mod call_instruction;
pub mod cast_instruction;
pub mod check_time_out_instruction;
pub mod close_scope_instruction;
pub mod const_instruction;
pub mod define_function_instruction;
pub mod define_local_instruction;
pub mod for_each_instruction;
pub mod for_instruction;
pub mod get_field_instruction;
pub mod get_method_instruction;
pub mod index_instruction;
pub mod jump_if_instruction;
pub mod jump_if_pop_instruction;
pub mod jump_instruction;
pub mod load_instruction;
pub mod load_lambda_instruction;
pub mod method_invoke_instruction;
pub mod multi_new_array_instruction;
pub mod new_array_instruction;
pub mod new_filled_instance_instruction;
pub mod new_instance_instruction;
pub mod new_list_instruction;
pub mod new_map_instruction;
pub mod new_scope_instruction;
pub mod operator_instruction;
pub mod pop_instruction;
pub mod ql_instruction;
pub mod return_instruction;
pub mod slice_instruction;
pub mod spread_get_field_instruction;
pub mod spread_method_invoke_instruction;
pub mod string_join_instruction;
pub mod throw_instruction;
pub mod trace_evaluated_instruction;
pub mod trace_peek_instruction;
pub mod try_catch_instruction;
pub mod unary_instruction;
pub mod while_instruction;

pub use break_continue_instruction::BreakContinueInstruction;
pub use call_const_instruction::CallConstInstruction;
pub use call_function_instruction::CallFunctionInstruction;
pub use call_instruction::CallInstruction;
pub use cast_instruction::CastInstruction;
pub use check_time_out_instruction::CheckTimeOutInstruction;
pub use close_scope_instruction::CloseScopeInstruction;
pub use const_instruction::ConstInstruction;
pub use define_function_instruction::DefineFunctionInstruction;
pub use define_local_instruction::DefineLocalInstruction;
pub use for_each_instruction::ForEachInstruction;
pub use for_instruction::ForInstruction;
pub use get_field_instruction::GetFieldInstruction;
pub use get_method_instruction::GetMethodInstruction;
pub use index_instruction::IndexInstruction;
pub use jump_if_instruction::JumpIfInstruction;
pub use jump_if_pop_instruction::JumpIfPopInstruction;
pub use jump_instruction::JumpInstruction;
pub use load_instruction::LoadInstruction;
pub use load_lambda_instruction::LoadLambdaInstruction;
pub use method_invoke_instruction::MethodInvokeInstruction;
pub use multi_new_array_instruction::MultiNewArrayInstruction;
pub use new_array_instruction::NewArrayInstruction;
pub use new_filled_instance_instruction::NewFilledInstanceInstruction;
pub use new_instance_instruction::NewInstanceInstruction;
pub use new_list_instruction::NewListInstruction;
pub use new_map_instruction::NewMapInstruction;
pub use new_scope_instruction::NewScopeInstruction;
pub use operator_instruction::OperatorInstruction;
pub use pop_instruction::PopInstruction;
pub use ql_instruction::{Instruction, QLInstruction};
pub use return_instruction::{ReturnInstruction, ReturnResultType};
pub use slice_instruction::{SliceInstruction, SliceMode};
pub use spread_get_field_instruction::SpreadGetFieldInstruction;
pub use spread_method_invoke_instruction::SpreadMethodInvokeInstruction;
pub use string_join_instruction::StringJoinInstruction;
pub use throw_instruction::ThrowInstruction;
pub use trace_evaluated_instruction::TraceEvaluatedInstruction;
pub use trace_peek_instruction::TracePeekInstruction;
pub use try_catch_instruction::TryCatchInstruction;
pub use unary_instruction::UnaryInstruction;
pub use while_instruction::WhileInstruction;

pub(crate) use ql_instruction::with_trace;
