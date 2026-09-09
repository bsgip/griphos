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
