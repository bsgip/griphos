use std::rc::Rc;
use std::cell::{RefCell, Ref, RefMut};
// use std::sync::Arc;
// use parking_lot::{Mutex, MutexGuard};
use std::collections::HashMap;
use std::fmt;
use std::ops::{Add};
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

pub type Shared<T> = Rc<RefCell<T>>;
pub type Borrowed<'a, T> = Ref<'a, T>;
pub type BorrowedMut<'a, T> = RefMut<'a, T>;

pub trait SharedExt<T> {
    fn into_shared(self) -> Shared<T>;
}

impl<T> SharedExt<T> for T {
    fn into_shared(self) -> Shared<T> {
        Rc::new(RefCell::new(self))
    }
}

// -----------------------------------------------------------------------------
// Utility unwrap for map iterators
// -----------------------------------------------------------------------------
pub trait KvIteratorUnwrap<'a, K: 'a, V: 'a>: Iterator<Item = (&'a K, &'a Option<V>)> + Sized + 'a {
    fn kv_unwrapped(self) -> impl Iterator<Item = (&'a K, &'a V)> + 'a {
        self.map(|(k, v)| (k, v.as_ref().unwrap()))
    }
}

impl<'a, K: 'a, V: 'a, I> KvIteratorUnwrap<'a, K, V> for I
where
    I: Iterator<Item = (&'a K, &'a Option<V>)> + 'a,
{
}

// -----------------------------------------------------------------------------
// Serialization and Display
// -----------------------------------------------------------------------------

/// Trait for objects that are serializable via serde and displayable via the
/// associated JSON serialization.
pub trait GSerDisp {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer;
    fn g_fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result where Self: Sized {
        g_fmt_json(self, f)
    }
}

pub fn g_fmt_yaml<T>(x: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result where T: GSerDisp {
    let mut buf = Vec::new();
    let mut serializer = serde_yml::Serializer::new(&mut buf);
    x.g_serialize(&mut serializer).map_err(|_| fmt::Error)?;
    let yaml_str = String::from_utf8(buf).map_err(|_| fmt::Error)?;
    write!(f, "{yaml_str}")
}

pub fn g_fmt_json<T>(x: &T, f: &mut fmt::Formatter<'_>) -> fmt::Result where T: GSerDisp {
    let mut buf = Vec::new();
    let formatter = serde_json::ser::PrettyFormatter::with_indent(b"    "); // 4 spaces
    let mut serializer = serde_json::Serializer::with_formatter(&mut buf, formatter);
    x.g_serialize(&mut serializer).map_err(|_| fmt::Error)?;
    let json_str = String::from_utf8(buf).map_err(|_| fmt::Error)?;
    write!(f, "{json_str}")
}

#[macro_export]
macro_rules! impl_g_ser_disp {
    ($($t:ty),+) => {
        $(impl GSerDisp for $t {
            fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
                self.serialize(serializer)
            }
        })*
    }
}

#[macro_export]
macro_rules! impl_disp_from_wrap {
    ($($t:ty),+) => {
        $(impl fmt::Display for $t {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result where $t: GSerDisp {
                write!(f, "{}", Wrap(self))
            }
        })*
    }
}

#[macro_export]
macro_rules! impl_dbg_from_wrap {
    ($($t:ty),+) => {
        $(impl std::fmt::Debug for $t {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result where $t: GSerDisp {
                write!(f, "{}", Wrap(self))
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

impl_g_ser_disp!(i32, usize, f32, f64, bool, String);

pub struct Wrap<'a, T>(pub &'a T) where T: GSerDisp;

impl<'a, T> GSerDisp for Wrap<'a, T> where T: GSerDisp {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.0.g_serialize(serializer)
    }
}

impl<'a, T> Serialize for Wrap<'a, T> where T: GSerDisp {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.g_serialize(serializer)
    }
}

impl<'a, T> fmt::Display for Wrap<'a, T> where T: GSerDisp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result where Self: Sized {
        self.g_fmt(f)
    }
}

impl<T, U> GSerDisp for (T, U) where T: GSerDisp, U: GSerDisp {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut tup = serializer.serialize_tuple(2)?;
        tup.serialize_element(&Wrap(&self.0))?;
        tup.serialize_element(&Wrap(&self.1))?;
        tup.end()
    }
}

impl<T> GSerDisp for Shared<T> where T: GSerDisp {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.borrow().g_serialize(serializer)
    }
}

impl<T> GSerDisp for Vec<T> where T: GSerDisp {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.len()))?;
        for x in self {
            seq.serialize_element(&Wrap(x))?;
        }
        seq.end()
    }
}

impl<I, T> GSerDisp for IndexMap<I, T> where I: GSerDisp, T: GSerDisp {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(&Wrap(k), &Wrap(v))?;
        }
        map.end()
    }
}

impl<T> GSerDisp for HashMap<String, T> where T: GSerDisp {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut map = serializer.serialize_map(Some(self.len()))?;
        for (k, v) in self {
            map.serialize_entry(k, &Wrap(v))?;
        }
        map.end()
    }
}

impl<T> GSerDisp for Option<T> where T: GSerDisp {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        match self {
            Some(x) => x.g_serialize(serializer),
            None => serializer.serialize_none(),
        }
    }
}

impl<T> GSerDisp for CsMat<T> where T: GSerDisp {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.nnz()))?;
        for x in self.iter() {
            seq.serialize_element(&(x.1.0, x.1.1, Wrap(x.0)))?;
        }
        seq.end()
    }
}

impl<T> GSerDisp for CsVec<T> where T: GSerDisp {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let mut seq = serializer.serialize_seq(Some(self.nnz()))?;
        for (i, v) in self.iter() {
            seq.serialize_element(&(i, Wrap(v)))?;
        }
        seq.end()
    }
}

impl<T, U> GSerDisp for UnGraph<T, U> where T: Serialize, U: Serialize {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        self.serialize(serializer)
    }
}

pub fn serialize_with_wrap<'a, T, S>(x: &'a T, serializer: S) -> Result<S::Ok, S::Error>
where Wrap<'a, T>: GSerDisp, T: GSerDisp, S: Serializer {
    Wrap(x).g_serialize(serializer)
}

// -----------------------------------------------------------------------------
// Logging
// -----------------------------------------------------------------------------

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

pub struct PrettyPrint<'a, T, U = ()>(pub &'a T, pub U);

// -----------------------------------------------------------------------------
// linear algebra / sparse stuff
// -----------------------------------------------------------------------------

#[allow(non_camel_case_types)]
pub type c64 = na::Complex<f64>;

impl GSerDisp for c64 {
    fn g_serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: Serializer {
        let s = format!("{:}{:+}j", self.re, self.im);
        serializer.serialize_str(s.as_str())
    }
}

static COMPLEX_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^(?P<re>[+-]?[\d.]+(?:[eE][+-]?\d+)?)?(?P<im>[+-][\d.]+(?:[eE][+-]?\d+)?i)?$").unwrap()
});

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

#[macro_export]
macro_rules! p {
    ($mag:expr, $ang:expr) => {
        c64::from_polar($mag, $ang)
    };
}

pub type Mat<T> = na::DMatrix<T>;
pub type MatF64 = Mat<f64>;
pub type MatC64 = Mat<c64>;
pub type MatBool = Mat<bool>;

pub type Col<T> = na::DVector<T>;
pub type ColF64 = Col<f64>;
pub type ColC64 = Col<c64>;
pub type ColBool = Col<bool>;

pub type Row<T> = na::Matrix<T, na::U1, na::Dyn, na::VecStorage<T, na::U1, na::Dyn>>;
pub type RowF64 = Row<f64>;
pub type RowC64 = Row<c64>;
pub type RowBool = Row<bool>;

impl_g_ser_disp!(MatF64, MatC64, MatBool, ColF64, ColC64, ColBool, RowF64, RowC64, RowBool);

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

pub fn outer_sum<T>(mat: &CsMat<T>, i: usize) -> T where T: num_traits::Zero + Copy + std::ops::Add<Output = T> {
    mat.outer_view(i).unwrap().iter().fold(T::zero(), |acc, (_, &val)| acc + val)
}

impl<'a, T, I, IptrStorage, IndStorage, DataStorage> fmt::Display
    for PrettyPrint<'a, CsMatBase<T, I, IptrStorage, IndStorage, DataStorage>,
    (Option<&[String]>, Option<&[String]>)>
where
    T: std::fmt::Display + Copy,
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
// Ordering
// -----------------------------------------------------------------------------
pub fn ord_ij<T>(i: T, j: T) -> ((T, T), i32) where T: PartialOrd {
    if j >= i {((i, j), 1)} else {((j, i), -1)}
}

pub fn maybe_reverse<T>(ij: (T, T), orientation: i32) -> (T, T) {
    if orientation == 1 {(ij.0, ij.1)} else {(ij.1, ij.0)}
}

// -----------------------------------------------------------------------------
// Constants
// -----------------------------------------------------------------------------

pub const C_ZERO: c64 = c!(0.0, 0.0);
pub const C_I: c64 = c!(0.0, 1.0);
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
