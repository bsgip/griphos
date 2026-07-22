use std::io::{self, Read};
use pyo3::prelude::*;
use pyo3::types::{PyBytes, PyString};

/// Adapts a Python file-like object into something Rust's `Read` trait can consume.
pub(crate) struct PyFileReader<'py> {
    pub(crate) file: Bound<'py, PyAny>,
}

impl<'py> Read for PyFileReader<'py> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let result = self
            .file
            .call_method1("read", (buf.len(),))
            .map_err(|e| io::Error::other(e.to_string()))?;

        // Handle both binary-mode (`bytes`) and text-mode (`str`) files.
        let data: Vec<u8> = if let Ok(b) = result.cast::<PyBytes>() {
            b.as_bytes().to_vec()
        } else if let Ok(s) = result.cast::<PyString>() {
            s.to_string().into_bytes()
        } else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "file object's read() did not return str or bytes",
            ));
        };

        let n = data.len().min(buf.len());
        buf[..n].copy_from_slice(&data[..n]);
        Ok(n)
    }
}

#[macro_export]
macro_rules! py_struct {
    ($py_name:ident, $rust_ty:path { $($vis:vis $field:ident: $field_ty:ty),* $(,)? }) => {
        #[pyo3_stub_gen::derive::gen_stub_pyclass]
        #[pyclass(module = "griphos", get_all, set_all, from_py_object)]
        #[derive(Clone)]
        pub struct $py_name {
            $($vis $field: $field_ty,)*
        }

        impl From<$rust_ty> for $py_name {
            fn from(v: $rust_ty) -> Self {
                $py_name { $($field: v.$field.into(),)* }
            }
        }

        impl $py_name {
            pub fn into_inner(self) -> $rust_ty {
                $rust_ty { $($field: self.$field.into(),)* }
            }
        }
    };
}

#[macro_export]
macro_rules! py_enum {
    ($py_name:ident, $rust_ty:ty { $($variant:ident),* $(,)? }) => {
        #[pyo3_stub_gen::derive::gen_stub_pyclass_enum]
        #[pyclass(module = "griphos", eq, eq_int, from_py_object)]
        #[derive(Clone, PartialEq)]
        pub enum $py_name {
            $($variant,)*
        }

        impl From<$rust_ty> for $py_name {
            fn from(v: $rust_ty) -> Self {
                match v {
                    $(<$rust_ty>::$variant => $py_name::$variant,)*
                }
            }
        }

        impl From<$py_name> for $rust_ty {
            fn from(v: $py_name) -> Self {
                match v {
                    $($py_name::$variant => <$rust_ty>::$variant,)*
                }
            }
        }
    };
}
