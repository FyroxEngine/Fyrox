// Copyright (c) 2019-present Dmitry Stepanov and Fyrox Engine contributors.
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.

#[macro_export]
macro_rules! newtype_reflect {
    () => {
        fn type_name(&self) -> &'static str {
            self.0.type_name()
        }

        fn doc(&self) -> &'static str {
            self.0.doc()
        }

        fn fields_ref(&self, func: &mut dyn FnMut(&[$crate::reflect::FieldRef])) {
            self.0.fields_ref(func)
        }

        fn into_any(self: Box<Self>) -> Box<dyn std::any::Any> {
            self
        }

        fn as_any(&self, func: &mut dyn FnMut(&dyn std::any::Any)) {
            self.0.as_any(func)
        }

        fn as_any_mut(&mut self, func: &mut dyn FnMut(&mut dyn std::any::Any)) {
            self.0.as_any_mut(func)
        }

        fn inner_ref(&self, func: &mut dyn FnMut(&dyn $crate::reflect::Reflect)) {
            self.0.inner_ref(func)
        }

        fn inner_mut(&mut self, func: &mut dyn FnMut(&mut dyn $crate::reflect::Reflect)) {
            self.0.inner_mut(func)
        }

        fn set(
            &mut self,
            value: Box<dyn $crate::reflect::Reflect>,
        ) -> Result<Box<dyn $crate::reflect::Reflect>, Box<dyn $crate::reflect::Reflect>> {
            self.0.set(value)
        }

        fn find_field(
            &self,
            name: &str,
            func: &mut dyn FnMut(Option<&dyn $crate::reflect::Reflect>),
        ) {
            self.0.find_field(name, func)
        }

        fn find_field_mut(
            &mut self,
            name: &str,
            func: &mut dyn FnMut(Option<&mut dyn $crate::reflect::Reflect>),
        ) {
            self.0.find_field_mut(name, func)
        }

        fn as_array(&self, func: &mut dyn FnMut(Option<&dyn $crate::reflect::ReflectArray>)) {
            self.0.as_array(func)
        }

        fn as_array_mut(
            &mut self,
            func: &mut dyn FnMut(Option<&mut dyn $crate::reflect::ReflectArray>),
        ) {
            self.0.as_array_mut(func)
        }

        fn as_list(&self, func: &mut dyn FnMut(Option<&dyn $crate::reflect::ReflectList>)) {
            self.0.as_list(func)
        }

        fn as_list_mut(
            &mut self,
            func: &mut dyn FnMut(Option<&mut dyn $crate::reflect::ReflectList>),
        ) {
            self.0.as_list_mut(func)
        }

        fn as_inheritable_variable(
            &self,
            func: &mut dyn FnMut(Option<&dyn $crate::reflect::ReflectInheritableVariable>),
        ) {
            self.0.as_inheritable_variable(func)
        }

        fn as_inheritable_variable_mut(
            &mut self,
            func: &mut dyn FnMut(Option<&mut dyn $crate::reflect::ReflectInheritableVariable>),
        ) {
            self.0.as_inheritable_variable_mut(func)
        }

        fn as_handle(&self, func: &mut dyn FnMut(Option<&dyn $crate::reflect::ReflectHandle>)) {
            self.0.as_handle(func)
        }

        fn as_handle_mut(
            &mut self,
            func: &mut dyn FnMut(Option<&mut dyn $crate::reflect::ReflectHandle>),
        ) {
            self.0.as_handle_mut(func)
        }

        fn as_hash_map(&self, func: &mut dyn FnMut(Option<&dyn $crate::reflect::ReflectHashMap>)) {
            self.0.as_hash_map(func)
        }

        fn as_hash_map_mut(
            &mut self,
            func: &mut dyn FnMut(Option<&mut dyn $crate::reflect::ReflectHashMap>),
        ) {
            self.0.as_hash_map_mut(func)
        }
    };
}

#[macro_export]
macro_rules! blank_reflect {
    ($type_uuid:expr) => {
        fn type_info() -> $crate::reflect::TypeInfo {
            $crate::reflect::TypeInfo {
                source_path: file!(),
                type_name: std::any::type_name::<Self>(),
                assembly_name: env!("CARGO_PKG_NAME"),
                doc_comment: "",
                derived_types: &[],
                type_uuid: $crate::uuid::uuid!($type_uuid),
            }
        }

        fn type_info_ref(&self) -> $crate::reflect::TypeInfo {
            Self::type_info()
        }

        fn try_clone_box(&self) -> Option<Box<dyn $crate::reflect::Reflect>> {
            Some(Box::new(self.clone()))
        }

        fn fields_ref(&self, func: &mut dyn FnMut(&[$crate::reflect::FieldRef])) {
            func(&[])
        }

        #[inline]
        fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [$crate::reflect::FieldMut])) {
            func(&mut [])
        }

        fn field_direct_ref(&self, _index: usize) -> Option<$crate::reflect::FieldRef> {
            None
        }

        fn field_direct_mut(&mut self, _index: usize) -> Option<$crate::reflect::FieldMut> {
            None
        }

        fn set(
            &mut self,
            value: Box<dyn $crate::reflect::Reflect>,
        ) -> Result<Box<dyn $crate::reflect::Reflect>, Box<dyn $crate::reflect::Reflect>> {
            let this = std::mem::replace(self, value.take()?);
            Ok(Box::new(this))
        }
    };
}

#[macro_export]
macro_rules! blank_reflect_ref {
    ($type_uuid:expr) => {
        fn type_info() -> $crate::reflect::TypeInfo {
            $crate::reflect::TypeInfo {
                source_path: file!(),
                type_name: std::any::type_name::<Self>(),
                assembly_name: env!("CARGO_PKG_NAME"),
                doc_comment: "",
                derived_types: &[],
                type_uuid: $crate::uuid::uuid!($type_uuid),
            }
        }

        fn type_info_ref(&self) -> $crate::reflect::TypeInfo {
            Self::type_info()
        }

        fn try_clone_box(&self) -> Option<Box<dyn $crate::reflect::Reflect>> {
            None
        }

        fn fields_ref(&self, func: &mut dyn FnMut(&[$crate::reflect::FieldRef])) {
            func(&[])
        }

        #[inline]
        fn fields_mut(&mut self, func: &mut dyn FnMut(&mut [$crate::reflect::FieldMut])) {
            func(&mut [])
        }

        fn find_field(
            &self,
            name: &str,
            func: &mut dyn FnMut(Option<&dyn $crate::reflect::Reflect>),
        ) {
            func(if name == "self" { Some(self) } else { None })
        }

        fn find_field_mut(
            &mut self,
            name: &str,
            func: &mut dyn FnMut(Option<&mut dyn $crate::reflect::Reflect>),
        ) {
            func(if name == "self" { Some(self) } else { None })
        }

        fn field_direct_ref(&self, _index: usize) -> Option<$crate::reflect::FieldRef> {
            None
        }

        fn field_direct_mut(&mut self, _index: usize) -> Option<$crate::reflect::FieldMut> {
            None
        }

        fn set(
            &mut self,
            value: Box<dyn $crate::reflect::Reflect>,
        ) -> Result<Box<dyn $crate::reflect::Reflect>, Box<dyn $crate::reflect::Reflect>> {
            let this = std::mem::replace(self, value.take()?);
            Ok(Box::new(this))
        }
    };
}

pub use blank_reflect;
pub use newtype_reflect;
