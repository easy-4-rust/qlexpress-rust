#![allow(dead_code)]
//! Shared mock operators for Stage 3b tests (Stage 4 delivers real ones).
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use qlexpress_rust::aparser::compile_time_function::{CodeGenerator, CompileTimeFunction};
use qlexpress_rust::aparser::import_manager::ImportManager;
use qlexpress_rust::aparser::interpolation_mode::InterpolationMode;
use qlexpress_rust::aparser::operator_factory::{OperatorFactory, OperatorManager};
use qlexpress_rust::aparser::qlparser::build_tree;
use qlexpress_rust::aparser::qvm_instruction_visitor::{
    compile_script, CompileTimeFunctions, UserDefineFunctions,
};
use qlexpress_rust::class_supplier::DefaultClassSupplier;
use qlexpress_rust::exception::error_reporter::ErrorReporter;
use qlexpress_rust::exception::pure_err_reporter::PureErrReporter;
use qlexpress_rust::exception::QLException;
use qlexpress_rust::init_options::InitOptions;
use qlexpress_rust::ql_options::QLOptions;
use qlexpress_rust::ql_precedences;
use qlexpress_rust::runtime::data::index_map::IndexMap;
use qlexpress_rust::runtime::function::{CustomFunction, LazyArgCustomFunction};
use qlexpress_rust::runtime::instruction::{ConstInstruction, Instruction, QLInstruction};
use qlexpress_rust::runtime::member::{NativeRegistry, NativeType};
use qlexpress_rust::runtime::operator::base::{BinaryOperator, UnaryOperator};
use qlexpress_rust::runtime::operator::custom_binary_operator::CustomBinaryOperator;
use qlexpress_rust::runtime::parameters::Parameters;
use qlexpress_rust::runtime::qcontext::QContext;
use qlexpress_rust::runtime::qlambda_definition::QLambdaDefinition;
use qlexpress_rust::runtime::qlambda_definition_inner::QLambdaDefinitionInner;
use qlexpress_rust::runtime::qvm_global_scope::QvmGlobalScope;
use qlexpress_rust::runtime::qvm_runtime::QvmRuntime;
use qlexpress_rust::runtime::value::{DataValue, QValue};

// ---- mock operators (Stage 4 delivers the real ones) ---------------------

#[derive(Clone, Copy)]
enum BinKind {
    Assign,
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
}

struct MockBin {
    kind: BinKind,
    lexeme: &'static str,
    priority: i32,
}

pub fn as_f64(v: &DataValue) -> Option<f64> {
    match v {
        DataValue::Byte(x) => Some(*x as f64),
        DataValue::Short(x) => Some(*x as f64),
        DataValue::Int(x) => Some(*x as f64),
        DataValue::Long(x) => Some(*x as f64),
        DataValue::Float(x) => Some(*x as f64),
        DataValue::Double(x) => Some(*x),
        DataValue::BigInt(x) => Some(*x as f64),
        _ => None,
    }
}

fn is_float(v: &DataValue) -> bool {
    matches!(v, DataValue::Float(_) | DataValue::Double(_))
}

fn num_bin(l: &DataValue, r: &DataValue, f: impl Fn(f64, f64) -> f64) -> DataValue {
    let v = f(as_f64(l).unwrap(), as_f64(r).unwrap());
    if is_float(l) || is_float(r) {
        DataValue::Double(v)
    } else if matches!(l, DataValue::Int(_)) && matches!(r, DataValue::Int(_)) {
        DataValue::Int(v as i32)
    } else {
        DataValue::Long(v as i64)
    }
}

impl BinaryOperator for MockBin {
    fn execute(
        &self,
        left: &QValue,
        right: &QValue,
        _ctx: &mut dyn QContext,
        _opts: &QLOptions,
        reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let l = left.get();
        let r = right.get();
        Ok(match self.kind {
            BinKind::Assign => {
                let Some(slot) = left.as_left() else {
                    return Err(reporter.report("INVALID_ASSIGNMENT", "not a left value"));
                };
                slot.borrow_mut().set(r.clone(), reporter)?;
                r
            }
            BinKind::Add => {
                if matches!(l, DataValue::Str(_)) || matches!(r, DataValue::Str(_)) {
                    DataValue::Str(format!("{}{}", l.string_value_of(), r.string_value_of()))
                } else {
                    num_bin(&l, &r, |a, b| a + b)
                }
            }
            BinKind::Sub => num_bin(&l, &r, |a, b| a - b),
            BinKind::Mul => num_bin(&l, &r, |a, b| a * b),
            BinKind::Div => num_bin(&l, &r, |a, b| a / b),
            BinKind::Rem => num_bin(&l, &r, |a, b| a % b),
            BinKind::Eq => DataValue::Bool(l == r),
            BinKind::Ne => DataValue::Bool(l != r),
            BinKind::Lt => DataValue::Bool(as_f64(&l).zip(as_f64(&r)).is_some_and(|(a, b)| a < b)),
            BinKind::Le => DataValue::Bool(as_f64(&l).zip(as_f64(&r)).is_some_and(|(a, b)| a <= b)),
            BinKind::Gt => DataValue::Bool(as_f64(&l).zip(as_f64(&r)).is_some_and(|(a, b)| a > b)),
            BinKind::Ge => DataValue::Bool(as_f64(&l).zip(as_f64(&r)).is_some_and(|(a, b)| a >= b)),
            BinKind::And => match (&l, &r) {
                (DataValue::Bool(a), DataValue::Bool(b)) => DataValue::Bool(*a && *b),
                _ => return Err(reporter.report("INVALID_BINARY_OPERAND", "not booleans")),
            },
            BinKind::Or => match (&l, &r) {
                (DataValue::Bool(a), DataValue::Bool(b)) => DataValue::Bool(*a || *b),
                _ => return Err(reporter.report("INVALID_BINARY_OPERAND", "not booleans")),
            },
        })
    }

    fn operator(&self) -> &str {
        self.lexeme
    }

    fn priority(&self) -> i32 {
        self.priority
    }
}

struct MockUnary {
    lexeme: &'static str,
    not: bool,
}

impl UnaryOperator for MockUnary {
    fn execute(
        &self,
        value: &QValue,
        reporter: &dyn ErrorReporter,
    ) -> Result<DataValue, QLException> {
        let v = value.get();
        if self.not {
            return match v {
                DataValue::Bool(b) => Ok(DataValue::Bool(!b)),
                _ => Err(reporter.report("INVALID_UNARY_OPERAND", "not boolean")),
            };
        }
        // unary minus
        match v {
            DataValue::Int(x) => Ok(DataValue::Int(-x)),
            DataValue::Long(x) => Ok(DataValue::Long(-x)),
            DataValue::Double(x) => Ok(DataValue::Double(-x)),
            _ => Err(reporter.report("INVALID_UNARY_OPERAND", "not number")),
        }
    }

    fn operator(&self) -> &str {
        self.lexeme
    }

    fn priority(&self) -> i32 {
        ql_precedences::UNARY
    }
}

pub fn operator_manager() -> OperatorManager {
    let mut manager = OperatorManager::new();
    let mut reg = |lexeme: &'static str, kind: BinKind, priority: i32| {
        manager.register_default_binary_operator(Rc::new(MockBin {
            kind,
            lexeme,
            priority,
        }));
    };
    reg("=", BinKind::Assign, ql_precedences::ASSIGN);
    reg("||", BinKind::Or, ql_precedences::OR);
    reg("&&", BinKind::And, ql_precedences::AND);
    reg("==", BinKind::Eq, ql_precedences::EQUAL);
    reg("!=", BinKind::Ne, ql_precedences::EQUAL);
    reg("<", BinKind::Lt, ql_precedences::COMPARE);
    reg("<=", BinKind::Le, ql_precedences::COMPARE);
    reg(">", BinKind::Gt, ql_precedences::COMPARE);
    reg(">=", BinKind::Ge, ql_precedences::COMPARE);
    reg("+", BinKind::Add, ql_precedences::ADD);
    reg("-", BinKind::Sub, ql_precedences::ADD);
    reg("*", BinKind::Mul, ql_precedences::MULTI);
    reg("/", BinKind::Div, ql_precedences::MULTI);
    reg("%", BinKind::Rem, ql_precedences::MULTI);
    manager.register_default_prefix_unary_operator(Rc::new(MockUnary {
        lexeme: "!",
        not: true,
    }));
    manager.register_default_prefix_unary_operator(Rc::new(MockUnary {
        lexeme: "-",
        not: false,
    }));
    manager
}
