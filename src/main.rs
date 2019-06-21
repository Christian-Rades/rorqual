mod git2graph;
mod analyser;

use petgraph::graph::NodeIndex;
use clap::{App, Arg};
use std::{env};


fn main() -> std::io::Result<()> {
    let matches = App::new("Rorqual")
        .version("0.1")
        .author("CR")
        .about("Graph analysis for git repos")
        .arg(Arg::with_name("debug").short("d").long("debug"))
        .arg(Arg::with_name("repo").help("path to the repo"))
        .get_matches();

    let repo_path = if let Some(rel_path) = matches.value_of("repo") {
        env::current_dir()?.join(rel_path)
    } else {
        env::current_dir()?
    };

    let commit_graph = git2graph::repo_to_graph(repo_path);

    //    output_graph(&commit_graph);

    let mut bc: Vec<(NodeIndex, f64)> = analyser::centrality::betweenness_centrality(&commit_graph).into_iter().collect();
    bc.sort_by(|(_, ba), (_, bb)| ba.partial_cmp(bb).unwrap());
    bc.reverse();

    for (vertex, betweenness) in bc {
        if betweenness == 0.0 {
            continue;
        }
        println!(
            "{} -> {1:.6}",
            commit_graph.node_weight(vertex).unwrap(),
            betweenness
        );
    }
    Ok(())
}

