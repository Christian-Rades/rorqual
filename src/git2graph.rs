use git2::{Delta, Repository, Sort};
use petgraph::{graph::NodeIndex, Graph, Undirected};
use chrono::format::{self, strftime::StrftimeItems, Parsed};
use chrono::{Utc, DateTime};

use std::collections::{HashMap, HashSet};


pub struct GitFilter {
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    max_commits: Option<u32>,
    path_filters: Vec<String>,
}

pub fn repo_to_graph(path: std::path::PathBuf) -> Graph<HashMap<String, String>, i64, Undirected> {
    let mut files: HashMap<String, NodeIndex> = HashMap::new();
    let mut edges: HashMap<(NodeIndex, NodeIndex), i64> = HashMap::new();
    let mut graph = Graph::<HashMap<String, String>, i64, Undirected>::new_undirected();

    let repo = Repository::open(path).unwrap();
    let commit_trees = search_repo(&repo).unwrap();
    let commit_trees: Vec<git2::Tree> = commit_trees.into_iter().rev().collect();

    let diffs = commit_trees.windows(2).flat_map(|window| match window {
        [old, new] => repo.diff_tree_to_tree(Some(old), Some(new), None).ok(),
        _ => None,
    });

    let mut deleted: HashSet<String> = HashSet::new();

    for d in diffs {
        let file_count = d.deltas().count();
        if file_count > 40 {
            continue;
        }
        let mut commit_files = Vec::with_capacity(file_count);
        for delta in d.deltas() {
            match delta.status() {
                Delta::Added | Delta::Modified | Delta::Renamed => {
                    let name = delta
                        .old_file()
                        .path()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    let node = files
                        .entry(name.clone())
                        .or_insert_with(
                            || {
                                let mut hm = HashMap::new();
                                hm.insert("file".into(), name);
                                graph.add_node(hm)
                            });
                    commit_files.push(node.clone());
                }
                Delta::Deleted => {
                    let name = delta
                        .old_file()
                        .path()
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    deleted.insert(name);
                }
                _ => (),
            }
        }
        let commit_size = commit_files.len();
        for (a, b) in combinations_k_2(commit_size) {
            let edge_count = edges
                .entry((commit_files[a].to_owned(), commit_files[b].to_owned()))
                .or_insert(0);
            *edge_count += 1;
        }
    }
    let max_count = *edges.iter().map(|(_, c)| c).max().unwrap();

    for ((a, b), count) in edges {
        let inv_weight =  max_count - count;
        graph.add_edge(a, b, inv_weight);
    }

    graph.retain_nodes(|g, node_i| !deleted.contains(&g[node_i]["file"]));
    graph
}

fn search_repo(repo: &Repository) -> Result<Vec<git2::Tree>, git2::Error> {
    let mut rev_walk = repo.revwalk()?;
    rev_walk.set_sorting(Sort::NONE);
    rev_walk.push_head()?;

    let mut p = Parsed::default();

    format::parse(&mut p, "2018-01-01", StrftimeItems::new("%Y-%m-%d")).unwrap();
    p.hour_mod_12 = Some(0);
    p.hour_div_12 = Some(0);
    p.minute = Some(0);
    p.second = Some(0);
    let dt = p
        .to_datetime_with_timezone(&chrono_tz::Europe::Berlin)
        .unwrap();

    let commit_trees: Vec<git2::Tree> = rev_walk
        .flat_map(|commit_id| repo.find_commit(commit_id.unwrap()))
        .take_while(|commit| commit.time().seconds() > dt.naive_utc().timestamp())
//        .filter(|commit| commit.message().and_then(|msg: &str| Some(msg.contains("Merge pull request"))).unwrap_or(false))
        .flat_map(|commit| commit.tree())
        .collect();
    Ok(commit_trees)
}

fn combinations_k_2(n: usize) -> Vec<(usize, usize)> {
    (0..n)
        .map(|x| (x..n).filter_map(move |y| if x < y { Some((x, y)) } else { None }))
        .flatten()
        .collect()
}

#[test]
fn test_combinations() {
    let c = combinations_k_2(4);
    let expected: Vec<(usize, usize)> = vec![(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)];
    assert_eq!(c.len(), expected.len());
    assert_eq!(c, expected);
}
