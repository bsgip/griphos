//! NetworkModel: low level one node per bus node model of a network.
//!
//! [`NetworkModel`]s provide a lower level bridge to the [`Network`] that are useful for writing solvers.

use std::collections::{BTreeSet, HashSet, HashMap};
use std::ops::{Add, MulAssign};
use std::vec::Vec;

use petgraph::graph::UnGraph;
use petgraph::prelude::{NodeIndex};
use petgraph::algo::kosaraju_scc;
use petgraph::visit::EdgeRef;

use serde::Serialize;

use crate::foundation::*;
use crate::network::*;
use crate::graph::*;

/// A node is a single phase of a bus, to be used in a NetworkModel. It has a complex voltage.
#[derive(Clone, Debug, Serialize)]
pub struct NmNode {
    /// Nominal voltage
    pub v_nom: c64,

    /// Is the node grounded?
    pub is_grounded: bool,

    /// There can be more than one BusNodeId due to the possibility of copperplating.
    pub bus_node_ids: Vec<BusNodeId>,

    /// Voltage, in chosen units, if known.
    pub v: Option<c64>,
}

impl NmNode {
    /// Creates a new [`NmNode`].
    fn new(v_nom: c64, is_grounded: bool) -> NmNode {
        NmNode {v_nom, is_grounded, bus_node_ids: vec![], v: None}
    }

    /// Display name for the node.
    pub fn name(&self) -> String {
        self.bus_node_ids.iter().map(|bn| bn.to_string()).collect::<Vec<_>>().join("|")
    }
}

impl_g_serialize!(NmNode);
impl_disp_from_fmt!(NmNode);

/// A branch is an ordered connection between two nodes. It has a current and a voltage.
#[derive(Clone, Debug, Serialize)]
pub struct NmBranch {
    /// NmNode 0 index
    pub nd_0_idx: usize,

    /// NmNode 1 index
    pub nd_1_idx: usize,

    /// Nominal voltage, based on the nodes.
    pub v_nom: c64,

    /// Voltage v_01 = v0 - v1
    pub v_01: Option<c64>,

    /// Current i->j
    pub i_01: Option<c64>,
}

impl NmBranch {
    /// Creates a new branch.
    fn new(nd_0_idx: usize, nd_1_idx: usize, v_nom: c64) -> NmBranch {
        NmBranch {nd_0_idx, nd_1_idx, v_nom, v_01: None, i_01: None}
    }
}

impl_g_serialize!(NmBranch);
impl_disp_from_fmt!(NmBranch);

/// # [`NetworkModel`]s
///
/// A [`NetworkModel`] is a lower level representation of the network, which breaks apart the grouping into [`Bus`]es
/// with phases and [`Element`]s with branches, and instead works with [`NmNode`]s (each corresponding to a single phase
/// of a bus) and [`NmBranch`]es (each corresponding to a single connection between two nodes).
#[derive(Debug)]
pub struct NetworkModel {
    // Distinct nodes (wrt copperplating), keys are uid
    pub nodes: Vec<NmNode>,

    // Branches, keys are uid
    pub branches: Vec<NmBranch>,

    // BusNodeId to NmNode map
    pub bt_to_nd_idx: HashMap<BusNodeId, usize>,

    // ElemTerm to NmBranch map
    pub eb_to_br_idx: HashMap<ElemBranchId, usize>,

    // Structure graph: graph nodes are network nodes, graph edges are branches.
    // Labels are master node index, master branch index.
    pub structure_graph: UnGraph<(), ()>,

    pub y_const: CsMat<c64>, // Indices are branch indices (not UIDs) in this NetworkModel's branches
    pub i_const: CsVec<c64>, // Indices are branch indices (not UIDs) in this NetworkModel's branches
    pub s_const: CsVec<c64>, // Indices are branch indices (not UIDs) in this NetworkModel's branches
    pub vmag_const: CsVec<f64>, // Indices are branch indices (not UIDs) in this NetworkModel's branches
    pub vang_const: CsVec<f64>, // Indices are branch indices (not UIDs) in this NetworkModel's branches
}

impl Default for NetworkModel {
    fn default() -> Self {
        Self::new()
    }
}

impl NetworkModel {
    /// Creates an netw empty NetworkModel.
    pub fn new() -> NetworkModel {
        NetworkModel {
            nodes: vec![],
            branches: vec![],
            bt_to_nd_idx: HashMap::new(),
            eb_to_br_idx: HashMap::new(),
            structure_graph: UnGraph::new_undirected(),
            y_const: CsMat::empty(sprs::CompressedStorage::CSC, 0),
            i_const: CsVec::empty(0),
            s_const: CsVec::empty(0),
            vmag_const: CsVec::empty(0),
            vang_const: CsVec::empty(0),
        }
    }

    /// Creates an netw master NetworkModel from a [`Network`].
    ///
    /// The master model contains all nodes and branches in the network, as distinct from island models, which split the
    /// network into electrically isolated islands.
    pub fn new_master(netw: &Network) -> Result<NetworkModel, String> {
        let mut nmod = NetworkModel::new();
        nmod.make_nodes(netw)?;
        nmod.make_branches(netw)?;
        nmod.make_structure_graph()?;

        nmod.y_const = nmod.elem_sparse_mat(netw, |x| x.y_const())?;
        nmod.i_const = nmod.elem_sparse_vec(netw, |x| x.i_const())?;
        nmod.s_const = nmod.elem_sparse_vec(netw, |x| x.s_const())?;
        nmod.vmag_const = nmod.elem_sparse_vec(netw, |x| x.vmag_const())?;
        nmod.vang_const = nmod.elem_sparse_vec(netw, |x| x.vang_const())?;

        Ok(nmod)
    }

    /// Splits a master NetworkModel into islands.
    pub fn islands(&self) -> Result<Vec<NetworkModel>, String> {
        // Ensure that transformers don't separate out two components that should be together.
        // We do this by adding extra edges to the structure.

        let mut sg_plus = self.structure_graph.clone();

        for (_, (ibr, jbr)) in self.y_const.iter() {
            let n_bri_0 = self.branches[ibr].nd_0_idx;
            let n_bri_1 = self.branches[ibr].nd_1_idx;
            let n_brj_0 = self.branches[jbr].nd_0_idx;
            let n_brj_1 = self.branches[jbr].nd_1_idx;
            if [n_bri_0, n_bri_1, n_brj_0, n_brj_1].into_iter().collect::<HashSet<usize>>().len() == 4 {
                sg_plus.add_edge(NodeIndex::new(n_bri_0), NodeIndex::new(n_brj_1), ());
            }
        }

        let ccs = kosaraju_scc(&sg_plus);

        let mut nd_idx_to_island: HashMap<usize, usize> = HashMap::new();
        let mut nd_idxs_of_island: Vec<BTreeSet<usize>> = (0..ccs.len()).map(|_| BTreeSet::new()).collect();
        for (island_idx, cci) in ccs.iter().enumerate() {
            for nd_idx in cci {
                nd_idx_to_island.insert(nd_idx.index(), island_idx);
                nd_idxs_of_island[island_idx].insert(nd_idx.index());
            }
        }

        let mut br_idxs_of_island: Vec<BTreeSet<usize>> = (0..ccs.len()).map(|_| BTreeSet::new()).collect();
        for (br_idx, br) in self.branches.iter().enumerate() {
            let island_idx = nd_idx_to_island[&br.nd_0_idx];
            br_idxs_of_island[island_idx].insert(br_idx);
        }

        let nd_idxs_of_island: Vec<Vec<usize>> = nd_idxs_of_island.into_iter()
            .map(|btreeset| btreeset.into_iter().collect()).collect();

        let br_idxs_of_island: Vec<Vec<usize>> = br_idxs_of_island.into_iter()
            .map(|btreeset| btreeset.into_iter().collect()).collect();

        let mut island_models: Vec<NetworkModel> = Vec::with_capacity(nd_idxs_of_island.len());

        for (island_nd_master_idxs, island_br_master_idxs) in nd_idxs_of_island.iter().zip(br_idxs_of_island) {
            let n_branches = island_br_master_idxs.len();

            // Map master -> island indices for nodes and branches.
            let nd_map: HashMap<usize, usize> = island_nd_master_idxs.iter().enumerate()
                .map(|(iisland, &imaster)| (imaster, iisland)).collect();
            let br_map: HashMap<usize, usize> = island_br_master_idxs.iter().enumerate()
                .map(|(iisland, &imaster)| (imaster, iisland)).collect();

            let nd_idx_set: HashSet<usize> = island_nd_master_idxs.iter().copied().collect();
            let br_idx_set: HashSet<usize> = island_br_master_idxs.iter().copied().collect();

            let bt_to_nd_idx = self.bt_to_nd_idx.iter()
                .filter(|(_, nd_idx)| nd_idx_set.contains(*nd_idx))
                .map(|(bn, nd_idx)| (bn.clone(), *nd_idx)).collect();

            let eb_to_br_idx = self.eb_to_br_idx.iter()
                .filter(|(_, br_idx)| br_idx_set.contains(*br_idx))
                .map(|(eb, br_idx)| (eb.clone(), *br_idx)).collect();

            let island_nodes = island_nd_master_idxs.iter()
                .map(|&nd_idx_master| self.nodes[nd_idx_master].clone())
                .collect::<Vec<NmNode>>();

            let island_branches = island_br_master_idxs.iter()
                .map(|&br_idx_master| self.branches[br_idx_master].clone())
                .collect::<Vec<NmBranch>>();

            let mut island_structure_graph: UnGraph<(), ()> = UnGraph::new_undirected();

            island_nd_master_idxs.iter().for_each(|_| {island_structure_graph.add_node(());});

            for e in self.structure_graph.edge_references() {
                let nd_idx_master_src = e.source().index();
                let nd_idx_master_trg = e.target().index();
                if let Some(&nd_idx_island_src) = nd_map.get(&nd_idx_master_src)
                && let Some(&nd_idx_island_trg) = nd_map.get(&nd_idx_master_trg) {
                    island_structure_graph.add_edge(
                        NodeIndex::new(nd_idx_island_src), NodeIndex::new(nd_idx_island_trg), ()
                    );
                }
            }

            let y_const = map_spmat(&self.y_const, n_branches, &br_map);
            let i_const = map_spvec(&self.i_const, n_branches, &br_map);
            let s_const = map_spvec(&self.s_const, n_branches, &br_map);
            let vmag_const = map_spvec(&self.vmag_const, n_branches, &br_map);
            let vang_const = map_spvec(&self.vang_const, n_branches, &br_map);

            island_models.push(NetworkModel {
                nodes: island_nodes,
                branches: island_branches,
                bt_to_nd_idx,
                eb_to_br_idx,
                y_const,
                i_const,
                s_const,
                vmag_const,
                vang_const,
                structure_graph: island_structure_graph,
            });
        }
        Ok(island_models)
    }

    #[allow(clippy::type_complexity)]
    fn make_nodes(&mut self, netw: &Network) -> Result<(), String> {
        // We want to coalesce any bus/phase pairs that are connected by copperplate connections
        // into a single node.

        // Create a graph of copper plate connected bus/phase pairs
        let mut copper = UnGraph::<BusNodeId, ()>::new_undirected();

        // Insert all bus/phase pairs as graph nodes while noting the mapping between bus/phase and
        // node index.
        let mut bt_to_gnode = HashMap::<BusNodeId, NodeIndex>::new();
        for bus in netw.buses() {
            let bus = bus.borrow();
            for t in bus.terms.iter() {
                let bn = bn!(&bus.uid(), t.phase);
                if !bt_to_gnode.contains_key(&bn) {
                    let idx = copper.add_node(bn.clone());
                    bt_to_gnode.insert(bn.clone(), idx);
                }
            }
        }

        // Insert copper plate connections as edges.
        for e in netw.elements() {
            let e = e.borrow();
            let cps = e.copper_plate();
            for cp in cps {
                let gnodei = bt_to_gnode[&e.bus_node_ids[cp[0]]];
                for j in &cp[1..] {
                    let gnodej = bt_to_gnode[&e.bus_node_ids[cp[*j]]];
                    copper.update_edge(gnodei, gnodej, ());
                }
            }
        }

        // Copper plate connect all ground nodes to the first ground node. This will have the
        // effect that there is a single ground node.
        let mut first_ground: Option<NodeIndex> = None;
        for e in netw.elements() {
            let e = e.borrow();
            let gnd = e.grounding();
            for i in gnd {
                let gnodei = bt_to_gnode[&e.bus_node_ids[i]];
                if let Some(g0) = first_ground {
                    copper.update_edge(gnodei, g0, ());
                } else {
                    first_ground = Some(gnodei);
                }
            }
        }

        // Find connected components.
        let ccs = kosaraju_scc(&copper);

        // For each connected component, make a single node and use it for all bus/phase pairs in
        // the component.
        let mut nodes: Vec<(usize, NmNode)> = Vec::with_capacity(ccs.len());

        {
            let mut bt_to_nd_idx: HashMap<BusNodeId, usize> = HashMap::new();
            for cc in ccs.iter() {
                // Get v_base by looking at the first bus that has this node.
                let bt0 = copper.node_weight(cc[0]).unwrap();
                let v_base = netw.bus(&bt0.b).unwrap().borrow().v_base;
                let v_nom = v_base * bt0.p.v_nom_pu();

                // Create a shared node for this set of copperplated nodes.
                nodes.push((nodes.len(), NmNode::new(v_nom, false)));
                let (nd_idx, node) = &mut nodes.last_mut().unwrap();

                for &cc_i in cc.iter() {
                    // TODO: Think about what happens if two nodes with different v_base are copperplated.
                    let bn = copper.node_weight(cc_i).unwrap();
                    assert!(bn.p == bt0.p, "Can't copperplate two bus/phase pairs with different phases: {bt0} {bn}");
                    node.bus_node_ids.push(bn.clone());
                    bt_to_nd_idx.insert(bn.clone(), *nd_idx);
                }
                node.bus_node_ids.sort();
            }

            // Set grounding flag
            for e in netw.elements() {
                for i in e.borrow().grounding() {
                    let nd = &mut nodes[bt_to_nd_idx[&e.borrow().bus_node_ids[i]]];
                    nd.1.is_grounded = true;
                    nd.1.v = Some(c!(0.0, 0.0));
                }
            }

            // Run through bus/phases in order and set node indices.
            let mut seen = HashSet::<usize>::new();
            let mut i = 0usize;
            for b in netw.buses() {
                for t in b.borrow().terms.iter() {
                    let bn = bn!(b.borrow().uid, t.phase);
                    let (nd_idx, _) = &mut nodes[bt_to_nd_idx[&bn]];
                    if seen.insert(*nd_idx) {
                        *nd_idx = i;
                        i += 1;
                    }
                }
            }

            nodes.sort_by_key(|(i, _)| *i);
        }

        self.nodes = nodes.into_iter().map(|(_, nd)| nd).collect();

        for (nd_idx, nd) in self.nodes.iter().enumerate() {
            for bn in nd.bus_node_ids.iter() {
                self.bt_to_nd_idx.insert(bn.clone(), nd_idx);
            }
        }

        Ok(())
    }

    fn make_branches(
        &mut self,
        netw: &Network,
    ) -> Result<(), String> {
        for elem in netw.elements() {
            let elem = elem.borrow();
            for (ibr, (i, j)) in elem.branch_wiring().iter().enumerate() {
                let indi = self.bt_to_nd_idx[&elem.bus_node_ids[*i]];
                let indj = self.bt_to_nd_idx[&elem.bus_node_ids[*j]];
                let v_nom = self.nodes[indi].v_nom - self.nodes[indj].v_nom;
                self.eb_to_br_idx.insert(eb!(elem.uid, ibr), self.branches.len());
                self.branches.push(NmBranch::new(indi, indj, v_nom));
            }
        }
        Ok(())
    }

    fn make_structure_graph(&mut self) -> Result<(), String> {
        for _ in self.nodes.iter() {
            self.structure_graph.add_node(());
        }

        for b in self.branches.iter() {
            let idx_0 = NodeIndex::new(b.nd_0_idx);
            let idx_1 = NodeIndex::new(b.nd_1_idx);
            self.structure_graph.add_edge(idx_0, idx_1, ());
        }

        Ok(())
    }

    fn elem_sparse_vec<F, T>(&self, netw: &Network, f: F) -> Result<CsVec<T>, String>
    where
        F: Fn(&Element) -> Vec<(usize, T)>,
        T: Clone + Add<Output = T> + MulAssign<f64>
    {
        let n_branches = self.branches.len();
        let mut entries: Vec<(usize, T)> = Vec::with_capacity(n_branches);
        netw.foreach_element(|elem| {
            for (i, x) in f(elem) {
                let eb = eb!(&elem.uid, i);
                let idx_br = self.eb_to_br_idx[&eb];
                entries.push((idx_br, x));
            }
        });
        entries.sort_by_key(|(idx, _)| *idx);
        let (indices, data): (Vec<_>, Vec<_>) = entries.into_iter().unzip();
        let vec: CsVec<T> = CsVec::new(n_branches, indices, data);
        Ok(vec)
    }

    fn elem_sparse_mat<F, T>(&self, netw: &Network, f: F) -> Result<CsMat<T>, String>
    where
        F: Fn(&Element) -> CooMat<T>,
        T: Clone + Add<Output = T>
    {
        let n_branches = self.branches.len();
        let mut tri = TriMat::<T>::new((n_branches, n_branches));
        netw.foreach_element(|elem| {
            for ((i, j), x) in f(elem) {
                let eb_i = eb!(&elem.uid, i);
                let eb_j = eb!(&elem.uid, j);
                let idx_br_i = self.eb_to_br_idx[&eb_i];
                let idx_br_j = self.eb_to_br_idx[&eb_j];
                tri.add_triplet(idx_br_i, idx_br_j, x);
            }
        });
        Ok(tri.to_csc())
    }

    /// Sets the node voltages based on branch voltages.
    pub fn propagate_branch_voltages_to_nodes(&mut self) {
        // TODO: for now we're only using ground to propagate the voltages.
        let mut somes: Vec<usize> = self.nodes.iter().enumerate()
            .filter(|(_, nd)| nd.v.is_some()).map(|(i, _)| i).collect();

        if somes.is_empty() {
            return;
        }

        somes.sort_by(
            |i, j| self.nodes[*i].v.unwrap().norm_sqr().partial_cmp(&self.nodes[*j].v.unwrap().norm_sqr())
                .unwrap_or(std::cmp::Ordering::Equal)
        );

        let mut n_none = self.nodes.len() - somes.len();

        while n_none > 0 {
            let idx_start = &somes[0];
            let idx_start = NodeIndex::new(*idx_start);

            dfs(&self.structure_graph, idx_start, |cur_node_idx, maybe_incoming| -> bool {
                if let Some((prev_node_idx, edge_idx)) = maybe_incoming {
                    // Only update if we're coming from a previous node.
                    let cur_node_v = &self.nodes[cur_node_idx.index()].v;
                    if cur_node_v.is_none() {
                        let prev_node = &self.nodes[prev_node_idx.index()];
                        let branch = &self.branches[edge_idx.index()];
                        let prev_node_v = prev_node.v.unwrap(); // Will never be None
                        if let Some(v_01) = branch.v_01 {
                            let v_cur_prev = {
                                if cur_node_idx.index() == branch.nd_1_idx {-v_01}
                                else {v_01}
                            };
                            self.nodes[cur_node_idx.index()].v = Some(prev_node_v + v_cur_prev);
                            n_none -= 1;
                            true
                        } else {
                            false
                        }
                    } else {
                        true
                    }
                } else {
                    true
                }
            });
        }
    }

    /// Sets the bus voltages based on nodes, and the element voltages and currents based on branches.
    pub fn propagate_to_network(&self, netw: &mut Network) {
        for (bn, nd_idx) in self.bt_to_nd_idx.iter() {
            if let Some(nd_v) = self.nodes[*nd_idx].v {
                netw.bus(&bn.b).unwrap().borrow_mut().bus_node_mut(bn.p).unwrap().v = nd_v;
            }
        }

        for (eb, br_idx) in self.eb_to_br_idx.iter() {
            let br = &self.branches[*br_idx];
            let elem_shared = netw.element(&eb.e).unwrap();
            let mut elem = elem_shared.borrow_mut();
            let elem_br = &mut elem.branches[eb.b];
            if let Some(v) = br.v_01 {
                elem_br.v = v;
            }
            if let Some(i) = br.i_01 {
                elem_br.i = i;
            }
        }
    }
}

/// Convenience struct to store master and island [`NetworkModel`]s.
#[derive(Debug)]
pub struct NetworkModels {
    pub master: NetworkModel,
    pub islands: Vec<NetworkModel>,
}

impl NetworkModels {
    /// Creates a new [`NetworkModels`].
    pub fn new(netw: &Network) -> NetworkModels {
        let master = NetworkModel::new_master(netw).unwrap();
        let islands = master.islands().unwrap();
        NetworkModels {master, islands}
    }
}

#[cfg(test)]
mod tests {
    use crate::e_json::*;
    use crate::generic_element::GenericElement;
    use super::*;

    /// Make a network with optional ground for the first bus and specified phases for the rest.
    fn make_network(add_ground: bool, bus_n_phases: &[usize]) -> Network {
        let mut netw = Network::new();
        if add_ground {
            netw.add_bus(Bus::new("bus_g", vec![Phase::A], 1.0)).unwrap();
        }
        for (i, n_phs) in bus_n_phases.iter().enumerate() {
            let uid = format!("bus_{i}");
            let phs = match n_phs {
                0 => todo!(),
                1 => vec![Phase::A],
                2 => vec![Phase::A, Phase::B],
                3 => vec![Phase::A, Phase::B, Phase::C],
                4.. => todo!(),
            };
            netw.add_bus(Bus::new(&uid, phs, 1.0)).unwrap();
        }

        netw
    }

    #[test]
    fn test_coalesce_nodes() {
        let mut netw = make_network(false, &[1, 1, 1]); // 3 single phase buses.

        let y_const = vec![((0, 0), c!(1.0, 0.0))]; // A single branch constant Y
        let copper_plate = vec![vec![0, 1]]; // Copper plate two nodes together

        netw.add_element(
            (GenericElement {n_terms: 2, branch_wiring: vec![(0, 1)], y_const, ..Default::default()})
                .element("y01", vec![bn!("bus_0", Phase::A), bn!("bus_1", Phase::A)])).unwrap();

        netw.add_element(
            (GenericElement {n_terms: 2, copper_plate, ..Default::default()})
                .element("cp12", vec![bn!("bus_1", Phase::A), bn!("bus_2", Phase::A)])).unwrap();

        let mods = NetworkModels::new(&netw);
        assert!(mods.master.nodes.len() == 2);
        assert!(netw.bus("bus_0").unwrap().borrow().terms.len() == 1);
        assert!(netw.bus("bus_1").unwrap().borrow().terms.len() == 1);
        assert!(netw.bus("bus_2").unwrap().borrow().terms.len() == 1);
    }

    // TODO: May have taken the teeth out of some of these tests, review.
    #[test]
    fn test_coalesce_ground() {
        // Four single phase buses, no ground.
        let mut netw = make_network(false, &[1, 1, 1, 1]);

        // Function to connect two buses with a constant y connection
        let y_const = vec![((0, 0), c!(1.0, 0.0))];

        fn bts(i: usize, j: usize) -> Vec<BusNodeId> {
            vec![
                bn!(&format!("bus_{i}"), Phase::A),
                bn!(&format!("bus_{j}"), Phase::A),
            ]
        }

        // b0-b1g-b2g-b3
        // We expect buses 2 and 3 to get aggregated into a single ground node, thus the indexing
        // should be 0, 1, 1, 2
        netw.add_element(
            (GenericElement {n_terms: 2, branch_wiring: vec![(0, 1)], y_const: y_const.clone(), ..Default::default()})
                .element("y01", bts(0, 1))).unwrap();
        netw.add_element(
            (GenericElement {n_terms: 2, branch_wiring: vec![(0, 1)], y_const: y_const.clone(), ..Default::default()})
                .element("y12", bts(1, 2))).unwrap();
        netw.add_element(
            (GenericElement {n_terms: 2, grounding: vec![0, 1], ..Default::default()})
            .element("g12", bts(1, 2))).unwrap();
        netw.add_element(
            (GenericElement {n_terms: 2, branch_wiring: vec![(0, 1)], y_const, ..Default::default()})
            .element("y23", bts(2, 3))).unwrap();
        let mods = NetworkModels::new(&netw);
        assert!(mods.master.nodes.len() == 3);
    }

    #[test]
    fn test_cycles_and_islands_a() {
        // Setup: create a phase map (adjust based on your actual Phase type)
        let ejson_phase_map = ejson_phase_map_abcng();

        let netw = parse_ejson_file("test_data/netw-i1l1s1.json", &ejson_phase_map).expect("Failed to parse e-JSON");
        let mods = NetworkModels::new(&netw);

        println!("Network:\n{}", &netw);

        println!("{:?}", mods.master.y_const.to_dense());
        println!("{}", graph_dot(&mods.master.structure_graph));
        assert!(mods.islands.len() == 1);
    }

    #[test]
    fn test_cycles_and_islands_b() {
        // Setup: create a phase map (adjust based on your actual Phase type)
        let ejson_phase_map = ejson_phase_map_abcng();

        let netw = parse_ejson_file("test_data/netw-i2l2s2.json", &ejson_phase_map)
            .expect("Failed to parse e-JSON");

        println!("Network:\n{}", Wrap(&netw));

        let mods = NetworkModels::new(&netw);
        println!("{:?}", mods.master.y_const.to_dense());
        println!("{}", graph_dot(&mods.master.structure_graph));
        assert!(mods.islands.len() == 1);
    }

    #[test]
    fn test_cycles_and_islands_c() {
        // Setup: create a phase map (adjust based on your actual Phase type)
        let ejson_phase_map = ejson_phase_map_abcng();

        let netw = parse_ejson_file("test_data/netw-dup-i1l1s1.json", &ejson_phase_map).expect("Failed to parse e-JSON");
        println!("Network:\n{}", Wrap(&netw));

        let mods = NetworkModels::new(&netw);
        println!("{:?}", mods.master.y_const.to_dense());
        println!("{}", graph_dot(&mods.master.structure_graph));
        assert!(mods.islands.len() == 1)
    }
}
