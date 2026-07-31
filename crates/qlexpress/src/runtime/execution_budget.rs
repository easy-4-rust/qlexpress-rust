//! 单次安全执行的运行期预算状态。

use std::cell::Cell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::{Duration, Instant};

use crate::exception::QLException;
use crate::exception::ql_exception::QLExceptionKind;
use crate::runtime::value::DataValue;
use crate::security::{CancellationToken, ResourceLimits};

/// 单次执行共享的 fuel、调用深度、集合和截止时间状态。
///
/// 该对象是 Rust 沙箱扩展；普通 Java 兼容执行路径不创建它。
/// 对应 Java: 无（Rust 安全增强）。
pub struct ExecutionBudget {
    limits: ResourceLimits,
    cancellation_token: CancellationToken,
    deadline: Instant,
    fuel_used: Cell<u64>,
    call_depth: Cell<usize>,
    collection_items: Cell<usize>,
}

impl ExecutionBudget {
    /// 根据安全预算和外部取消令牌创建状态。
    pub fn new(limits: ResourceLimits, cancellation_token: CancellationToken) -> Self {
        let deadline = Instant::now() + Duration::from_millis(limits.timeout_millis);
        Self {
            limits,
            cancellation_token,
            deadline,
            fuel_used: Cell::new(0),
            call_depth: Cell::new(0),
            collection_items: Cell::new(0),
        }
    }

    /// 返回本次执行的绝对截止时间，供宿主函数设置下游超时。
    pub fn deadline(&self) -> Instant {
        self.deadline
    }

    /// 返回共享取消令牌。
    pub fn cancellation_token(&self) -> &CancellationToken {
        &self.cancellation_token
    }

    /// 在一个可抢占检查点校验取消和截止时间。
    pub fn checkpoint(&self) -> Result<(), QLException> {
        if self.cancellation_token.is_cancelled() {
            return Err(budget_error(
                QLExceptionKind::Timeout,
                "SANDBOX_CANCELLED",
                "sandbox execution was cancelled",
            ));
        }
        if Instant::now() >= self.deadline {
            return Err(budget_error(
                QLExceptionKind::Timeout,
                "SANDBOX_DEADLINE_EXCEEDED",
                "sandbox execution deadline exceeded",
            ));
        }
        Ok(())
    }

    /// 消耗 QVM fuel；每次实际取指至少消耗一个单位。
    pub fn consume_fuel(&self, amount: u64) -> Result<(), QLException> {
        self.checkpoint()?;
        let next = self.fuel_used.get().saturating_add(amount);
        if next > self.limits.max_fuel {
            return Err(budget_error(
                QLExceptionKind::Runtime,
                "SANDBOX_FUEL_EXCEEDED",
                format!(
                    "sandbox fuel exceeded: used {next}, limit {}",
                    self.limits.max_fuel
                ),
            ));
        }
        self.fuel_used.set(next);
        Ok(())
    }

    /// 进入脚本 Lambda、宿主函数或原生方法调用。
    pub fn enter_call(&self) -> Result<(), QLException> {
        self.checkpoint()?;
        let next = self.call_depth.get().saturating_add(1);
        if next > self.limits.max_call_depth {
            return Err(budget_error(
                QLExceptionKind::Runtime,
                "SANDBOX_CALL_DEPTH_EXCEEDED",
                format!(
                    "sandbox call depth exceeded: depth {next}, limit {}",
                    self.limits.max_call_depth
                ),
            ));
        }
        self.call_depth.set(next);
        Ok(())
    }

    /// 离开一次调用。所有进入路径都必须在返回前配对调用。
    pub fn exit_call(&self) {
        self.call_depth.set(self.call_depth.get().saturating_sub(1));
    }

    /// 记录本次创建或从宿主接收的集合元素。
    pub fn charge_collection_items(&self, amount: usize) -> Result<(), QLException> {
        self.checkpoint()?;
        let next = self.collection_items.get().saturating_add(amount);
        if next > self.limits.max_collection_items {
            return Err(budget_error(
                QLExceptionKind::Runtime,
                "SANDBOX_COLLECTION_ITEMS_EXCEEDED",
                format!(
                    "sandbox collection item budget exceeded: items {next}, limit {}",
                    self.limits.max_collection_items
                ),
            ));
        }
        self.collection_items.set(next);
        Ok(())
    }

    /// 在字符串扩容前校验目标 UTF-8 字节数。
    pub fn check_string_bytes(&self, bytes: usize) -> Result<(), QLException> {
        self.checkpoint()?;
        if bytes > self.limits.max_string_bytes {
            return Err(budget_error(
                QLExceptionKind::Runtime,
                "SANDBOX_STRING_BYTES_EXCEEDED",
                format!(
                    "sandbox string bytes exceeded: bytes {bytes}, limit {}",
                    self.limits.max_string_bytes
                ),
            ));
        }
        Ok(())
    }

    /// 校验新产生的值中字符串、单集合大小和嵌套值规模。
    pub fn validate_value(&self, value: &DataValue) -> Result<(), QLException> {
        self.checkpoint()?;
        let mut visited = HashSet::new();
        self.validate_value_inner(value, &mut visited)
    }

    fn validate_value_inner(
        &self,
        value: &DataValue,
        visited: &mut HashSet<usize>,
    ) -> Result<(), QLException> {
        match value {
            DataValue::Str(value) => {
                let bytes = value.len().saturating_mul(std::mem::size_of::<u16>());
                if bytes > self.limits.max_string_bytes {
                    return Err(budget_error(
                        QLExceptionKind::Runtime,
                        "SANDBOX_STRING_BYTES_EXCEEDED",
                        format!(
                            "sandbox string bytes exceeded: bytes {}, limit {}",
                            bytes, self.limits.max_string_bytes
                        ),
                    ));
                }
            }
            DataValue::BigDec(value) => {
                if value.len() > self.limits.max_string_bytes {
                    return Err(budget_error(
                        QLExceptionKind::Runtime,
                        "SANDBOX_STRING_BYTES_EXCEEDED",
                        format!(
                            "sandbox string bytes exceeded: bytes {}, limit {}",
                            value.len(),
                            self.limits.max_string_bytes
                        ),
                    ));
                }
            }
            DataValue::List(values) => {
                let identity = Rc::as_ptr(values) as usize;
                if !visited.insert(identity) {
                    return Ok(());
                }
                let values = values.borrow();
                if values.len() > self.limits.max_collection_items {
                    return Err(budget_error(
                        QLExceptionKind::Runtime,
                        "SANDBOX_COLLECTION_ITEMS_EXCEEDED",
                        "individual collection exceeds sandbox item limit",
                    ));
                }
                for value in values.iter() {
                    self.validate_value_inner(value, visited)?;
                }
            }
            DataValue::Array(values) => {
                let identity = Rc::as_ptr(values) as usize;
                if !visited.insert(identity) {
                    return Ok(());
                }
                let values = values.borrow();
                if values.len() > self.limits.max_collection_items {
                    return Err(budget_error(
                        QLExceptionKind::Runtime,
                        "SANDBOX_COLLECTION_ITEMS_EXCEEDED",
                        "individual collection exceeds sandbox item limit",
                    ));
                }
                for value in values.iter() {
                    self.validate_value_inner(value, visited)?;
                }
            }
            DataValue::Map(values) => {
                let identity = Rc::as_ptr(values) as usize;
                if !visited.insert(identity) {
                    return Ok(());
                }
                let values = values.borrow();
                if values.len() > self.limits.max_collection_items {
                    return Err(budget_error(
                        QLExceptionKind::Runtime,
                        "SANDBOX_COLLECTION_ITEMS_EXCEEDED",
                        "individual map exceeds sandbox item limit",
                    ));
                }
                for (key, value) in values.entries() {
                    self.validate_value_inner(key, visited)?;
                    self.validate_value_inner(value, visited)?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    /// 校验最终结果的估算输出大小。
    pub fn validate_output(&self, value: &DataValue) -> Result<(), QLException> {
        self.validate_value(value)?;
        let mut visited = HashSet::new();
        let size = estimated_size(value, &mut visited, self.limits.max_output_bytes)?;
        if size > self.limits.max_output_bytes {
            return Err(budget_error(
                QLExceptionKind::Runtime,
                "SANDBOX_OUTPUT_BYTES_EXCEEDED",
                format!(
                    "sandbox output bytes exceeded: estimated {size}, limit {}",
                    self.limits.max_output_bytes
                ),
            ));
        }
        Ok(())
    }

    /// 对宿主函数、扩展函数或 Native 方法返回的新值计入集合预算。
    pub fn charge_external_value(&self, value: &DataValue) -> Result<(), QLException> {
        self.validate_value(value)?;
        let mut visited = HashSet::new();
        let items = collection_item_count(value, &mut visited);
        self.charge_collection_items(items)
    }
}

fn collection_item_count(value: &DataValue, visited: &mut HashSet<usize>) -> usize {
    match value {
        DataValue::List(values) => {
            let identity = Rc::as_ptr(values) as usize;
            if !visited.insert(identity) {
                return 0;
            }
            let values = values.borrow();
            values.iter().fold(values.len(), |total, value| {
                total.saturating_add(collection_item_count(value, visited))
            })
        }
        DataValue::Array(values) => {
            let identity = Rc::as_ptr(values) as usize;
            if !visited.insert(identity) {
                return 0;
            }
            let values = values.borrow();
            values.iter().fold(values.len(), |total, value| {
                total.saturating_add(collection_item_count(value, visited))
            })
        }
        DataValue::Map(values) => {
            let identity = Rc::as_ptr(values) as usize;
            if !visited.insert(identity) {
                return 0;
            }
            let values = values.borrow();
            values
                .entries()
                .iter()
                .fold(values.len(), |total, (key, value)| {
                    total
                        .saturating_add(collection_item_count(key, visited))
                        .saturating_add(collection_item_count(value, visited))
                })
        }
        _ => 0,
    }
}

fn estimated_size(
    value: &DataValue,
    visited: &mut HashSet<usize>,
    stop_after: usize,
) -> Result<usize, QLException> {
    let mut size = match value {
        DataValue::Null => 4,
        DataValue::Bool(_) => 5,
        DataValue::Byte(_) | DataValue::Short(_) | DataValue::Int(_) => 16,
        DataValue::Long(_) | DataValue::Float(_) | DataValue::Double(_) => 32,
        DataValue::BigInt(value) => value.to_string().len(),
        DataValue::BigDec(value) => value.len(),
        DataValue::Str(value) => value.len().saturating_mul(std::mem::size_of::<u16>()),
        DataValue::Char(_) => 2,
        DataValue::Lambda(_) | DataValue::Object(_) => 64,
        DataValue::List(values) => {
            let identity = Rc::as_ptr(values) as usize;
            if !visited.insert(identity) {
                return Ok(0);
            }
            let mut total = 2usize;
            for item in values.borrow().iter() {
                total = total.saturating_add(estimated_size(item, visited, stop_after)? + 1);
                if total > stop_after {
                    break;
                }
            }
            total
        }
        DataValue::Array(values) => {
            let identity = Rc::as_ptr(values) as usize;
            if !visited.insert(identity) {
                return Ok(0);
            }
            let mut total = 2usize;
            for item in values.borrow().iter() {
                total = total.saturating_add(estimated_size(item, visited, stop_after)? + 1);
                if total > stop_after {
                    break;
                }
            }
            total
        }
        DataValue::Map(values) => {
            let identity = Rc::as_ptr(values) as usize;
            if !visited.insert(identity) {
                return Ok(0);
            }
            let mut total = 2usize;
            for (key, value) in values.borrow().entries() {
                total = total
                    .saturating_add(estimated_size(key, visited, stop_after)?)
                    .saturating_add(estimated_size(value, visited, stop_after)?)
                    .saturating_add(2);
                if total > stop_after {
                    break;
                }
            }
            total
        }
    };
    if size > stop_after {
        size = stop_after.saturating_add(1);
    }
    Ok(size)
}

/// 构造统一的沙箱错误。
pub(crate) fn budget_error(
    kind: QLExceptionKind,
    code: &'static str,
    reason: impl Into<String>,
) -> QLException {
    QLException::for_test(kind, reason, code)
}
