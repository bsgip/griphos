macro_rules! py_value {
    ($py_name:ident, $rust_ty:path { $($vis:vis $field:ident: $field_ty:ty),* $(,)? }) => {
        #[pyo3_stub_gen::derive::gen_stub_pyclass]
        #[pyclass(module = "griphos", get_all, eq, frozen, from_py_object)]
        #[derive(Clone, PartialEq)]
        pub struct $py_name {
            $($vis $field: $field_ty,)*
        }

        #[pyo3_stub_gen::derive::gen_stub_pymethods]
        #[pymethods]
        impl $py_name {
            #[new]
            fn new($($field: $field_ty),*) -> Self {
                $py_name { $($field,)* }
            }
        }

        impl From<$rust_ty> for $py_name {
            fn from(v: $rust_ty) -> Self {
                $py_name { $($field: v.$field.into(),)* }
            }
        }

        impl From<&$rust_ty> for $py_name {
            fn from(v: &$rust_ty) -> Self {
                $py_name { $($field: v.$field.clone().into(),)* }
            }
        }

        impl From<$py_name> for $rust_ty {
            fn from(v: $py_name) -> Self {
                $rust_ty { $($field: v.$field.into(),)* }
            }
        }

        impl From<&$py_name> for $rust_ty {
            fn from(v: &$py_name) -> Self {
                $rust_ty { $($field: v.$field.clone().into(),)* }
            }
        }
    };
}
pub(crate) use py_value;

macro_rules! py_enum {
    ($py_name:ident, $rust_ty:ty { $($variant:ident),* $(,)? }) => {
        #[pyo3_stub_gen::derive::gen_stub_pyclass_enum]
        #[pyclass(module = "griphos", eq, eq_int, from_py_object)]
        #[derive(Clone, Copy, PartialEq, Hash)]
        pub enum $py_name {
            $($variant,)*
        }

        impl From<&$rust_ty> for $py_name {
            fn from(v: &$rust_ty) -> Self {
                match v {
                    $(<$rust_ty>::$variant => $py_name::$variant,)*
                }
            }
        }

        impl From<$rust_ty> for $py_name {
            fn from(v: $rust_ty) -> Self {
                (&v).into()
            }
        }

        impl From<&$py_name> for $rust_ty {
            fn from(v: &$py_name) -> Self {
                match v {
                    $($py_name::$variant => <$rust_ty>::$variant,)*
                }
            }
        }

        impl From<$py_name> for $rust_ty {
            fn from(v: $py_name) -> Self {
                (&v).into()
            }
        }
    };
}
pub(crate) use py_enum;

/// Boilerplate for a pyclass that just wraps a `Shared<$rust_ty>` handle
/// (module = "griphos", unsendable, plus any extra bare pyclass flags you
/// pass, e.g. `subclass`). `#[pymethods]` is left for you to write by hand
/// right after invoking this.
macro_rules! py_wrapper {
    ($py_name:ident, $rust_ty:path $(, $flag:ident)*) => {
        #[pyo3_stub_gen::derive::gen_stub_pyclass]
        #[pyclass(module = "griphos", unsendable $(, $flag)*)]
        pub struct $py_name {
            pub(crate) inner: Shared<$rust_ty>,
        }

        impl From<Shared<$rust_ty>> for $py_name {
            fn from(inner: Shared<$rust_ty>) -> Self {
                $py_name { inner }
            }
        }

        impl From<$py_name> for Shared<$rust_ty> {
            fn from(v: $py_name) -> Self {
                v.inner
            }
        }
    };
}
pub(crate) use py_wrapper;

macro_rules! element_subclass {
    ($name:ident, $spec:ty) => {
        #[pyclass(extends=$crate::network::Element, unsendable)]
        pub struct $name {
            // Convenience reference to the parent elem for the purpose of rust code.
            elem: Shared<gr::Element>,
        }

        #[allow(unused)]
        impl $name {
            fn from_elem(elem: Shared<gr::Element>) -> Self {
                Self { elem }
            }

            fn into_initializer(self) -> PyClassInitializer<Self> {
                PyClassInitializer::from($crate::network::Element { inner: self.elem.clone() })
                    .add_subclass(self)
            }

            fn with_spec<R>(&self, f: impl FnOnce(&$spec) -> R) -> R {
                let elem = self.elem.borrow();
                let spec = elem.as_type::<$spec>().expect("type mismatch");
                f(spec)
            }

            fn with_spec_mut<R>(&self, f: impl FnOnce(&mut $spec) -> R) -> R {
                let mut elem = self.elem.borrow_mut();
                let spec = elem.as_type_mut::<$spec>().expect("type mismatch");
                f(spec)
            }

            fn uid(&self) -> String {
                self.elem.borrow().uid.clone()
            }
        }
    };
}
pub(crate) use element_subclass;
