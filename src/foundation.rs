//! Various foundational code that can be used throughout griphos.

use std::cell::{RefCell, Ref, RefMut};
// use std::sync::Arc;
// use parking_lot::{Mutex, MutexGuard};
use std::collections::HashMap;
use std::fmt;
use std::ops::Add;
use std::rc::Rc;
use std::sync::LazyLock;
use std::vec::Vec;

pub use indexmap::IndexMap;
extern crate nalgebra as na;
pub use nalgebra::ComplexField;
use petgraph::graph::UnGraph;
use regex::Regex;
pub use sprs::{CompressedStorage, CsMatBase, CsMat, CsMatI, CsVec, TriMat, TriMatI};
use serde::{Serialize, Serializer};
use serde::ser::{SerializeMap, SerializeSeq, SerializeTuple};

// -----------------------------------------------------------------------------
// Shared thread safe data
// -----------------------------------------------------------------------------

/// Shared data storage through Rc<RefCell<...>>
pub type Shared<T> = Rc<RefCell<T>>;
/// Alias for borrowed shared data.
pub type Borrowed<'a, T> = Ref<'a, T>;
/// Alias for mutably borrowed shared data.
pub type BorrowedMut<'a, T> = RefMut<'a, T>;

/// Trait to allow free conversion of a value into Shared.
pub trait IntoShared<T> {
    fn into_shared(self) -> Shared<T>;
}

impl<T> IntoShared<T> for T {
    /// Converts a value of type T into Shared\<T\>
    ///
    /// # Examples
    ///
    /// ```
    /// # use griphos::{Bus, IntoShared};
    /// let s: String = String::from("this is a string");
    /// let shared = s.into_shared();
    fn into_shared(self) -> Shared<T> {
        Rc::new(RefCell::new(self))
    }
}

// -----------------------------------------------------------------------------
// Serialization and Formatting
// -----------------------------------------------------------------------------

/// Trait for objects that are serializable via serde.
pub trait GSerialize {
    /// Serializes self with the given serde serializer.
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer;
}

/// Trait for objects that are formattable via serde.
pub trait GFormat {
    /// Formats self for display.
    fn g_format(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result where Self: Sized;
}

/// Blanket implementation of [`GFormat`] via YAML for any type that implements [`GSerialize`].
impl<T> GFormat for T where T: GSerialize {
    fn g_format(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result where Self: Sized {
        g_format_yaml(self, f)
    }
}

/// Formats a value using YAML.
pub fn g_format_yaml<T>(x: &T, f: &mut fmt::Formatter<'_>) -> std::fmt::Result where T: GSerialize {
    let mut buf = Vec::new();
    let mut serializer = serde_yml::Serializer::new(&mut buf);
    x.g_serialize(&mut serializer).map_err(|_| std::fmt::Error)?;
    let yaml_str = String::from_utf8(buf).map_err(|_| fmt::Error)?;
    write!(f, "{yaml_str}")
}

/// Format a value using JSON.
pub fn g_format_json<T>(x: &T, f: &mut fmt::Formatter<'_>) -> std::fmt::Result where T: GSerialize {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    "); // 4 spaces
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
    x.g_serialize(&mut serializer).map_err(|_| fmt::Error)?;
    let json_str = String::from_utf8(buf).map_err(|_| fmt::Error)?;
    write!(f, "{json_str}")
}

/// Implement GSerialize for a type, allowing it to be serialized using serde and displayed using its serialization.
#[macro_export]
macro_rules! impl_g_serialize {
    ($($t:ty),+) => {
        $(impl $crate::GSerialize for $t {
            fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
                self.serialize(serializer)
            }
        })*
    }
}

/// Implement Display for a type using g_format.
#[macro_export]
macro_rules! impl_disp_from_fmt {
    ($($t:ty),+) => {
        $(impl std::fmt::Display for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result where $t: GFormat {
                self.g_format(f)
            }
        })*
    }
}

/// Implement Debug for a type using g_format.
#[macro_export]
macro_rules! impl_dbg_from_fmt {
    ($($t:ty),+) => {
        $(impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result where $t: GFormat {
                write!(f, "{}", self)
            }
        })*
    }
}

#[macro_export]
macro_rules! impl_disp_from_dbg {
    ($($t:ty),+) => {
        $(impl std::fmt::Display for $t where $t: std::fmt::Debug {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                std::fmt::Debug::fmt(self, f)
            }
        })*
    }
}

impl_g_serialize!(i32, usize, f32, f64, bool, String);

/// A wrapper for (often) foreign types that we wish to use with GSerialize.
pub struct Wrap<'a, T>(pub &'a T);

/// Forwards serialization to the wrapped value's own [`GSerialize`] impl.
impl<'a, T> GSerialize for Wrap<'a, T> where T: GSerialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.0.g_serialize(serializer)
    }
}

/// Implement serde's Serialize for GSerialize wrapped values.
impl<'a, T> Serialize for Wrap<'a, T> where T: GSerialize {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.g_serialize(serializer)
    }
}

/// Implement Debug for GFormat wrapped values.
impl<'a, T> fmt::Debug for Wrap<'a, T> where T: GFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result where Self: Sized {
        self.0.g_format(f)
    }
}

/// Implement Display for GFormat wrapped values.
impl<'a, T> fmt::Display for Wrap<'a, T> where T: GFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result where Self: Sized {
        self.0.g_format(f)
    }
}

/// Implement GSerialize for tuples.
impl<T, U> GSerialize for (T, U) where T: GSerialize, U: GSerialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&Wrap(&self.0))?;
        tup.serialize_element(&Wrap(&self.1))?;
        tup.end()
    }
}

/// Implement GSerialize for `Shared` values.
impl<T> GSerialize for Shared<T> where T: GSerialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.borrow().g_serialize(serializer)
    }
}

/// Implement GSerialize for `Vec`s.
impl<T> GSerialize for Vec<T> where T: GSerialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for x in self {
            seq.serialize_element(&Wrap(x))?;
        }
        seq.end()
    }
}

/// Implement GSerialize for `IndexMap`s.
impl<I, T> GSerialize for IndexMap<I, T> where I: GSerialize, T: GSerialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(&Wrap(k), &Wrap(v))?;
        }
        map.end()
    }
}

/// Implement GSerialize for `HashMap`s.
impl<T> GSerialize for HashMap<String, T> where T: GSerialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(k, &Wrap(v))?;
        }
        map.end()
    }
}

/// Implement GSerialize for `Option`s.
impl<T> GSerialize for Option<T> where T: GSerialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            Some(x) => x.g_serialize(serializer),
            None => serializer.serialize_none(),
        }
    }
}

/// Implement GSerialize for `CsMat`s.
impl<T> GSerialize for CsMat<T> where T: GSerialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.nnz()))?;
        for x in self.iter() {
            seq.serialize_element(&(x.1.0, x.1.1, Wrap(x.0)))?;
        }
        seq.end()
    }
}

/// Implement GSerialize for `CsVec`s.
impl<T> GSerialize for CsVec<T> where T: GSerialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.nnz()))?;
        for (i, v) in self.iter() {
            seq.serialize_element(&(i, Wrap(v)))?;
        }
        seq.end()
    }
}

/// Implement GSerialize for `UnGraph`s.
impl<T, U> GSerialize for UnGraph<T, U> where T: Serialize, U: Serialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.serialize(serializer)
    }
}

/// Serialize a value by wrapping it in a Wrap.
pub fn serialize_with_wrap<'a, T, S>(x: &'a T, serializer: S) -> Result<S::Ok, S::Error>
where Wrap<'a, T>: GSerialize, T: GSerialize, S: Serializer {
    Wrap(x).g_serialize(serializer)
}

// -----------------------------------------------------------------------------
// Logging
// -----------------------------------------------------------------------------

/// Run this at the start of an executable to configure logging using the RUST_LOG environment variable.
pub fn configure_logging() {
    use std::io::Write;
    let _ = env_logger::Builder::new()
        .format(|buf, record| {
            let msg = record.args().to_string();
            writeln!(buf, "[{}] {}", record.level(), msg)?;
            Ok(())
        })
        .parse_env("RUST_LOG")
        .try_init();
}

// -----------------------------------------------------------------------------
// Pretty Printer
// -----------------------------------------------------------------------------

/// Pretty printing wrapper. T is the type to pretty print, and U is config.
pub struct PrettyPrint<'a, T, U = ()>(pub &'a T, pub U);

// -----------------------------------------------------------------------------
// linear algebra / sparse stuff
// -----------------------------------------------------------------------------

/// Complex type, alias of na::Complex\<c64\>
#[allow(non_camel_case_types)]
pub type c64 = na::Complex<f64>;

impl GSerialize for c64 {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let s = format!("{:}{:+}j", self.re, self.im);
        serializer.serialize_str(s.as_str())
    }
}

static COMPLEX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<re>[+-]?[\d.]+(?:[eE][+-]?\d+)?)?(?P<im>[+-][\d.]+(?:[eE][+-]?\d+)?i)?$").unwrap()
});

/// Creates a [`c64`] from a string.
pub fn parse_complex(s: &str) -> Result<c64, String> {
    let s = s.replace(" ", "");
    let caps = COMPLEX_RE.captures(&s)
        .ok_or_else(|| format!("cannot parse complex: '{}'", s))?;
    let re_part: f64 = caps.name("re")
        .map(|m| m.as_str().parse::<f64>().unwrap_or(0.0))
        .unwrap_or(0.0);
    let im_part: f64 = caps.name("im")
        .map(|m| m.as_str().trim_end_matches('i').parse::<f64>().unwrap_or(0.0))
        .unwrap_or(0.0);
    Ok(c64::new(re_part, im_part))
}

/// Macro to create a [`c64`] from real and imaginary parts.
#[macro_export]
macro_rules! c {
    ($re:expr, $im:expr) => {
        c64 {re:$re as f64, im:$im as f64}
    };
    ($re:expr) => {
        c64 {re:$re as f64, im:0.0}
    };
    ($s:literal) => {
        parse_complex($s).unwrap()
    };
}

/// Macro to create a [`c64`] from polar abs and arg.
#[macro_export]
macro_rules! p {
    ($mag:expr, $ang:expr) => {
        c64::from_polar($mag, $ang)
    };
}

/// Matrix of type T.
pub type Mat<T> = na::DMatrix<T>;
/// f64 matrix.
pub type MatF64 = Mat<f64>;
/// c64 matrix.
pub type MatC64 = Mat<c64>;
/// bool matrix.
pub type MatBool = Mat<bool>;

/// Column vector of type `T`.
pub type Col<T> = na::DVector<T>;
/// Column vector of `f64`.
pub type ColF64 = Col<f64>;
/// Column vector of `c64`.
pub type ColC64 = Col<c64>;
/// Column vector of `bool`.
pub type ColBool = Col<bool>;

/// Row vector of type `T`.
pub type Row<T> = na::Matrix<T, na::U1, na::Dyn, na::VecStorage<T, na::U1, na::Dyn>>;
/// Column vector of `f64`.
pub type RowF64 = Row<f64>;
/// Column vector of `c64`.
pub type RowC64 = Row<c64>;
/// Column vector of `bool`.
pub type RowBool = Row<bool>;

impl_g_serialize!(MatF64, MatC64, MatBool, ColF64, ColC64, ColBool, RowF64, RowC64, RowBool);

/// Trait to get a value at an index.
trait GetAt {
    fn get_at(&self, i: usize) -> Option<usize>;
}

impl GetAt for Vec<usize> {
    fn get_at(&self, index: usize) -> Option<usize> {
        self.get(index).cloned()
    }
}

impl GetAt for HashMap<usize, usize> {
    fn get_at(&self, index: usize) -> Option<usize> {
        self.get(&index).cloned()
    }
}

/// Given a sparse matrix, and a map from old to new indices, create another mapped sparse CSC matrix.
///
/// Any indices missing from the map will be omitted.
#[allow(private_bounds)]
pub fn map_spmat<T: Copy + Add<Output=T>, M: GetAt>(mat: &CsMat<T>, n: usize, map: &M) -> CsMat<T> {
    let mut tm = TriMat::<T>::new((n, n));
    for (v, (i, j)) in mat.iter() {
        if let Some(mi) = map.get_at(i) && let Some(mj) = map.get_at(j) {
            tm.add_triplet(mi, mj, *v);
        }
    }
    tm.to_csc()
}

/// Given a sparse vector, and a map from old to new indices, create another mapped sparse vector.
#[allow(private_bounds)]
pub fn map_spvec<T: Copy + Add<Output = T>, M: GetAt>(
    vec: &CsVec<T>, n: usize, map: &M
) -> CsVec<T> {
    let nnz = vec.nnz();
    let mut indices = Vec::with_capacity(nnz);
    let mut data = Vec::with_capacity(nnz);

    for (i, &v) in vec.iter() {
        if let Some(mi) = map.get_at(i) {
            indices.push(mi);
            data.push(v);
        }
    }

    let mut zipped: Vec<(usize, T)> = indices.into_iter().zip(data).collect();
    zipped.sort_by_key(|&(idx, _)| idx);
    let (indices, data): (Vec<usize>, Vec<T>) = zipped.into_iter().unzip();

    CsVec::new(n, indices, data)
}

/// Apply f to each element of column (CSC) or row (CSR) i
pub fn map_csmat_outer<T, F>(mat: &mut CsMat<T>, i: usize, f: F) where F: Fn(&mut T) {
    let col_range = mat.indptr().outer_inds_sz(i);
    for val in &mut mat.data_mut()[col_range] {
        f(val);
    }
}

/// Sum rows i (for a row major CSMat) or column i (for a column major CSMat).
pub fn outer_sum<T>(mat: &CsMat<T>, i: usize) -> T where T: num_traits::Zero + Copy + std::ops::Add<Output = T> {
    mat.outer_view(i).unwrap().iter().fold(T::zero(), |acc, (_, &val)| acc + val)
}

/// Pretty printing for `CsMat`s.
impl<'a, T, I, IptrStorage, IndStorage, DataStorage> fmt::Display
    for PrettyPrint<'a, CsMatBase<T, I, IptrStorage, IndStorage, DataStorage>,
    (Option<&[String]>, Option<&[String]>)>
where
    T: fmt::Display + Copy,
    I: sprs::SpIndex,
    IptrStorage: std::ops::Deref<Target = [I]>,
    IndStorage:  std::ops::Deref<Target = [I]>,
    DataStorage: std::ops::Deref<Target = [T]>,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mat = &self.0;

        let nrows = mat.rows();
        let ncols = if mat.nnz() == 0 {0} else {mat.cols()};

        let (row_names, col_names) = self.1;

        let mut row_names_stor: Vec<String> = vec![];
        if row_names.is_none() {
            row_names_stor = (0..nrows).map(|i| format!("{i}")).collect::<Vec<String>>();
        }
        let row_names_ref = row_names.unwrap_or(&row_names_stor);

        let mut col_names_stor: Vec<String> = vec![];
        if col_names.is_none() {
            col_names_stor = (0..ncols).map(|i| format!("{i}")).collect::<Vec<String>>();
        }
        let col_names_ref = col_names.unwrap_or(&col_names_stor);

        let cells: Vec<Vec<String>> = (0..nrows).map(|i| {
            (0..ncols).map(|j| {
                match mat.get(i, j) {
                    Some(v) => format!("{:.4}", v),
                    None    => "0".to_string(),
                }
            }).collect()
        }).collect();

        let col_width = cells.iter().flatten().map(|s| s.len()).max().unwrap_or(1).max(2);
        let sep_width = (col_width + 2) * ncols + 1;

        // Column headers
        let header = (0..ncols)
            .map(|j| format!("{:>width$}", col_names_ref[j], width = col_width))
            .collect::<Vec<_>>()
            .join("  ");
        writeln!(f, "#    {}", header)?;

        let sep = "-".repeat(sep_width + 24).to_string();

        // Top separator
        writeln!(f, "# {sep}")?;

        // Rows
        for (i, row) in cells.iter().enumerate() {
            let inner = row.iter()
                .map(|s| format!("{:>width$}", s, width = col_width))
                .collect::<Vec<_>>()
                .join("  ");

            let prefix = if i == 0         { "[" } else { " " };
            let suffix = if i == nrows - 1 { "]" } else { ";" };

            writeln!(f, "{}    {}    {} # {}", prefix, inner, suffix, row_names_ref[i])?;
        }

        // Bottom separator
        writeln!(f, "#{sep}")?;

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

/// Complex zero.
pub const C_ZERO: c64 = c!(0.0, 0.0);
/// Complex i.
pub const C_I: c64 = c!(0.0, 1.0);
/// Complex 1.
pub const C_ONE: c64 = c!(1.0, 0.0);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sparse_mat_stores_zero() {
        // new gives csr, new_csc gives csc
        // The following demonstrates that, e.g. below (n_rows, n_cols) = (2, 3) regardless of whether we're working
        // with CSC or CSR. Also that triplets doesn't get consumed.
        let mut triplets = TriMat::<f64>::from_triplets((2, 3), vec![0, 1], vec![1, 0], vec![0.1, 0.3]);

        let mat: CsMat<f64> = triplets.to_csr::<usize>();
        assert!(mat.shape() == (2, 3));
        assert!(mat.rows() == 2);
        assert!(mat.cols() == 3);
        assert!(*mat.get(0, 1).unwrap() == 0.1);
        assert!(*mat.get(1, 0).unwrap() == 0.3);

        let mat: CsMat<f64> = triplets.to_csc::<usize>();
        assert!(mat.shape() == (2, 3));
        assert!(mat.rows() == 2);
        assert!(mat.cols() == 3);
        assert!(*mat.get(0, 1).unwrap() == 0.1);
        assert!(*mat.get(1, 0).unwrap() == 0.3);

        assert!(mat.nnz() == 2);
        triplets.add_triplet(0, 0, 0.0);

        // We'd also like to know if sparse matrices store zeros or do they filter them out?
        // Turns out they store them. This is in fact useful for some purposes.
        let mat: CsMat<f64> = triplets.to_csc::<usize>();
        assert!(mat.nnz() == 3);
        assert!(*mat.get(0, 0).unwrap() == 0.0);
        assert!(*mat.get(0, 1).unwrap() == 0.1);
        assert!(*mat.get(1, 0).unwrap() == 0.3);

        // Also, are triplets additive? Turns out they are, also good.
        triplets.add_triplet(1, 1, 0.4);
        triplets.add_triplet(1, 1, 0.3);
        let mat: CsMat<f64> = triplets.to_csc::<usize>();
        assert!(mat.nnz() == 4);
        assert!(*mat.get(0, 0).unwrap() == 0.0);
        assert!(*mat.get(0, 1).unwrap() == 0.1);
        assert!(*mat.get(1, 0).unwrap() == 0.3);
        assert!(*mat.get(1, 1).unwrap() == 0.7);
        println!("SP:\n{}", mat.to_dense());

        // Also check that we can tell if an element is set or not.
        assert!(mat.get(1, 2).is_none());
    }

    #[test]
    fn test_serialize_c64() {
        let z = c!(1.0, 2.0);
        println!("{}", &Wrap(&z));
    }

    #[test]
    fn test_serialize_vec_c64() {
        let v = vec![c!(1.0, 2.0); 10];
        println!("{}", &Wrap(&v));
    }

    #[test]
    fn test_serialize_vec_f64() {
        let v = vec![3.0, 4.0];
        println!("{}", &Wrap(&v));
    }
}
