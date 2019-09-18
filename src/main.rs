mod git2graph;
mod analyser;

use petgraph::graph::NodeIndex;
use petgraph::dot::{Dot, Config};
use petgraph_graphml::GraphMl;
use clap::{App, Arg};
use std::{env, fs::File, io::Write};


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

    let mut foldername: String = repo_path
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap()
        .to_owned();
    foldername.push_str(".graphml");

    let out = File::create(env::current_dir()?.join(foldername))?;

    let mut commit_graph = git2graph::repo_to_graph(repo_path);

    let mut bc = analyser::centrality::betweenness_centrality(&commit_graph);
    for (vertex, betweenness) in bc.into_iter() {
        if let Some(hm) = commit_graph.node_weight_mut(vertex) {
            hm.insert("centrality".into(), format!("{:.6}", betweenness));
        }
        else {
            commit_graph.remove_node(vertex);
        }
    }
    let stuff = GraphMl::new(&commit_graph).pretty_print(true).export_node_weights(
        Box::new(|node| {
            let name = node["file"].split("/").last().unwrap();
            let last_slash = node["file"].rfind('/').unwrap_or(0);
            let (module, _) = node["file"].split_at(last_slash);
            let centrality = &node["centrality"];
            vec![
                ("name".into(), name.into()),
                ("module".into(), module.into()),
                ("centrality".into(), centrality.into())
            ]
        })
    )
    .export_edge_weights_display();
    stuff.to_writer(out)?;

    Ok(())
}
