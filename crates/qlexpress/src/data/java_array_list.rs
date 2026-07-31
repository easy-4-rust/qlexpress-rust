//! Java `ArrayList` 的 Rust 运行时存储适配。

use std::ops::Index;

use crate::runtime::value::DataValue;

/// 保存列表元素以及 Java `AbstractList.modCount` 等价状态。
///
/// 对应 Java: `java.util.ArrayList`。QLExpress 的 `for-each` 直接使用
/// `Iterable.iterator()`，因此结构性修改必须使既有迭代器失效；仅保存
/// `Vec<DataValue>` 会错误地把循环降级成快照遍历。
#[derive(Clone)]
pub struct JavaArrayList {
    values: Vec<DataValue>,
    mod_count: u64,
}

impl JavaArrayList {
    /// 使用给定元素创建列表。
    ///
    /// 参数：`values` 为初始元素；返回：修改计数为零的新列表。
    /// 对应 Java：`java.util.ArrayList#ArrayList(Collection)`。
    pub fn new(values: Vec<DataValue>) -> Self {
        Self {
            values,
            mod_count: 0,
        }
    }

    /// 创建具有指定预分配容量的空列表。
    ///
    /// 参数：`capacity` 为预分配容量；返回：空列表。
    /// 对应 Java：`java.util.ArrayList#ArrayList(int)`。
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            values: Vec::with_capacity(capacity),
            mod_count: 0,
        }
    }

    /// 返回元素数量。
    /// 对应 Java：`java.util.ArrayList#size()`。
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// 判断列表是否为空。
    /// 对应 Java：`java.util.ArrayList#isEmpty()`。
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// 返回指定位置的元素引用。
    ///
    /// 参数：`index` 为零基下标；返回：存在时的元素引用。
    /// 对应 Java：`java.util.ArrayList#get(int)`。
    pub fn get(&self, index: usize) -> Option<&DataValue> {
        self.values.get(index)
    }

    /// 按顺序遍历所有元素。
    /// 对应 Java：`java.util.ArrayList#iterator()` 的 encounter order。
    pub fn iter(&self) -> std::slice::Iter<'_, DataValue> {
        self.values.iter()
    }

    /// 返回底层元素切片。
    /// 对应 Java：无（Rust 对 Java 列表存储的借用接口）。
    pub fn as_slice(&self) -> &[DataValue] {
        &self.values
    }

    /// 克隆当前元素，不复制列表身份。
    /// 对应 Java：`java.util.ArrayList#toArray()` 的元素快照语义。
    pub fn to_vec(&self) -> Vec<DataValue> {
        self.values.clone()
    }

    /// 返回结构修改计数，供 fail-fast 迭代器校验。
    /// 对应 Java：`java.util.AbstractList#modCount`。
    pub fn mod_count(&self) -> u64 {
        self.mod_count
    }

    /// 在尾部添加元素并记录结构修改。
    ///
    /// 对应 Java: `ArrayList#add(Object)`。
    pub fn push(&mut self, value: DataValue) {
        self.values.push(value);
        self.mod_count = self.mod_count.wrapping_add(1);
    }

    /// 添加一组元素并记录结构修改。
    ///
    /// 对应 Java: `ArrayList#addAll(Collection)`。Java `ArrayList` 即使
    /// 传入空集合也会递增 `modCount`，这里保留该细节。
    pub fn extend(&mut self, values: impl IntoIterator<Item = DataValue>) {
        self.mod_count = self.mod_count.wrapping_add(1);
        self.values.extend(values);
    }

    /// 删除并返回指定位置元素，同时记录结构修改。
    ///
    /// 对应 Java: `ArrayList#remove(int)`。
    pub fn remove(&mut self, index: usize) -> DataValue {
        let value = self.values.remove(index);
        self.mod_count = self.mod_count.wrapping_add(1);
        value
    }

    /// 清空列表并记录结构修改。
    ///
    /// 对应 Java: `ArrayList#clear()`；Java 对空列表调用 `clear` 也会
    /// 递增 `modCount`。
    pub fn clear(&mut self) {
        self.values.clear();
        self.mod_count = self.mod_count.wrapping_add(1);
    }

    /// 替换指定元素并返回旧值，不改变结构修改计数。
    ///
    /// 对应 Java: `ArrayList#set(int,Object)`。
    pub fn set(&mut self, index: usize, value: DataValue) -> DataValue {
        std::mem::replace(&mut self.values[index], value)
    }
}

impl<I> Index<I> for JavaArrayList
where
    Vec<DataValue>: Index<I>,
{
    type Output = <Vec<DataValue> as Index<I>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.values[index]
    }
}

impl PartialEq for JavaArrayList {
    fn eq(&self, other: &Self) -> bool {
        self.values == other.values
    }
}

impl PartialEq<Vec<DataValue>> for JavaArrayList {
    fn eq(&self, other: &Vec<DataValue>) -> bool {
        &self.values == other
    }
}

impl std::fmt::Debug for JavaArrayList {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.values.fmt(formatter)
    }
}
