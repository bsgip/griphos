use std::collections::HashMap;
use std::time::Instant;

use log::{debug, info};

use crate::foundation::*;
use crate::klu::solve_klu;
use crate::network::*;
use crate::{NetworkSolver, SolverPlugin, SolverStatus, SolverStatusFlags, SolverStatusItem};

fn name_maybe_idx(idx: Option<usize>) -> String {
    match idx {Some(i) => format!("{i}"), None => "*".to_string()}
}

fn name_vr(idx: Option<usize>) -> String {
    format!("vr_{}", name_maybe_idx(idx))
}

fn name_vi(idx: Option<usize>) -> String {
    format!("vi_{}", name_maybe_idx(idx))
}

fn name_pg_nd(idx: Option<usize>) -> String {
    format!("pgn_{}", name_maybe_idx(idx))
}

fn name_qg_nd(idx: Option<usize>) -> String {
    format!("qgn_{}", name_maybe_idx(idx))
}

fn name_pg_br(idx: Option<usize>) -> String {
    format!("pgb_{}", name_maybe_idx(idx))
}

fn name_qg_br(idx: Option<usize>) -> String {
    format!("qgb_{}", name_maybe_idx(idx))
}

fn name_eqn_kcl_re(idx: Option<usize>) -> String {
    format!("kclr_{}", name_maybe_idx(idx))
}

fn name_eqn_kcl_im(idx: Option<usize>) -> String {
    format!("kcli_{}", name_maybe_idx(idx))
}

fn name_eqn_vm_c_nd(idx: Option<usize>) -> String {
    format!("vmcn_{}", name_maybe_idx(idx))
}

fn name_eqn_vr_c_br(idx: Option<usize>) -> String {
    format!("vrcb_{}", name_maybe_idx(idx))
}

fn name_eqn_vi_c_br(idx: Option<usize>) -> String {
    format!("vicb_{}", name_maybe_idx(idx))
}

fn name_eqn_vm_c_br(idx: Option<usize>) -> String {
    format!("vmcb_{}", name_maybe_idx(idx))
}

/// Vc stands for Variable or Constant or None
#[derive(Clone, Debug)]
pub enum Vc {
    Var {i: usize},
    Const {x: f64},
}

impl Default for Vc {
    fn default() -> Self {
        Vc::Const {x: 0.0}
    }
}

impl Vc {
    fn val(&self, vars: &[f64]) -> f64 {
        match self {
            Vc::Var {i, ..} => vars[*i],
            Vc::Const {x} => *x,
        }
    }
}

/// Voltage control type - either voltage controlled slack, voltage controlled PV, PQ (uncontrolled) or grounded.
pub enum CtrlType {
    Sl,
    Pv,
    Pq,
    Gr,
}

/// Additional info all nodes.
#[derive(Clone, Debug)]
pub struct NodeInfo {
    vr: Vc,
    vi: Vc,
    ieqn_kcl_re: Option<usize>,
    ieqn_kcl_im: Option<usize>,
}

/// Additional info for PV or SL generators at branches.
#[derive(Clone, Debug, Default)]
pub struct BranchGenInfo {
    pg: Vc,
    qg: Vc,
    ieqn_vr_c: Option<usize>,
    ieqn_vi_c: Option<usize>,
    ieqn_vm_c: Option<usize>,
}

/// Additional info for PV generators at nodes.
#[derive(Clone, Debug, Default)]
pub struct NodeGenInfo {
    qg: Vc,
    ieqn_vm_c: Option<usize>,
}

/// Enum for different kinds of variables.
pub enum VarKind {
    Vr {nd_idx: usize},
    Vi {nd_idx: usize},
    PgNd {nd_idx: usize},
    QgNd {nd_idx: usize},
    PgBr {br_idx: usize},
    QgBr {br_idx: usize},
}

/// Variable information.
pub struct VarInfo {
    kind: VarKind,
    x0: f64,
}

impl VarInfo {
    fn name(&self) -> String {
        match &self.kind {
            VarKind::Vr {nd_idx} => {name_vr(Some(*nd_idx))},
            VarKind::Vi {nd_idx} => {name_vi(Some(*nd_idx))},
            VarKind::PgNd {nd_idx} => {name_pg_nd(Some(*nd_idx))},
            VarKind::QgNd {nd_idx} => {name_qg_nd(Some(*nd_idx))},
            VarKind::PgBr {br_idx} => {name_pg_br(Some(*br_idx))},
            VarKind::QgBr {br_idx} => {name_qg_br(Some(*br_idx))},
        }
    }
}

/// Enum for different kinds of equations.
pub enum EqnKind {
    KclRe {nd_idx: usize},
    KclIm {nd_idx: usize},
    VmConstNd {nd_idx: usize, vm: f64},
    VrConstBr {br_idx: usize, vr: f64},
    ViConstBr {br_idx: usize, vi: f64},
    VmConstBr {br_idx: usize, vm: f64},
}

impl EqnKind {
    fn name(&self) -> String {
        match self {
            EqnKind::KclRe {nd_idx} => {name_eqn_kcl_re(Some(*nd_idx))},
            EqnKind::KclIm {nd_idx} => {name_eqn_kcl_im(Some(*nd_idx))},
            EqnKind::VmConstNd {nd_idx, ..} => {name_eqn_vm_c_nd(Some(*nd_idx))},
            EqnKind::VrConstBr {br_idx, ..} => {name_eqn_vr_c_br(Some(*br_idx))},
            EqnKind::ViConstBr {br_idx, ..} => {name_eqn_vi_c_br(Some(*br_idx))},
            EqnKind::VmConstBr {br_idx, ..} => {name_eqn_vm_c_br(Some(*br_idx))},
        }
    }
}

/// Solver parameters.
pub struct Params {
    pub max_iter: u32,
    pub tol: f64,
    pub debug_mode: bool,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            max_iter: 50,
            tol: 1e-6,
            debug_mode: false,
        }
    }
}

/// Rectangular nodal solver.
pub struct NodalRectSolver {
    params: Params,

    pub nmod: NetworkModel,

    pub node_infos: Vec<NodeInfo>,

    pub node_gen_infos: HashMap<usize, NodeGenInfo>,
    pub branch_gen_infos: HashMap<usize, BranchGenInfo>,

    pub var_infos: Vec<VarInfo>,
    pub eqn_kinds: Vec<EqnKind>,

    pub vars: Vec<f64>,
    pub rhs: Vec<f64>,
    pub jac_csc: CsMatI<f64, i32>, // i32 indexing for compatibility with KLU.
    pub jac_csc_idxs: Vec<usize>,
}

impl NodalRectSolver {
    pub fn new(nmod: NetworkModel) -> NodalRectSolver {
        let mut solver = NodalRectSolver {
            params: Params::default(),
            nmod,
            node_infos: vec![],
            node_gen_infos: HashMap::new(),
            branch_gen_infos: HashMap::new(),
            var_infos: vec![],
            eqn_kinds: vec![],
            vars: vec![],
            rhs: vec![],
            jac_csc: CsMatI::empty(CompressedStorage::CSC, 0),
            jac_csc_idxs: vec![],
        };

        solver.init();
        solver
    }

    pub fn ndim(&self) -> usize {
        self.vars.len()
    }

    pub fn init(&mut self) {
        if self.nmod.nodes.is_empty() {return;}

        let mut nd_gen_info: Vec<[Option<f64>; 2]> = vec![[None, None]; self.nmod.nodes.len()];
        let mut br_gen_info: Vec<[Option<f64>; 2]> = vec![[None, None]; self.nmod.branches.len()];

        let mut f = |v: &CsVec<f64>, i: usize| {
            for (m_br, &x) in v.iter() {
                let br = &self.nmod.branches[m_br];
                let nd0 = &self.nmod.nodes[br.nd_0_idx];
                let nd1 = &self.nmod.nodes[br.nd_1_idx];
                match (nd0.is_grounded, nd1.is_grounded) {
                    (false, false) => {
                        br_gen_info[m_br][i] = Some(x);
                    },
                    (false, true) => {
                        nd_gen_info[br.nd_0_idx][i] = Some(x);
                    },
                    (true, false) => {
                        nd_gen_info[br.nd_1_idx][i] = Some(-x);
                    },
                    (true, true) => {
                        panic!("Voltage controlled branch with both nodes grounded.");
                    }
                }
            }
        };
        f(&self.nmod.vmag_const, 0);
        f(&self.nmod.vang_const, 1);

        let n_ground = self.nmod.nodes.iter().filter(|nd| nd.is_grounded).count();
        let n_nd_sl = nd_gen_info.iter().filter(|[vmc, vac]| vmc.is_some() && vac.is_some()).count();
        let n_nd_pv = nd_gen_info.iter().filter(|[vmc, vac]| vmc.is_some() && vac.is_none()).count();
        let n_br_sl = br_gen_info.iter().filter(|[vmc, vac]| vmc.is_some() && vac.is_some()).count();
        let n_br_pv = br_gen_info.iter().filter(|[vmc, vac]| vmc.is_some() && vac.is_none()).count();
        let n_not_const = self.nmod.nodes.len() - n_ground - n_nd_sl;
        let ndim = 2 * n_not_const + n_nd_pv + 2 * n_br_sl + n_br_pv;

        debug!("Nodal Solver");
        debug!("n_ground = {n_ground}");
        debug!("nd_nd_sl = {n_nd_sl}");
        debug!("nd_nd_pv = {n_nd_pv}");
        debug!("nd_br_sl = {n_br_sl}");
        debug!("nd_br_pv = {n_br_pv}");
        debug!("n_not_const = {n_not_const}");
        debug!("ndim = {ndim}");

        self.node_infos.clear(); self.node_infos.reserve(self.nmod.nodes.len());

        self.node_gen_infos.clear();
        self.branch_gen_infos.clear();

        self.var_infos.clear(); self.var_infos.reserve(ndim);
        self.eqn_kinds.clear(); self.eqn_kinds.reserve(ndim);

        let mut idim: usize = 0;

        for (i_nd, (nd, ngi)) in self.nmod.nodes.iter().zip(nd_gen_info.iter()).enumerate() {
            if nd.is_grounded {
                // Ground node
                self.node_infos.push(
                    NodeInfo {vr: Vc::Const {x: 0.0}, vi: Vc::Const {x: 0.0}, ieqn_kcl_re: None, ieqn_kcl_im: None}
                );
            }
            else {
                match ngi {
                    [Some(vmc), Some(vac)] => {
                        // Slack node
                        let c64 {re: vrc, im: vic} = p!(*vmc, *vac);
                        self.node_infos.push(
                            NodeInfo {
                                vr: Vc::Const {x: vrc},
                                vi: Vc::Const {x: vic},
                                ieqn_kcl_re: None,
                                ieqn_kcl_im: None,
                            }
                        );
                    },
                    [Some(vmc), None] => {
                        // PV node
                        let x0 = vmc * nd.v_nom / nd.v_nom.abs();
                        self.node_infos.push(
                            NodeInfo {
                                vr: Vc::Var {i: idim},
                                vi: Vc::Var {i: idim + 1},
                                ieqn_kcl_re: Some(idim),
                                ieqn_kcl_im: Some(idim + 1)
                            }
                        );

                        self.node_gen_infos.insert(
                            i_nd,
                            NodeGenInfo {qg: Vc::Var {i: idim + 2}, ieqn_vm_c: Some(idim + 2)}
                        );
                        // TODO

                        self.var_infos.push(VarInfo {kind:VarKind::Vr {nd_idx: i_nd}, x0: x0.re});
                        self.var_infos.push(VarInfo {kind: VarKind::Vi {nd_idx: i_nd}, x0: x0.im});
                        self.var_infos.push(VarInfo {kind: VarKind::QgNd {nd_idx: i_nd}, x0: 0.0});

                        self.eqn_kinds.push(EqnKind::KclRe {nd_idx: i_nd});
                        self.eqn_kinds.push(EqnKind::KclIm {nd_idx: i_nd});
                        self.eqn_kinds.push(EqnKind::VmConstNd {nd_idx: i_nd, vm: *vmc});

                        idim += 3;
                    },
                    [None, None] => {
                        // PQ Node
                        self.node_infos.push(
                            NodeInfo {
                                vr: Vc::Var {i: idim},
                                vi: Vc::Var {i: idim + 1},
                                ieqn_kcl_re: Some(idim),
                                ieqn_kcl_im: Some(idim + 1),
                            }
                        );

                        self.var_infos.push(VarInfo {kind: VarKind::Vr {nd_idx: i_nd}, x0: nd.v_nom.re});
                        self.var_infos.push(VarInfo {kind: VarKind::Vi {nd_idx: i_nd}, x0: nd.v_nom.im});

                        self.eqn_kinds.push(EqnKind::KclRe {nd_idx: i_nd});
                        self.eqn_kinds.push(EqnKind::KclIm {nd_idx: i_nd});

                        idim += 2;
                    },
                    _ => panic!("Unexpected ctrl type")
                }
            }
        }

        for (m_br, bgi) in br_gen_info.iter().enumerate() {
            match bgi {
                [Some(vmc), Some(vac)] => {
                    // Slack branch
                    let c64 {re: vr, im: vi} = p!(*vmc, *vac);
                    self.branch_gen_infos.insert(
                        m_br,
                        BranchGenInfo {
                            pg: Vc::Var {i: idim},
                            qg: Vc::Var {i: idim + 1},
                            ieqn_vr_c: Some(idim),
                            ieqn_vi_c: Some(idim + 1),
                            ..Default::default()
                        }
                    );

                    self.var_infos.push(VarInfo {kind: VarKind::PgBr {br_idx: m_br}, x0: 0.0});
                    self.var_infos.push(VarInfo {kind: VarKind::QgBr {br_idx: m_br}, x0: 0.0});

                    self.eqn_kinds.push(EqnKind::VrConstBr {br_idx: m_br, vr});
                    self.eqn_kinds.push(EqnKind::ViConstBr {br_idx: m_br, vi});

                    idim += 2;
                },
                [Some(vmc), None] => {
                    // PV branch
                    self.branch_gen_infos.insert(
                        m_br,
                        BranchGenInfo {
                            qg: Vc::Var{i: idim},
                            ieqn_vm_c: Some(idim),
                            ..Default::default()
                        }
                    );

                    self.var_infos.push(VarInfo {kind: VarKind::QgBr {br_idx: m_br}, x0: 0.0});

                    self.eqn_kinds.push(EqnKind::VmConstBr {br_idx: m_br, vm: *vmc});

                    idim += 1;
                },
                _ => {}
            }
        }

        self.vars = vec![0.0; ndim];
        self.rhs = vec![0.0; ndim];
        self.jac_csc = CsMatI::empty(CompressedStorage::CSC, 0);
        self.jac_csc_idxs.clear();

        assert!(idim == ndim);
        assert!(self.node_infos.len() == self.nmod.nodes.len());
        assert!(self.node_gen_infos.len() == n_nd_pv);
        assert!(self.branch_gen_infos.len() == n_br_sl + n_br_pv);
        assert!(self.var_infos.len() == ndim);
        assert!(self.eqn_kinds.len() == ndim);
    }

    fn init_vars(&mut self) {
        debug!("init_vars");

        for (var, var_info) in self.vars.iter_mut().zip(self.var_infos.iter()) {
            *var = var_info.x0;
        }
    }

    fn prepare_for_iter(&mut self) {
        debug!("prepare_for_iter");

        let ndim = self.ndim();

        // Initialize rhs to 0.0
        for x in self.rhs.iter_mut() {
            *x = 0.0
        }

        enum JacTarget {
            Tri {
                tri: TriMatI<f64, i32>,
                order: Vec<(usize, usize)>,
            },
            Existing {
                pos: usize,
            },
        }

        impl JacTarget {
            fn add(&mut self, jac_csc: &mut CsMatI<f64, i32>, ordered_indices: &[usize], i: usize, j: usize, x: f64) {
                match self {
                    JacTarget::Tri { tri, order } => {
                        tri.add_triplet(i, j, x);
                        order.push((i, j));
                    }
                    JacTarget::Existing { pos } => {
                        let idx = ordered_indices[*pos];
                        jac_csc.data_mut()[idx] += x;
                        *pos += 1;
                    }
                }
            }

            fn finalize(self, jac_csc: &mut CsMatI<f64, i32>, ordered_indices: &mut Vec<usize>) {
                match self {
                    JacTarget::Tri { tri, order } => {
                        *jac_csc = tri.to_csc();
                        ordered_indices.clear();
                        ordered_indices.reserve(order.len());
                        for (i, j) in order {
                            let idx = jac_csc
                                .nnz_index(i, j)
                                .expect("triplet (i,j) must exist in the finalized pattern")
                            .0;
                            ordered_indices.push(idx);
                        }
                    }
                    JacTarget::Existing { pos } => {
                        debug_assert_eq!(
                            pos,
                            ordered_indices.len(),
                            "call count changed between iterations — ordered_indices assumes a stable call order"
                        );
                    }
                }
            }
        }

        let mut jac_target = if self.jac_csc.nnz() == 0 {
            JacTarget::Tri { tri: TriMatI::new((ndim, ndim)), order: vec![] }
        } else {
            for x in self.jac_csc.data_mut() {
                *x = 0.0;
            }
            JacTarget::Existing { pos: 0 }
        };

        for (&c64 {re: gmn, im: bmn}, (m, n)) in self.nmod.y_const.iter() {
            let br_m = &self.nmod.branches[m];
            let nd_info_im0 = &self.node_infos[br_m.nd_0_idx];
            let nd_info_im1 = &self.node_infos[br_m.nd_1_idx];

            let br_n = &self.nmod.branches[n];
            let nd_info_in0 = &self.node_infos[br_n.nd_0_idx];
            let nd_info_in1 = &self.node_infos[br_n.nd_1_idx];

            let v_n_re = nd_info_in0.vr.val(&self.vars) - nd_info_in1.vr.val(&self.vars);
            let v_n_im = nd_info_in0.vi.val(&self.vars) - nd_info_in1.vi.val(&self.vars);

            for (nd_info_im, sgn_im) in [(&nd_info_im0, 1.0), (&nd_info_im1, -1.0)] {
                if let &Some(ieqn) = &nd_info_im.ieqn_kcl_re {
                    self.rhs[ieqn] += sgn_im * (gmn * v_n_re - bmn * v_n_im);

                    for (nd_info_in, sgn_n) in [(&nd_info_in0, 1.0), (&nd_info_in1, -1.0)] {
                        let sgn = sgn_im * sgn_n;
                        if let &Vc::Var {i: ivar} = &nd_info_in.vr {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * gmn);
                        }
                        if let &Vc::Var {i: ivar} = &nd_info_in.vi {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -sgn * bmn);
                        }
                    }
                }

                if let Some(ieqn) = nd_info_im.ieqn_kcl_im {
                    self.rhs[ieqn] += sgn_im * (gmn * v_n_im + bmn * v_n_re);

                    for (nd_info_in, sgn_n) in [(&nd_info_in0, 1.0), (&nd_info_in1, -1.0)] {
                        let sgn = sgn_im * sgn_n;
                        if let &Vc::Var {i: ivar} = &nd_info_in.vr {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * bmn);
                        }
                        if let &Vc::Var {i: ivar} = &nd_info_in.vi {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * gmn);
                        }
                    }
                }
            }
        }

        // Terms involving i_const in KCL
        for (m, c64 {re: cm, im: dm}) in self.nmod.i_const.iter() {
            let br_m = &self.nmod.branches[m];
            let nd_info_im0 = &self.node_infos[br_m.nd_0_idx];
            let nd_info_im1 = &self.node_infos[br_m.nd_1_idx];

            let v_m_re = nd_info_im0.vr.val(&self.vars) - nd_info_im1.vr.val(&self.vars);
            let v_m_im = nd_info_im0.vi.val(&self.vars) - nd_info_im1.vi.val(&self.vars);
            let v_m_mag_2 = v_m_re.powi(2) + v_m_im.powi(2);
            let v_m_mag = v_m_mag_2.sqrt();
            let v_m_mag_n3 = v_m_mag.powi(-3);

            let add_kclr0 = (cm * v_m_re + dm * v_m_im) / v_m_mag;
            let add_kcli0 = (-dm * v_m_re + cm * v_m_im) / v_m_mag;
            let temp = v_m_mag_n3 * (cm * v_m_im - dm * v_m_re);
            let d_kclr0_dvr0 = v_m_im * temp;
            let d_kclr0_dvi0 = -v_m_re * temp;
            let temp = v_m_mag_n3 * (dm * v_m_im + cm * v_m_re);
            let d_kcli0_dvr0 = -v_m_im * temp;
            let d_kcli0_dvi0 = v_m_re * temp;

            for (nd_info_ima, sgn_ima) in [(&nd_info_im0, 1.0), (&nd_info_im1, -1.0)] {
                if let Some(ieqn) = nd_info_ima.ieqn_kcl_re {
                    self.rhs[ieqn] += sgn_ima * add_kclr0;
                    for (nd_info_imb, sgn_imb) in [(&nd_info_im0, 1.0), (&nd_info_im1, -1.0)] {
                        let sgn = sgn_ima * sgn_imb;

                        if let &Vc::Var {i: ivar} = &nd_info_imb.vr {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * d_kclr0_dvr0);
                        }

                        if let &Vc::Var {i: ivar} = &nd_info_imb.vi {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * d_kclr0_dvi0);
                        }
                    }
                }
                if let Some(ieqn) = nd_info_im0.ieqn_kcl_im {
                    self.rhs[ieqn] += sgn_ima * add_kcli0;
                    for (nd_info_imb, sgn_imb) in [(&nd_info_im0, 1.0), (&nd_info_im1, -1.0)] {
                        let sgn = sgn_ima * sgn_imb;

                        if let &Vc::Var {i: ivar} = &nd_info_imb.vr {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * d_kcli0_dvr0);
                        }

                        if let &Vc::Var {i: ivar} = &nd_info_imb.vi {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * d_kcli0_dvi0);
                        }

                    }
                }
            }
        }

        // Terms involving s_const in KCL
        for (m, c64 {re: pm, im: qm}) in self.nmod.s_const.iter() {
            let br_m = &self.nmod.branches[m];
            let nd_info_im0 = &self.node_infos[br_m.nd_0_idx];
            let nd_info_im1 = &self.node_infos[br_m.nd_1_idx];

            let v_m_re = nd_info_im0.vr.val(&self.vars) - nd_info_im1.vr.val(&self.vars);
            let v_m_im = nd_info_im0.vi.val(&self.vars) - nd_info_im1.vi.val(&self.vars);
            let v_m_mag_2 = v_m_re.powi(2) + v_m_im.powi(2);
            let v_m_mag_n4 = v_m_mag_2.powi(-2);

            let add_kclr0 = (pm * v_m_re + qm * v_m_im) / v_m_mag_2;
            let add_kcli0 = (pm * v_m_im - qm * v_m_re) / v_m_mag_2;
            let d_kclr0_dvr0 = (-pm * (v_m_re.powi(2) - v_m_im.powi(2)) - 2.0 * qm * v_m_im * v_m_re) * v_m_mag_n4;
            let d_kclr0_dvi0 = (qm * (v_m_re.powi(2) - v_m_im.powi(2)) - 2.0 * pm * v_m_im * v_m_re) * v_m_mag_n4;
            let d_kcli0_dvr0 = d_kclr0_dvi0;
            let d_kcli0_dvi0 = -d_kclr0_dvr0;

            for (nd_info_ima, sgn_ima) in [(&nd_info_im0, 1.0), (&nd_info_im1, -1.0)] {
                if let Some(ieqn) = nd_info_ima.ieqn_kcl_re {
                    self.rhs[ieqn] += sgn_ima * add_kclr0;

                    for (nd_info_imb, sgn_imb) in [(&nd_info_im0, 1.0), (&nd_info_im1, -1.0)] {
                        let sgn = sgn_ima * sgn_imb;

                        if let &Vc::Var {i: ivar} = &nd_info_imb.vr {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * d_kclr0_dvr0);
                        }

                        if let &Vc::Var {i: ivar} = &nd_info_imb.vi {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * d_kclr0_dvi0);
                        }
                    }
                }

                if let Some(ieqn) = nd_info_ima.ieqn_kcl_im {
                    self.rhs[ieqn] += sgn_ima * add_kcli0;

                    for (nd_info_imb, sgn_imb) in [(&nd_info_im0, 1.0), (&nd_info_im1, -1.0)] {
                        let sgn = sgn_ima * sgn_imb;

                        if let &Vc::Var {i: ivar} = &nd_info_imb.vr {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * d_kcli0_dvr0);
                        }

                        if let &Vc::Var {i: ivar} = &nd_info_imb.vi {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * d_kcli0_dvi0);
                        }
                    }
                }

            }
        }

        // Terms involving node pg and qg in KCL; constant voltage magnitude for nodes.
        for (&i, ngi_i) in self.node_gen_infos.iter() {
            let nd_info_i = &self.node_infos[i];
            let vr_i = nd_info_i.vr.val(&self.vars);
            let vi_i = nd_info_i.vi.val(&self.vars);
            let vr_i_2 = vr_i.powi(2);
            let vi_i_2 = vi_i.powi(2);
            let vm_i_2 = vr_i_2 + vi_i_2;
            let v2_i_diff = vr_i_2 - vi_i_2;
            let vm_i_n4 = vm_i_2.powi(-2);

            let qg = ngi_i.qg.val(&self.vars);

            if let Some(ieqn) = nd_info_i.ieqn_kcl_re {
                self.rhs[ieqn] += -qg * vi_i / vm_i_2;

                if let Vc::Var{i: ivar} = ngi_i.qg {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -vi_i / vm_i_2);
                }

                if let Vc::Var {i: ivar} = nd_info_i.vr {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, 2.0 * qg * vi_i * vr_i * vm_i_n4);
                }

                if let Vc::Var {i: ivar} = nd_info_i.vi {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -qg * v2_i_diff * vm_i_n4);
                }

            }

            if let Some(ieqn) = nd_info_i.ieqn_kcl_im {
                self.rhs[ieqn] += qg * vr_i / vm_i_2;

                if let Vc::Var{i: ivar} = ngi_i.qg {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, vr_i / vm_i_2);
                }

                if let Vc::Var {i: ivar} = nd_info_i.vr {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -qg * v2_i_diff * vm_i_n4);
                }

                if let Vc::Var {i: ivar} = nd_info_i.vi {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -2.0 * qg * vi_i * vr_i * vm_i_n4);
                }
            }

            if let Some(ieqn) = ngi_i.ieqn_vm_c && let EqnKind::VmConstNd{nd_idx: _, vm} = self.eqn_kinds[ieqn] {
                self.rhs[ieqn] = vr_i.powi(2) + vi_i.powi(2) - vm.powi(2);

                if let Vc::Var {i: ivar} = nd_info_i.vr {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, 2.0 * vr_i);
                }

                if let Vc::Var {i: ivar} = nd_info_i.vi {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, 2.0 * vi_i);
                }
            }
        }

        // Terms involving branch pg and qg in KCL
        for (&m, bgi_m) in self.branch_gen_infos.iter() {
            let br_m = &self.nmod.branches[m];
            let nd_info_im0 = &self.node_infos[br_m.nd_0_idx];
            let nd_info_im1 = &self.node_infos[br_m.nd_0_idx];
            let vr_m = nd_info_im0.vr.val(&self.vars) - nd_info_im1.vr.val(&self.vars);
            let vi_m = nd_info_im0.vr.val(&self.vars) - nd_info_im1.vr.val(&self.vars);
            let vr_m_2 = vr_m.powi(2);
            let vi_m_2 = vi_m.powi(2);
            let vm_m_2 = vr_m_2 + vi_m_2;
            let vm_m_n4 = vm_m_2.powi(-2);
            let v2_m_diff = vr_m_2 - vi_m_2;

            let pg_m = bgi_m.pg.val(&self.vars);
            let qg_m = bgi_m.qg.val(&self.vars);

            for (nd_info_ima, sgn_ima) in [(&nd_info_im0, 1.0), (&nd_info_im1, -1.0)] {
                if let Some(ieqn) = nd_info_ima.ieqn_kcl_re {
                    self.rhs[ieqn] += sgn_ima * (-pg_m * vr_m - qg_m * vi_m) / vm_m_2;

                    for (nd_info_imb, sgn_imb) in [(&nd_info_im0, 1.0), (&nd_info_im1, -1.0)] {
                        let sgn = sgn_ima * sgn_imb;
                        if let Vc::Var{i: ivar} = bgi_m.pg {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -sgn * vr_m / vm_m_2);
                        }

                        if let Vc::Var{i: ivar} = bgi_m.qg {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -sgn * vi_m / vm_m_2);
                        }

                        if let Vc::Var {i: ivar} = nd_info_imb.vr {
                            jac_target.add(
                                &mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar,
                                (pg_m * v2_m_diff + 2.0 * qg_m * vi_m * vr_m) * vm_m_n4
                            );
                        }

                        if let Vc::Var {i: ivar} = nd_info_imb.vi {
                            jac_target.add(
                                &mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar,
                                (-qg_m * v2_m_diff + 2.0 * pg_m * vi_m * vr_m) * vm_m_n4
                            );
                        }
                    }
                }
                if let Some(ieqn) = nd_info_ima.ieqn_kcl_im {
                    self.rhs[ieqn] += sgn_ima * (qg_m * vr_m - pg_m * vi_m) / vm_m_2;

                    for (nd_info_imb, sgn_imb) in [(&nd_info_im0, 1.0), (&nd_info_im1, -1.0)] {
                        let sgn = sgn_ima * sgn_imb;
                        if let Vc::Var{i: ivar} = bgi_m.pg {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -sgn * vi_m / vm_m_2);
                        }

                        if let Vc::Var{i: ivar} = bgi_m.qg {
                            jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, sgn * vr_m / vm_m_2);
                        }

                        if let Vc::Var {i: ivar} = nd_info_imb.vr {
                            jac_target.add(
                                &mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar,
                                (-qg_m * v2_m_diff + 2.0 * pg_m * vi_m * vr_m) * vm_m_n4
                            );
                        }

                        if let Vc::Var {i: ivar} = nd_info_imb.vi {
                            jac_target.add(
                                &mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar,
                                (-pg_m * v2_m_diff - 2.0 * qg_m * vi_m * vr_m) * vm_m_n4
                            );
                        }
                    }
                }
            }

            if let Some(ieqn) = bgi_m.ieqn_vr_c && let EqnKind::VrConstBr{br_idx: _, vr} = self.eqn_kinds[ieqn] {
                self.rhs[ieqn] = vr_m - vr;

                if let Vc::Var {i: ivar} = nd_info_im0.vr {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, 1.0);
                }

                if let Vc::Var {i: ivar} = nd_info_im1.vr {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -1.0);
                }
            }

            if let Some(ieqn) = bgi_m.ieqn_vi_c && let EqnKind::ViConstBr{br_idx: _, vi} = self.eqn_kinds[ieqn] {
                self.rhs[ieqn] = vi_m - vi;

                if let Vc::Var {i: ivar} = nd_info_im0.vi {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, 1.0);
                }

                if let Vc::Var {i: ivar} = nd_info_im1.vi {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -1.0);
                }
            }

            if let Some(ieqn) = bgi_m.ieqn_vm_c && let EqnKind::VmConstBr{br_idx: _, vm} = self.eqn_kinds[ieqn] {
                self.rhs[ieqn] = vm_m_2 - vm.powi(2);

                if let Vc::Var {i: ivar} = nd_info_im0.vr {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, 2.0 * vr_m);
                }

                if let Vc::Var {i: ivar} = nd_info_im1.vr {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -2.0 * vr_m);
                }

                if let Vc::Var {i: ivar} = nd_info_im0.vi {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, 2.0 * vi_m);
                }

                if let Vc::Var {i: ivar} = nd_info_im1.vi {
                    jac_target.add(&mut self.jac_csc, &self.jac_csc_idxs, ieqn, ivar, -2.0 * vi_m);
                }
            }
        }

        jac_target.finalize(&mut self.jac_csc, &mut self.jac_csc_idxs);
    }

    fn update_vars_after_solve(&mut self) {
        // On exit, RHS will contain the solution to J Delta x = rhs. Since NR normally solves J Delta x = -rhs
        // to determine the update, the solution to this system is actually the negative of the update, that we
        // need to *subtract* rather than add to the variables.
        for (vari, rhsi) in self.vars.iter_mut().zip(self.rhs.iter()) {
            *vari -= rhsi;
        }
    }

    fn propagate_to_model(&mut self) {
        for (nd, nd_info) in self.nmod.nodes.iter_mut().zip(self.node_infos.iter()) {
            nd.v = Some(c!(nd_info.vr.val(&self.vars), nd_info.vi.val(&self.vars)))
        }

        for br in self.nmod.branches.iter_mut() {
            if let Some(v0) = self.nmod.nodes[br.nd_0_idx].v && let Some(v1) = self.nmod.nodes[br.nd_1_idx].v {
                br.v_01 = Some(v0 - v1);
            }
        }
    }

    fn log_node_infos(&self, log: impl Fn(std::fmt::Arguments)) {
        log(format_args!("Nodes:"));
        for (i, nd_info) in self.node_infos.iter().enumerate() {
            log(
                format_args!(
                    "    {: <4}: vr = {: <24} vi = {: <24}",
                    i, format!("{:?}", nd_info.vr), format!("{:?}", nd_info.vi)
                )
            );
        }
    }

    pub fn debug_node_infos(&self) {
        if log::log_enabled!(log::Level::Debug) {
            self.log_node_infos(|args| debug!("{}", args));
        }
    }

    pub fn info_node_infos(&self) {
        if log::log_enabled!(log::Level::Info) {
            self.log_node_infos(|args| info!("{}", args));
        }
    }

    fn log_node_gen_infos(&self, log: impl Fn(std::fmt::Arguments)) {
        log(format_args!("Node Gens:"));
        for (i_ng, (i_nd, ng_info)) in self.node_gen_infos.iter().enumerate() {
            log(format_args!(
                "    {: <4}: nd = {: <4} qg = {: <24}", i_ng, i_nd, format!("{:?}", ng_info.qg)
            ));
        }
    }

    pub fn debug_node_gen_infos(&self) {
        if log::log_enabled!(log::Level::Debug) {
            self.log_node_gen_infos(|args| debug!("{}", args));
        }
    }

    pub fn info_node_gen_infos(&self) {
        if log::log_enabled!(log::Level::Info) {
            self.log_node_gen_infos(|args| info!("{}", args));
        }
    }

    fn log_branch_gen_infos(&self, log: impl Fn(std::fmt::Arguments)) {
        log(format_args!("Branch Gens:"));
        for (i_bg, (i_nd, bg_info)) in self.branch_gen_infos.iter().enumerate() {
            log(format_args!(
                "    {: <4}: br = {: <4} pg = {: <24} qg = {: <24}",
                i_bg, i_nd, format!("{:?}", bg_info.pg), format!("{:?}", bg_info.qg)
            ));
        }
    }

    pub fn debug_branch_gen_infos(&self) {
        if log::log_enabled!(log::Level::Debug) {
            self.log_branch_gen_infos(|args| debug!("{}", args));
        }
    }

    pub fn info_branch_gen_infos(&self) {
        if log::log_enabled!(log::Level::Info) {
            self.log_branch_gen_infos(|args| info!("{}", args));
        }
    }

    fn log_vars(&self, log: impl Fn(std::fmt::Arguments)) {
        log(format_args!("Variables:"));
        for (var_info, var) in self.var_infos.iter().zip(self.vars.iter()) {
            log(format_args!("    {:>10} = {:<16.8}", var_info.name(), var));
        }
    }

    pub fn debug_vars(&self) {
        if log::log_enabled!(log::Level::Debug) {
            self.log_vars(|args| debug!("{}", args));
        }
    }

    pub fn info_vars(&self) {
        if log::log_enabled!(log::Level::Info) {
            self.log_vars(|args| info!("{}", args));
        }
    }

    fn log_eqns(&self, log: impl Fn(std::fmt::Arguments)) {
        log(format_args!("Equations:"));
        for (eqn_kind, eqn) in self.eqn_kinds.iter().zip(self.rhs.iter()) {
            log(format_args!("    {:>10} = {:<16.8}", eqn_kind.name(), eqn));
        }
    }

    pub fn debug_eqns(&self) {
        if log::log_enabled!(log::Level::Debug) {
            self.log_eqns(|args| debug!("{}", args));
        }
    }

    pub fn info_eqns(&self) {
        if log::log_enabled!(log::Level::Info) {
            self.log_eqns(|args| info!("{}", args));
        }
    }

    /// Using a small delta x, compare the delta rhs computed using jac with that computed directly from RHS
    ///
    /// Return (delta_jac, delta_rhs, error)
    pub fn test_jac_against_rhs_ij(&mut self, i: usize, j: usize) -> (f64, f64, bool, f64) {

        let dxj = 1e-8;
        let max_allowed_err = dxj * 1e-4;

        let x0: Vec<f64> = self.vars.clone();
        let reset_x = |vars: &mut [f64]| {for (xi, x0i) in vars.iter_mut().zip(x0.iter()) {*xi = *x0i;}};

        reset_x(&mut self.vars);
        self.prepare_for_iter();
        let jij: f64 = self.jac_csc.get(i, j).copied().unwrap_or(0.0);
        debug!("i {} j {} jij {}", i, j, jij);
        let answer_jac = jij * dxj;

        reset_x(&mut self.vars);
        self.prepare_for_iter();
        let rhsma: f64 = self.rhs[i];
        debug!("RHS A {i} {rhsma}");

        self.vars[j] += dxj;
        self.prepare_for_iter();
        let rhsmb: f64 = self.rhs[i];
        debug!("RHS B {i} {rhsmb}");

        let answer_rhs = rhsmb - rhsma;
        let err = (answer_jac - answer_rhs).abs();
        if err > max_allowed_err {debug!("ERR! {i} {j} {err:e}")};
        debug!("TEST_JAC_AGAINST_RHS ({i}, {j}): jac: {} rhs: {} err: {}", answer_jac, answer_rhs, err);
        (answer_jac, answer_rhs, err <= max_allowed_err, err)
    }
}

impl NetworkSolver for NodalRectSolver {
    fn solve(&mut self) -> SolverStatus {
        debug!("solve");

        let start = Instant::now();

        let mut status = SolverStatus::default();

        self.init_vars();

        let mut converged: bool = self.ndim() == 0; // It's possible to set up problems with no vars and eqs.

        if !converged {
            for i in 0..self.params.max_iter {
                info!("Iteration {i}");

                let start_iter = Instant::now();

                status.iteration_statuses.push(SolverStatusItem::default());
                let iter_status = status.iteration_statuses.last_mut().unwrap();

                self.prepare_for_iter();
                self.debug_node_infos();
                self.debug_node_gen_infos();
                self.debug_branch_gen_infos();
                self.debug_vars();
                self.debug_eqns();

                let (i_residual, residual) = self.rhs
                    .iter().copied().map(f64::abs).enumerate().max_by(|(_, a), (_, b)| a.total_cmp(b)).unwrap();
                info!("residual = {} @ {}", residual, i_residual);
                iter_status.initial_residual = residual;

                if residual < self.params.tol {
                    info!("Converged, with residual = {} vs tolerance {}", residual, self.params.tol);
                    iter_status.flags |= SolverStatusFlags::SUCCESS;
                    converged = true;
                } else {
                    info!("solve_klu");
                    let start_ls = Instant::now();
                    let solve_klu_result = solve_klu(&mut self.jac_csc, &mut self.rhs);
                    let end_ls = Instant::now();
                    iter_status.linear_solver_time_s = (end_ls - start_ls).as_secs_f64();
                    if solve_klu_result.is_err() {
                        info!("solve_klu failed.");
                        iter_status.flags |= SolverStatusFlags::FAILURE;
                        iter_status.flags |= SolverStatusFlags::LINEAR_SOLVER_FAILURE;
                    } else {
                        info!("solve_klu succeeded.");
                        iter_status.flags |= SolverStatusFlags::SUCCESS;
                        self.update_vars_after_solve();
                        debug!("Updated vars:");
                        self.debug_vars();
                    }

                    let end_iter = Instant::now();
                    iter_status.total_time_s = (end_iter - start_iter).as_secs_f64();
                }

                if converged {
                    break;
                }
            }
        }

        let combine_final_flags = if converged {
            SolverStatusFlags::SUCCESS
        } else {
            SolverStatusFlags::FAILURE | SolverStatusFlags::MAX_ITER_EXCEEDED
        };
        status = status.finalize(Some(combine_final_flags));
        let iteration_residuals: Vec<_> = status.iteration_statuses.first()
            .map(|x| x.initial_residual)
            .into_iter()
            .chain(status.iteration_statuses.iter().map(|x| x.final_residual))
            .collect();

        info!("Converged = {:?}", converged);
        info!("iteration_residuals = {:?}", iteration_residuals);
        debug!("Final vars:");
        self.debug_vars();
        if converged {
            self.propagate_to_model();
        }
        let end = Instant::now();
        status.status.total_time_s = (end - start).as_secs_f64();
        status
    }

    fn propagate_to_network(&self, netw: &mut Network) {
        self.nmod.propagate_to_network(netw);
    }
}

inventory::submit! {
    SolverPlugin {
        name: "nodal_rect",
        constructor: |nmod| Box::new(NodalRectSolver::new(nmod)),
    }
}

#[cfg(test)]
mod tests {
    use rand::RngExt;
    use rand::SeedableRng;
    use rand::rngs::StdRng;

    use crate::nodal_rect_solver::*;
    use crate::test_networks::*;
    use crate::graph::graph_dot;
    use crate::{NetworkSolver, SolverStatusFlags};

    fn test_network(netw: Network) {
        let master = NetworkModel::new_master(&netw).unwrap();
        let islands = master.islands().unwrap();
        debug!("There are {} islands", islands.len());
        for nmod in islands.into_iter() {
            debug!("Solve network island");
            debug!("structure_graph:\n{:}", graph_dot(&nmod.structure_graph));
            debug!("network:\n{:?}", &netw);
            let mut solver = NodalRectSolver::new(nmod);
            let result = solver.solve();
            assert!(result.status.flags.contains(SolverStatusFlags::SUCCESS));
        }
    }

    #[test] fn test_solver_y() {
        configure_logging();
        test_network(test_netw_y(c!(1.0, 0.0)));
    }

    #[test] fn test_solver_y_cplx() {
        configure_logging();
        test_network(test_netw_y(c!(1.0, 1.2)));
    }

    #[test] fn test_solver_s() {
        configure_logging();
        test_network(test_netw_s(c!(0.123, 0.0)));
    }

    #[test] fn test_solver_s_cplx() {
        configure_logging();
        test_network(test_netw_s(c!(0.123, 0.0234)));
    }

    #[test] fn test_solver_yy() {
        configure_logging();
        test_network(test_netw_yy(c!(0.123, 0.0)));
    }

    #[test] fn test_solver_yy_cplx() {
        configure_logging();
        test_network(test_netw_yy(c!(0.123, 0.123)));
    }

    #[test] fn test_solver_ys() {
        configure_logging();
        test_network(test_netw_ys(c!(1.0, 0.0), c!(0.1, 0.0)));
    }

    #[test] fn test_solver_ys_cplx() {
        configure_logging();
        test_network(test_netw_ys(c!(3.15, 1.0), c!(0.123, 0.11)));
    }

    #[test] fn test_solver_tx() {
        configure_logging();
        test_network(test_netw_tx());
    }

    #[test] fn test_solver_large_a() {
        configure_logging();
        test_network(test_netw_large_a());
    }

    #[test] fn test_solver_large_b() {
        configure_logging();
        test_network(test_netw_large_b());
    }

    fn test_jac_against_rhs(netw: Network) {
        let mut rng = StdRng::seed_from_u64(42);

        let master = NetworkModel::new_master(&netw).unwrap();
        let islands = master.islands().unwrap();
        debug!("There are {} islands", islands.len());
        for nmod in islands.into_iter() {
            debug!("Solve network island");
            debug!("structure_graph:\n{}", graph_dot(&nmod.structure_graph));
            debug!("network:\n{:?}", &netw);

            let mut solver = NodalRectSolver::new(nmod);
            solver.init_vars();

            for (var, var_info) in solver.vars.iter_mut().zip(solver.var_infos.iter()) {
                *var = var_info.x0 + rng.random_range(-0.1..0.1);
            }

            solver.debug_vars();

            let var_names: Vec<String> = solver.var_infos.iter().map(|vi| vi.name()).collect();
            let eqn_names: Vec<String> = solver.eqn_kinds.iter().map(|ei| ei.name()).collect();

            #[allow(clippy::type_complexity)]
            let mut bad: Vec<((String, String), (f64, f64, f64))> = vec![];
            for (i, eqn_name) in eqn_names.iter().enumerate() {
                for (j, var_name) in var_names.iter().enumerate() {
                    debug!("{}", "=".repeat(80));
                    debug!("test_jac_against_rhs {}:{} {}:{}", i, eqn_name, j, var_name);
                    let (delta_jac, delta_rhs, ok, err) = solver.test_jac_against_rhs_ij(i, j);
                    if !ok {
                        bad.push(((eqn_names[i].clone(), var_names[j].clone()), (delta_jac, delta_rhs, err)));
                    }
                }
            }
            assert!(bad.is_empty(), "Test failed for elements {:?}", &bad);
        }
    }

    #[test] fn test_jac_against_rhs_ejson_i1l1s1() {
        configure_logging();
        test_jac_against_rhs(test_netw_ejson("netw-i1l1s1.json", None));
    }

    #[test] fn test_jac_against_rhs_ejson_i2l2s2() {
        configure_logging();
        test_jac_against_rhs(test_netw_ejson("netw-i2l2s2.json", None));
    }

    #[test] fn test_jac_against_rhs_ejson_i3l3s3() {
        configure_logging();
        test_jac_against_rhs(test_netw_ejson("netw-i3l3s3.json", None));
    }

    #[test] fn test_jac_against_rhs_y() {
        configure_logging();
        test_jac_against_rhs(test_netw_y(c!(0.658, 0.0)));
    }

    #[test] fn test_jac_against_rhs_y_cplx() {
        configure_logging();
        test_jac_against_rhs(test_netw_y(c!(0.658, 0.123)));
    }

    #[test] fn test_jac_against_rhs_s() {
        configure_logging();
        test_jac_against_rhs(test_netw_s(c!(0.5, 0.0)));
    }

    #[test] fn test_jac_against_rhs_s_cplx() {
        configure_logging();
        test_jac_against_rhs(test_netw_s(c!(0.5, 0.4)));
    }

    #[test] fn test_jac_against_rhs_yy() {
        configure_logging();
        test_jac_against_rhs(test_netw_yy(c!(0.658, 0.0)));
    }

    #[test] fn test_jac_against_rhs_yy_cplx() {
        configure_logging();
        test_jac_against_rhs(test_netw_yy(c!(0.658, 0.123)));
    }

    #[test] fn test_jac_against_rhs_ys() {
        configure_logging();
        test_jac_against_rhs(test_netw_ys(c!(0.658, 0.0), c!(0.23, 0.0)));
    }

    #[test] fn test_jac_against_rhs_ys_cplx() {
        configure_logging();
        test_jac_against_rhs(test_netw_ys(c!(0.658, 0.123), c!(0.23, 0.123)));
    }

    #[test] fn test_jac_against_rhs_tx() {
        configure_logging();
        test_jac_against_rhs(test_netw_tx());
    }

    #[test] fn test_jac_against_rhs_large_a() {
        configure_logging();
        test_jac_against_rhs(test_netw_large_a());
    }

    #[test] fn test_jac_against_rhs_large_b() {
        configure_logging();
        test_jac_against_rhs(test_netw_large_b());
    }
}
