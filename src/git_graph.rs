// TODO Only for test purposes REMOVE ME
#![allow(dead_code)]
use std::collections::HashMap;

use petgraph::{
    data::Build,
    graph::NodeIndex,
    visit::{EdgeRef, IntoEdgeReferences},
    Graph, Undirected,
};
use rayon::iter::{IntoParallelIterator, ParallelIterator};
#[derive(Copy, Clone)]
pub enum Status {
    Added,
    Deleted,
    Modified,
}

#[derive(Clone)]
pub struct GitFile {
    pub status: Status,
    pub name: String,
}

#[derive(Default)]
pub struct GitGraph {
    pub graph: Graph<GitFile, i64, Undirected>,
    name_table: HashMap<String, NodeIndex>,
}

pub type ChangeSet = Vec<GitFile>;

pub fn build_graph(changes: Vec<ChangeSet>) -> GitGraph {
    changes
        .into_par_iter()
        .map(GitGraph::from_chageset)
        .reduce(GitGraph::default, |a, b| a.merge(b))
}

impl GitGraph {
    fn from_chageset(changes: ChangeSet) -> Self {
        let mut graph = Graph::new_undirected();
        let mut name_table = HashMap::new();
        let mut nodes: Vec<NodeIndex> = Vec::new();

        for file in changes {
            if name_table.contains_key(&file.name) {
                continue;
            }
            let idx = graph.add_node(file.clone());
            nodes.push(idx);
            name_table.insert(file.name, idx);
        }

        for (a, b) in combinations_k_2(nodes.len()) {
            graph.add_edge(nodes[a], nodes[b], 1);
        }

        GitGraph { graph, name_table }
    }
    fn merge(self, other: GitGraph) -> Self {
        if self.len() == 0 {
            return other;
        }
        if other.len() == 0 {
            return self;
        }
        let (mut new, old) = if self.len() >= other.len() {
            (self, other)
        } else {
            (other, self)
        };
        let GitGraph {
            graph: old_graph,
            name_table: old_names,
        } = old;
        let old_edges: Vec<(NodeIndex, NodeIndex, i64)> = old_graph
            .edge_references()
            .map(|e| (e.source(), e.target(), e.weight().to_owned()))
            .collect();
        let mut index_rewrites: HashMap<NodeIndex, NodeIndex> =
            HashMap::with_capacity(old_names.len());

        for old_node in old_graph.into_nodes_edges().0.into_iter() {
            let name = old_node.weight.name.clone();
            let old_idx = old_names[&name];

            if let Some(idx) = new.name_table.get(&name) {
                index_rewrites.insert(old_idx, *idx);
            } else {
                let idx = new.graph.add_node(old_node.weight);
                new.name_table.insert(name, idx);

                index_rewrites.insert(old_idx, idx);
            }
        }

        for (source, target, weight) in old_edges.into_iter() {
            let source = index_rewrites[&source];
            let target = index_rewrites[&target];
            if let Some(edge) = new.graph.find_edge(source, target) {
                *new.graph.edge_weight_mut(edge).unwrap() += weight;
            } else {
                new.graph.add_edge(source, target, weight);
            }
        }

        new
    }
    fn len(&self) -> usize {
        self.name_table.len()
    }
}

fn combinations_k_2(n: usize) -> impl Iterator<Item = (usize, usize)> {
    (0..n)
        .map(move |x| (x..n).filter_map(move |y| if x < y { Some((x, y)) } else { None }))
        .flatten()
}

#[test]
fn test_build_graph_single_changeset() {
    let first_changeset = vec![
        GitFile {
            status: Status::Added,
            name: "a".to_string(),
        },
        GitFile {
            status: Status::Added,
            name: "b".to_string(),
        },
        GitFile {
            status: Status::Added,
            name: "c".to_string(),
        },
    ];

    let graph = build_graph(vec![first_changeset]);

    // Check name_table integrity
    assert_eq!(graph.name_table.len(), 3);
    assert_eq!(graph.graph.node_count(), 3);
    for (k, v) in &graph.name_table {
        assert_eq!(k, &graph.graph[*v].name);
    }

    // Check edges
    assert_eq!(graph.graph.edge_count(), 3);
    let expected_edges = vec![("a", "b"), ("a", "c"), ("b", "c")];
    let edges: Vec<(&str, &str)> = graph
        .graph
        .edge_references()
        .map(|e| (e.source(), e.target()))
        .map(|(a, b)| (&graph.graph[a].name, &graph.graph[b].name))
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    assert_eq!(edges, expected_edges);
    for e in graph.graph.edge_references() {
        assert_eq!(e.weight().to_owned(), 1);
    }
}

#[test]
fn test_build_graph_multi_changesets() {
    let change_sets = vec![
        vec![GitFile {
            status: Status::Added,
            name: "a".to_string(),
        }],
        vec![
            GitFile {
                status: Status::Added,
                name: "b".to_string(),
            },
            GitFile {
                status: Status::Added,
                name: "c".to_string(),
            },
        ],
        vec![
            GitFile {
                status: Status::Modified,
                name: "a".to_string(),
            },
            GitFile {
                status: Status::Added,
                name: "d".to_string(),
            },
        ],
        vec![
            GitFile {
                status: Status::Modified,
                name: "a".to_string(),
            },
            GitFile {
                status: Status::Added,
                name: "b".to_string(),
            },
            GitFile {
                status: Status::Added,
                name: "c".to_string(),
            },
        ],
    ];

    let graph = build_graph(change_sets);

    assert_eq!(graph.name_table.len(), 4);
    assert_eq!(graph.graph.node_count(), graph.name_table.len());

    // Check edges
    let expected_edges = vec![("a", "b"), ("a", "c"), ("a", "d"), ("b", "c")];
    let mut edges: Vec<(&str, &str)> = graph
        .graph
        .edge_references()
        .map(|e| (e.source(), e.target()))
        .map(|(a, b)| (&graph.graph[a].name, &graph.graph[b].name))
        .map(|(a, b)| (a.as_str(), b.as_str()))
        .collect();
    edges.sort_by(|a, b| a.1.cmp(b.1));
    edges.sort_by(|a, b| a.0.cmp(b.0));
    assert_eq!(edges, expected_edges);
    let b_idx = graph.name_table["b"];
    let c_idx = graph.name_table["c"];
    let b_c_edge = graph.graph.find_edge(b_idx, c_idx).unwrap();
    assert_eq!(graph.graph[b_c_edge], 2);
    for e in graph.graph.edge_references() {
        if e.id() == b_c_edge {
            continue;
        }
        assert_eq!(e.weight().to_owned(), 1);
    }
}

#[test]
fn test_combinations() {
    let c: Vec<(usize, usize)> = combinations_k_2(4).collect();
    let expected: Vec<(usize, usize)> = vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    assert_eq!(c.len(), expected.len());
    assert_eq!(c, expected);
}
