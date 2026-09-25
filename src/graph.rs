//! Graph utilities, based on petgraph.

use petgraph::dot::{Config, Dot}; // Useful for debugging
use petgraph::graph::{UnGraph, NodeIndex, EdgeIndex, GraphIndex};
use petgraph::visit::{EdgeRef, GraphProp, IntoEdgeReferences, IntoNodeReferences, NodeIndexable, NodeRef};
use std::collections::{HashMap, HashSet, VecDeque};

/// Computes a fundamental cycle basis for an undirected graph using a BFS spanning tree.
///
/// Returns a vector of cycles, where each cycle is a vector of edge indices with orientation +-1.
///
/// `start_node`, if given, is used as a seed: the actual spanning-tree root is chosen
/// near the center of the seed's connected component (via double-sweep BFS) so that
/// tree-paths — and therefore fundamental cycles — stay short. Pass `None` to seed
/// from an arbitrary node.
///
/// Note: only the connected component containing the (seeded) root is covered. Chords
/// in other components of a disconnected graph will not appear in the returned basis.
pub fn fundamental_cycle_basis<N, E>(
    graph: &UnGraph<N, E>,
    start_node: Option<NodeIndex>
) -> Vec<Vec<(EdgeIndex, i32)>> {
    let (_, visited, tree_edges, parent) = build_spanning_tree(graph, start_node);

    // For each non-tree edge (chord) inside the handled component, create its
    // fundamental cycle.
    let mut basis = Vec::new();

    for edge_ref in graph.edge_references() {
        let edge_idx = edge_ref.id();
        if tree_edges.contains(&edge_idx) {
            continue;
        }

        let (u, v) = graph.edge_endpoints(edge_idx).unwrap();
        if !visited.contains(&u) || !visited.contains(&v) {
            continue; // edge belongs to a different component; not covered
        }

        let cycle = build_fundamental_cycle(u, v, edge_idx, &parent);
        let cycle = oriented_cycle(graph, &cycle);
        basis.push(cycle);
    }

    basis
}

// Builds the BFS spanning tree used by `fundamental_cycle_basis`: picks a low-eccentricity root near `start_node` (or
// an arbitrary node if `None`), then BFS-explores its connected component.
#[allow(clippy::type_complexity)]
fn build_spanning_tree<N, E>(
    graph: &UnGraph<N, E>,
    start_node: Option<NodeIndex>,
) -> (
    Option<NodeIndex>,
    HashSet<NodeIndex>,
    HashSet<EdgeIndex>,
    HashMap<NodeIndex, (NodeIndex, EdgeIndex)>,
) {
    let mut visited = HashSet::new();
    let mut tree_edges = HashSet::new();
    let mut parent: HashMap<NodeIndex, (NodeIndex, EdgeIndex)> = HashMap::new();

    let seed = start_node.or_else(|| graph.node_indices().next());
    let start = seed.map(|s| pick_root(graph, s));
    if let Some(start) = start {
        bfs_spanning_tree(graph, start, &mut visited, &mut tree_edges, &mut parent);
    }

    (start, visited, tree_edges, parent)
}

// BFS distances (and BFS-tree parents) from `start`, restricted to `start`'s
// connected component. Used by `pick_root` to locate a low-eccentricity root.
fn bfs_distances<N, E>(
    graph: &UnGraph<N, E>,
    start: NodeIndex,
) -> (HashMap<NodeIndex, u32>, HashMap<NodeIndex, NodeIndex>) {
    let mut dist = HashMap::new();
    let mut parent = HashMap::new();
    let mut queue = VecDeque::new();

    dist.insert(start, 0);
    queue.push_back(start);

    while let Some(node) = queue.pop_front() {
        let d = dist[&node];
        for edge_ref in graph.edges(node) {
            let neighbor = if edge_ref.target() == node {
                edge_ref.source()
            } else {
                edge_ref.target()
            };

            if let std::collections::hash_map::Entry::Vacant(e) = dist.entry(neighbor) {
                e.insert(d + 1);
                parent.insert(neighbor, node);
                queue.push_back(neighbor);
            }
        }
    }

    (dist, parent)
}

// Approximates the center of the component containing `seed` via double-sweep BFS: find a farthest node `a` from
// `seed`, then a farthest node `b` from `a`, then walk halfway back along the `a`-`b` path. Rooting the spanning tree
// here keeps tree-paths — and therefore fundamental cycles — short, compared to rooting at an arbitrary (possibly
// peripheral) node.
fn pick_root<N, E>(graph: &UnGraph<N, E>, seed: NodeIndex) -> NodeIndex {
    let (dist_seed, _) = bfs_distances(graph, seed);
    let a = *dist_seed.iter().max_by_key(|&(_, &d)| d).unwrap().0;

    let (dist_a, parent_a) = bfs_distances(graph, a);
    let b = *dist_a.iter().max_by_key(|&(_, &d)| d).unwrap().0;
    let ecc = dist_a[&b];

    let mut node = b;
    for _ in 0..ecc / 2 {
        node = parent_a[&node];
    }
    node
}

// BFS to build a spanning tree
fn bfs_spanning_tree<N, E>(
    graph: &UnGraph<N, E>,
    start: NodeIndex,
    visited: &mut HashSet<NodeIndex>,
    tree_edges: &mut HashSet<EdgeIndex>,
    parent: &mut HashMap<NodeIndex, (NodeIndex, EdgeIndex)>,
) {
    let mut queue = VecDeque::new();
    visited.insert(start);
    queue.push_back(start);

    while let Some(node) = queue.pop_front() {
        for edge_ref in graph.edges(node) {
            let neighbor = edge_ref.target();
            let edge_idx = edge_ref.id();
            // For undirected graphs, edges() can return the current node as target;
            // resolve the actual neighbor on either end.
            let neighbor = if neighbor == node {
                edge_ref.source()
            } else {
                neighbor
            };

            if !visited.contains(&neighbor) {
                visited.insert(neighbor);
                tree_edges.insert(edge_idx);
                parent.insert(neighbor, (node, edge_idx));
                queue.push_back(neighbor);
            }
        }
    }
}

// Builds a fundamental cycle by finding the tree path between u and v, then adding the chord edge
fn build_fundamental_cycle(
    u: NodeIndex, v: NodeIndex, chord: EdgeIndex, parent: &HashMap<NodeIndex, (NodeIndex, EdgeIndex)>,
) -> Vec<EdgeIndex> {
    let get_path_to_root = |start: NodeIndex| -> Vec<(NodeIndex, EdgeIndex)> {
        // Find path from u to root
        let mut path = Vec::<(NodeIndex, EdgeIndex)>::new();
        let mut curr = start;
        while let Some(&(p, edge)) = parent.get(&curr) {
            path.push((curr, edge));
            curr = p;
        }
        path
    };

    let mut path_u = get_path_to_root(u);
    let mut path_v = get_path_to_root(v);

    while !path_u.is_empty() && !path_v.is_empty() {
        if path_u.last().unwrap() == path_v.last().unwrap() {
            path_u.pop();
            path_v.pop();
        } else {
            break;
        }
    }

    // Build cycle: chord + path from u to LCA + path from LCA to v
    let mut cycle = vec![chord];
    cycle.extend(path_u.into_iter().map(|x| x.1));
    cycle.extend(path_v.into_iter().rev().map(|x| x.1));

    cycle
}

fn oriented_cycle<N, E>(
    graph: &UnGraph<N, E>,
    edges: &[EdgeIndex],
) -> Vec<(EdgeIndex, i32)> {
    if edges.is_empty() {
        return Vec::new();
    }

    let mut ordered_edges: Vec<(EdgeIndex, NodeIndex, NodeIndex)> = Vec::new();
    let (a, b) = graph.edge_endpoints(edges[0]).unwrap();

    // Maybe reverse so b always connects to edges[1].
    let (a, b) = if edges.len() < 2 {
        (a, b)
    } else {
        let (u2, v2) = graph.edge_endpoints(edges[1]).unwrap();
        if a == u2 || a == v2 { (b, a) } else { (a, b) }
    };

    ordered_edges.push((edges[0], a, b));
    let mut curr = b;

    for &edge_idx in &edges[1..] {
        let (u, v) = graph.edge_endpoints(edge_idx).unwrap();
        let (from, to) = if u == curr { (u, v) } else { (v, u) };
        ordered_edges.push((edge_idx, from, to));
        curr = to;
    }

    let mut oriented: Vec<(EdgeIndex, i32)> = ordered_edges
        .into_iter()
        .map(|(edge_idx, from, _to)| {
            let (canonical_src, _) = graph.edge_endpoints(edge_idx).unwrap();
            let orientation = if from == canonical_src { 1 } else { -1 };
            (edge_idx, orientation)
        })
        .collect();

    // Normalise: prefer the direction with a positive orientation sum.
    let orientation_sum: i32 = oriented.iter().map(|x| x.1).sum();
    if orientation_sum < 0 {
        for x in oriented.iter_mut() {
            x.1 *= -1;
        }
    }

    oriented
}

// Performs a depth-firsst search on an undirected graph, performing a function f on each node.
//
// The return value of f specifies whether to stop the search.
pub fn dfs<N, E, F>(
    graph: &UnGraph<N, E>,
    start: NodeIndex,
    mut f: F,
)
where
    F: FnMut(NodeIndex, Option<(NodeIndex, EdgeIndex)>) -> bool,
{
    let mut stack: Vec<(NodeIndex, Option<(NodeIndex, EdgeIndex)>)> = vec![(start, None)];
    let mut visited = HashSet::new();

    while let Some((node, incoming)) = stack.pop() {
        if visited.contains(&node) {
            continue;
        }
        visited.insert(node);

        if !f(node, incoming) {
            continue;
        }

        for edge in graph.edges(node) {
            let (src, tgt) = (edge.source(), edge.target());
            let next = if src == node { tgt } else { src };
            if !visited.contains(&next) {
                stack.push((next, Some((node, edge.id()))));
            }
        }
    }
}

/// Outputs a graph in dot format.
pub fn graph_dot<G>(g: G) -> String
where
    G: IntoEdgeReferences + IntoNodeReferences + NodeIndexable + GraphProp + Copy,
    G::EdgeWeight: std::fmt::Debug, G::NodeWeight: std::fmt::Debug, G::EdgeId: petgraph::graph::GraphIndex,
{
    format!("{:?}", Dot::with_attr_getters(
        g,
        &[Config::EdgeNoLabel, Config::NodeNoLabel],
        &|_, edge_ref| format!("label=\"{}:{:?}\"", edge_ref.id().index(), edge_ref.weight()),
        &|g, node_ref| format!("label=\"{}:{:?}\"", g.to_index(node_ref.id()), node_ref.weight()),
    ))
}

#[cfg(test)]
mod tests {
    use petgraph::dot::{Dot, Config};
    use super::*;

    #[test]
    fn test_graph() {
        let mut graph = UnGraph::<usize, (usize, usize)>::new_undirected();
        let nodes: Vec<_> = (0..4).map(|i| graph.add_node(i)).collect();

        // Complete graph on 4 vertices
        graph.add_edge(nodes[0], nodes[1], (0,1));
        graph.add_edge(nodes[2], nodes[0], (2, 0));
        graph.add_edge(nodes[0], nodes[3], (0, 3));
        graph.add_edge(nodes[1], nodes[2], (1, 2));
        graph.add_edge(nodes[3], nodes[1], (3, 1));
        graph.add_edge(nodes[2], nodes[3], (2, 3));

        println!("{:?}", Dot::with_attr_getters(
            &graph,
            &[Config::EdgeNoLabel, Config::NodeNoLabel],
            &|_, eref| format!("label = \"{}:{},{}\"", eref.id().index(), eref.source().index(), eref.target().index()),
            &|_, nref| format!("label = \"{}\"", nref.0.index()),
        ));

        let basis = fundamental_cycle_basis(&graph, None);
        for cyc in basis.iter() {
            println!("CYCLE");
            for (ei, orient) in cyc.iter() {
                println!("{:?} {:?} {:?}", ei, orient, graph.edge_endpoints(*ei).unwrap());
            }
        }

        // Cycle space dimension: E - V + 1 = 6 - 4 + 1 = 3
        assert_eq!(basis.len(), 3);
    }
}
