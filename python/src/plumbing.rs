macro_rules! py_value {
    (
        $(#[$meta:meta])*
        $py_name:ident, $rust_ty:path {
            $( $(#[$fmeta:meta])* $vis:vis $field:ident: $field_ty:ty ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[pyo3_stub_gen::derive::gen_stub_pyclass]
        #[pyclass(module = "griphos", get_all, eq, frozen, from_py_object)]
        #[derive(Clone, PartialEq)]
        pub struct $py_name {
            $( $(#[$fmeta])* $vis $field: $field_ty, )*
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
    (
        $(#[$meta:meta])*
        $py_name:ident, $rust_ty:ty {
            $( $(#[$vmeta:meta])* $variant:ident ),* $(,)?
        }
    ) => {
        $(#[$meta])*
        #[pyo3_stub_gen::derive::gen_stub_pyclass_enum]
        #[pyclass(module = "griphos", eq, eq_int, from_py_object)]
        #[derive(Clone, Copy, PartialEq, Hash)]
        pub enum $py_name {
            $( $(#[$vmeta])* $variant, )*
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
/// right after invoking this. Doc comments placed before the class name are
/// forwarded to the generated struct (and so become the Python docstring).
macro_rules! py_wrapper {
    (
        $(#[$meta:meta])*
        $py_name:ident, $rust_ty:path $(, $flag:ident)*
    ) => {
        $(#[$meta])*
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

/// Like `py_wrapper!`, but for a Rust type that is itself a shared handle
/// (e.g. `gr::Element`), so no `Shared<..>` layer is wanted.
macro_rules! py_handle_wrapper {
    (
        $(#[$meta:meta])*
        $py_name:ident, $rust_ty:ty $(, $flag:ident)*
    ) => {
        $(#[$meta])*
        #[pyo3_stub_gen::derive::gen_stub_pyclass]
        #[pyclass(module = "griphos", unsendable $(, $flag)*)]
        pub struct $py_name {
            pub(crate) inner: $rust_ty,
        }

        impl From<$rust_ty> for $py_name {
            fn from(inner: $rust_ty) -> Self {
                $py_name { inner }
            }
        }

        impl From<$py_name> for $rust_ty {
            fn from(v: $py_name) -> Self {
                v.inner
            }
        }
    };
}
pub(crate) use py_handle_wrapper;

macro_rules! element_subclass {
    (
        $(#[$meta:meta])*
        $name:ident, $spec:ty
    ) => {
        $(#[$meta])*
        #[pyo3_stub_gen::derive::gen_stub_pyclass]
        #[pyclass(extends=$crate::network::Element, unsendable)]
        pub struct $name {
            elem: gr::Element<$spec>, // same Rc as the parent's handle
        }

        #[allow(unused)]
        impl $name {
            fn from_typed(elem: gr::Element<$spec>) -> Self { Self { elem } }

            fn try_from_elem(elem: &gr::Element) -> Option<Self> {
                elem.downcast::<$spec>().map(Self::from_typed)
            }

            fn into_initializer(self) -> PyClassInitializer<Self> {
                PyClassInitializer::from($crate::network::Element { inner: self.elem.erase() })
                    .add_subclass(self)
            }

            /// Build the Python object (parent and subclass together).
            fn into_py_object(self, py: Python<'_>) -> PyResult<Py<Self>> {
                Py::new(py, self.into_initializer())
            }

            fn elem(&self) -> &gr::Element<$spec> { &self.elem }
        }
    };
}
pub(crate) use element_subclass;
