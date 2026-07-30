//! Java 强类型数组的 Rust 运行时表示。

use std::ops::Index;
use std::rc::Rc;

use crate::runtime::class_ref::ClassRef;
use crate::runtime::native_registry::NativeRegistry;
use crate::runtime::value::DataValue;

/// 保存数组元素、声明组件类型和定义现场类型注册表。
///
/// 对应 Java 数组对象：`array.getClass().getComponentType()` 在空数组上也
/// 必须稳定，且元素写入需持续执行组件类型校验。
#[derive(Clone)]
pub struct JavaArray {
    values: Vec<DataValue>,
    component_type: ClassRef,
    type_registry: Option<Rc<NativeRegistry>>,
}

impl JavaArray {
    /// 创建 Java `Object[]` 适配值。
    pub fn object(values: Vec<DataValue>) -> Self {
        Self {
            values,
            component_type: ClassRef::Named("java.lang.Object".to_string()),
            type_registry: None,
        }
    }

    /// 创建携带完整声明组件类型的数组。
    pub fn typed(
        values: Vec<DataValue>,
        component_type: ClassRef,
        type_registry: Rc<NativeRegistry>,
    ) -> Self {
        Self {
            values,
            component_type,
            type_registry: Some(type_registry),
        }
    }

    /// 创建无需宿主继承查询的声明类型数组。
    ///
    /// 用于原语数组、内建数组和宿主直接传入的已类型化数组。
    pub fn typed_without_registry(values: Vec<DataValue>, component_type: ClassRef) -> Self {
        Self {
            values,
            component_type,
            type_registry: None,
        }
    }

    /// 返回数组长度。
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// 判断数组是否为空。
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// 返回指定元素引用。
    pub fn get(&self, index: usize) -> Option<&DataValue> {
        self.values.get(index)
    }

    /// 顺序遍历元素。
    pub fn iter(&self) -> std::slice::Iter<'_, DataValue> {
        self.values.iter()
    }

    /// 返回底层切片。
    pub fn as_slice(&self) -> &[DataValue] {
        &self.values
    }

    /// 克隆元素列表，不复制数组身份。
    pub fn to_vec(&self) -> Vec<DataValue> {
        self.values.clone()
    }

    /// 以相同组件类型和注册表创建新的数组身份。
    ///
    /// 对应 Java `Arrays.copyOfRange` 在切片时保留原数组类。
    pub fn copy_with_values(&self, values: Vec<DataValue>) -> Self {
        Self {
            values,
            component_type: self.component_type.clone(),
            type_registry: self.type_registry.as_ref().map(Rc::clone),
        }
    }

    /// 返回 Java 声明组件类型。
    pub fn component_type(&self) -> &ClassRef {
        &self.component_type
    }

    /// 返回定义现场类型注册表。
    pub fn type_registry(&self) -> Option<&Rc<NativeRegistry>> {
        self.type_registry.as_ref()
    }

    /// 替换指定元素；调用者须先完成组件类型转换。
    pub fn set(&mut self, index: usize, value: DataValue) {
        self.values[index] = value;
    }
}

impl<I> Index<I> for JavaArray
where
    Vec<DataValue>: Index<I>,
{
    type Output = <Vec<DataValue> as Index<I>>::Output;

    fn index(&self, index: I) -> &Self::Output {
        &self.values[index]
    }
}

impl std::fmt::Debug for JavaArray {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("JavaArray")
            .field("component_type", &self.component_type)
            .field("values", &self.values)
            .finish()
    }
}
