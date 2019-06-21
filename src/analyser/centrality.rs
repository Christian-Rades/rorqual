use num_traits::Zero;
use petgraph::{graph::NodeIndex, visit::EdgeRef, Graph, Undirected};
use rayon::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::{f64, iter};

type Predecessors = HashMap<NodeIndex, Vec<NodeIndex>>;

struct CB(HashMap<NodeIndex, f64>);

struct BrandesNet {
    stack: Vec<NodeIndex>,
    pred: Predecessors,
    sigma: HashMap<NodeIndex, usize>,
    start: NodeIndex,
}

impl BrandesNet {
    fn to_deltas(mut self) -> HashMap<NodeIndex, f64> {
        let mut delta: HashMap<NodeIndex, f64> =
            self.stack.iter().cloned().zip(iter::repeat(0.0)).collect();
        while let Some(w) = self.stack.pop() {
            let coeff = (1.0 + delta[&w]) / (self.sigma[&w] as f64);
            for v in self.pred.get(&w).unwrap_or(&Vec::new()) {
                let sigma_v = self.sigma[&v];
                let delta_v = delta.get_mut(&v).unwrap();
                *delta_v += (sigma_v as f64) * coeff;
            }
        }
        delta.insert(self.start, 0.0);
        delta
    }
}

pub fn betweenness_centrality<N, E>(g: &Graph<N, E, Undirected>) -> HashMap<NodeIndex, f64>
where
    N: Sync,
    E: Zero + Ord + Copy + Sync,
{
    let mut betweenness = CB::new(g);
    let indecies: Vec<NodeIndex> = g.node_indices().collect();
    let deltas: Vec<HashMap<NodeIndex, f64>> = indecies
        .par_iter()
        .map(|s| single_source_dijkstra_path(g, *s).to_deltas())
        .collect();

    for delta in deltas {
        betweenness.increment_by_deltas(delta);
    }

    let n = g.node_count();
    let scale = if n <= 2 {
        1.0
    } else {
        1.0 / (((n - 1) * (n - 2)) as f64)
    };
    for v in betweenness.inner_mut().values_mut() {
        *v *= scale;
    }

    betweenness.into_inner()
}

impl CB {
    fn new<N, E>(graph: &Graph<N, E, Undirected>) -> CB
    where
        E: Zero + Ord + Copy,
    {
        CB(graph
            .node_indices()
            .zip(iter::repeat(Zero::zero()))
            .collect())
    }

    fn accumulate_basic(&mut self, bn: BrandesNet) {
        let delta = bn.to_deltas();
        for w in delta.keys() {
            let cb_w = self.inner_mut().entry(*w).or_insert(0.0);
            *cb_w += delta[&w];
        }
    }

    fn increment_by_deltas(&mut self, deltas: HashMap<NodeIndex, f64>) {
        for w in deltas.keys() {
            let cb_w = self.inner_mut().entry(*w).or_insert(0.0);
            *cb_w += deltas[&w];
        }
    }

    fn inner_mut(&mut self) -> &mut HashMap<NodeIndex, f64> {
        let CB(i) = self;
        i
    }

    fn into_inner(self) -> HashMap<NodeIndex, f64> {
        let CB(i) = self;
        i
    }
}

//fn single_source_shortest_path<N>(
//    g: &Graph<N, f64, Undirected>,
//    s: NodeIndex,
//) -> (
//    Vec<NodeIndex>,
//    HashMap<NodeIndex, Vec<NodeIndex>>,
//    HashMap<NodeIndex, f64>,
//) {
//    let mut pred: Predecessors = g.node_indices().zip(iter::repeat_with(Vec::new)).collect();
//
//    let mut dist: HashMap<NodeIndex, f64> =
//        g.node_indices().zip(iter::repeat(f64::INFINITY)).collect();
//    let mut sigma: HashMap<NodeIndex, f64> = HashMap::new();
//
//    let mut queue: VecDeque<NodeIndex> = VecDeque::new();
//    let mut stack = Vec::new();
//
//    sigma.insert(s, 1.0);
//    queue.push_back(s);
//    *dist.get_mut(&s).unwrap() = 0.0;
//
//    while let Some(v) = queue.pop_front() {
//        stack.push(v);
//        for w in g.neighbors(v) {
//            if dist[&w] == f64::INFINITY {
//                *dist.get_mut(&w).unwrap() = dist[&v] + 1.0;
//                queue.push_back(w);
//            }
//            if dist[&w] == dist[&v] + 1.0 {
//                let sigma_v = *sigma.get(&v).unwrap();
//                let sigma_w = sigma.entry(w).or_insert(0.0);
//                *sigma_w += sigma_v;
//                pred.get_mut(&w).unwrap().push(v);
//            }
//        }
//    }
//    (stack, pred, sigma)
//}

fn single_source_dijkstra_path<N, E>(g: &Graph<N, E, Undirected>, s: NodeIndex) -> BrandesNet
where
    E: Zero + Ord + Copy,
{
    let mut pred: Predecessors = g.node_indices().zip(iter::repeat_with(Vec::new)).collect();

    let mut dist: HashMap<NodeIndex, Option<E>> =
        g.node_indices().zip(iter::repeat(None)).collect();
    let mut sigma: HashMap<NodeIndex, usize> = HashMap::new();

    let mut seen: HashMap<NodeIndex, E> = HashMap::new();
    let mut queue: VecDeque<(NodeIndex, NodeIndex, E)> = VecDeque::new();
    let mut stack = Vec::new();

    sigma.insert(s, 1);
    seen.insert(s, Zero::zero());
    queue.push_back((s, s, Zero::zero()));

    while let Some((p, v, d)) = queue.pop_front() {
        if dist[&v] != None {
            continue;
        };
        let sigma_pred = *sigma.get(&p).unwrap();
        let sigma_v = sigma.get_mut(&v).unwrap();
        *sigma_v += sigma_pred;
        stack.push(v);
        *dist.get_mut(&v).unwrap() = Some(d);
        for (w, edge_weigth) in g.edges(v).map(|e| (e.target(), e.weight())) {
            let dist_vw = d + *edge_weigth;
            if dist[&w] == None
                && seen
                    .get(&w)
                    .and_then(|seen_w| Some(dist_vw < *seen_w))
                    .unwrap_or(true)
            {
                *seen.entry(w).or_insert_with(Zero::zero) = dist_vw;
                queue.push_back((v, w, dist_vw));
                let sigma_w = sigma.entry(w).or_insert(0);
                *sigma_w = 0;
                *pred.get_mut(&w).unwrap() = vec![v];
            } else if seen
                .get(&w)
                .and_then(|seen_w| Some(dist_vw == *seen_w))
                .unwrap_or(false)
            {
                let sigma_v = *sigma.get(&v).unwrap();
                let sigma_w = sigma.entry(w).or_insert(0);
                *sigma_w += sigma_v;
                pred.get_mut(&w).unwrap().push(v);
            }
        }
    }
    BrandesNet {
        stack,
        pred,
        sigma,
        start: s,
    }
}
