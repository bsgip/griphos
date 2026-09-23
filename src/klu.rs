pub use log::debug;
pub use sprs::CsMatI;

use klu_sys::*;

use crate::foundation::PrettyPrint;

/// Mirrors KLU's c-struct for the symbolic info.
#[cfg(debug_assertions)]
#[repr(C)]
struct KluSymbolicFields {
    symmetry:        f64,
    est_flops:       f64,
    lnz:             f64,
    unz:             f64,
    lnz_blocks:      *mut f64,
    n:               i32,
    nz:              i32,
    p:               *mut i32,
    q:               *mut i32,
    r:               *mut i32,
    nzoff:           i32,
    nblocks:         i32,
    maxblock:        i32,
    ordering:        i32,
    do_btf:          i32,
    structural_rank: i32,
}

/// Mirrors KLU's c-struct for the numeric info.
#[cfg(debug_assertions)]
#[repr(C)]
struct KluNumericFields {
    n:              i32,
    nblocks:        i32,
    lnz:            i32,
    unz:            i32,
    max_lnz_block:  i32,
    max_unz_block:  i32,
    pnum:           *mut i32,
    pinv:           *mut i32,
    lip:            *mut i32,
    uip:            *mut i32,
    llen:           *mut i32,
    ulen:           *mut i32,
    lubx:           *mut *mut core::ffi::c_void,
    lusize:         *mut usize,
    udiag:          *mut core::ffi::c_void,
    rs:             *mut f64,
    worksize:       usize,
    work:           *mut core::ffi::c_void,
    xwork:          *mut core::ffi::c_void,
    iwork:          *mut i32,
    offp:           *mut i32,
    offi:           *mut i32,
    offx:           *mut core::ffi::c_void,
    nzoff:          i32,
}

/// Solves the linear system mx - b = 0 using KLU.
pub(crate) fn solve_klu(m: &mut CsMatI<f64, i32>, b: &mut Vec<f64>) -> Result<(), String> {
    let n: i32 = m.cols() as i32;

    unsafe {
        let mut common: KluCommon = std::mem::zeroed();
        klu_defaults(&mut common);

        let col_ptr = m.indptr().as_slice().unwrap().as_ptr() as *mut i32;
        let row_idx = m.indices().as_ptr() as *mut i32;

        debug!("KLU analyse");
        let mut symbolic = klu_analyze(n, col_ptr, row_idx, &mut common);
        if symbolic.is_null() {
            return Err(format!("klu_solve: klu_analyze failed with status {}", common.status));
        }

        let values = m.data_mut().as_mut_ptr();
        debug!("KLU factor");
        let mut numeric = klu_factor(col_ptr, row_idx, values, symbolic, &mut common);

        if numeric.is_null() {
            klu_free_symbolic(&mut symbolic, &mut common);
            return Err(format!("klu_solve: klu_factor failed with status {} singular_col {} matrix\n{}",
                common.status, common.singular_col, PrettyPrint(m, (None, None))));
        }

        debug!("KLU solve");
        let ok = klu_solve(symbolic, numeric, n, 1, b.as_mut_ptr(), &mut common);
        if ok == 0 {
            klu_free_symbolic(&mut symbolic, &mut common);
            klu_free_numeric(&mut numeric, &mut common);
            return Err(format!("klu_solve: failed with status {}", common.status));
        }

        #[cfg(debug_assertions)]
        if std::env::var("DEBUG_KLU").is_ok() {
            let sym_fields = &*(symbolic as *mut KluSymbolicFields);
            let num_fields = &*(numeric as *mut KluNumericFields);
            klu_flops(symbolic, numeric, &mut common);
            klu_rcond(symbolic, numeric, &mut common);
            klu_condest(col_ptr, values, symbolic, numeric, &mut common);
            println!(  "Symbolic:");
            println!(  "    n               = {}", sym_fields.n);
            println!(  "    nblocks         = {}", sym_fields.nblocks);
            println!(  "    structural_rank = {}", sym_fields.structural_rank);
            println!(  "    nz              = {}", sym_fields.nz);
            println!(  "    lnz             = {}", sym_fields.lnz);
            println!(  "    unz             = {}", sym_fields.unz);
            println!(  "    (lnz+unz)/nz    = {}", (sym_fields.lnz + sym_fields.unz)/(sym_fields.nz as f64));
            println!(  "Numeric:");
            println!(  "    lnz             = {}", num_fields.lnz);
            println!(  "    unz             = {}", num_fields.unz);
            println!(  "    lnz+unz         = {}", num_fields.lnz + num_fields.unz);
            println!(  "    max_lnz_block   = {}", num_fields.max_lnz_block);
            println!(  "    max_unz_block   = {}", num_fields.max_unz_block);
            println!(  "    nzoff           = {}", num_fields.nzoff);  // nnz outside BTF blocks
            println!(  "    scaling active  = {}", !num_fields.rs.is_null());
            println!(  "Common");
            println!(  "    flops           = {:.2e}", common.flops);
            println!(  "    rcond           = {:.2e}", common.rcond);
            println!(  "    condest         = {:.2e}", common.condest);
            println!(  "    status          = {}", common.status);
        }

        klu_free_symbolic(&mut symbolic, &mut common);
        klu_free_numeric(&mut numeric, &mut common);
    }
    Ok(())
}
