mod analyser;
mod git2graph;
mod git_graph;

use chrono::{
    format::{self, Parsed, StrftimeItems},
    Utc,
};
use chrono_tz::UTC;
use clap::{App, Arg};
use git2graph::GitFilter;
use git_graph::GitGraph;
use petgraph::{graph::NodeIndex, visit::IntoNeighbors};
use regex::RegexSet;
use std::{
    collections::{HashSet, VecDeque},
    env,
    fs::File,
    io::{stdout, Write},
};

fn main() -> std::io::Result<()> {
    let matches = App::new("Rorqual")
        .version("0.1")
        .author("CR")
        .about("Graph analysis for git repos")
        .arg(Arg::with_name("debug").short("d").long("debug"))
        .arg(
            Arg::with_name("repo")
                .long("repo")
                .takes_value(true)
                .help("path to the repo"),
        )
        .arg(
            Arg::with_name("start_time")
                .long("start-time")
                .takes_value(true)
                .help("time of eraliest commits"),
        )
        .arg(
            Arg::with_name("filter")
                .long("filter")
                .takes_value(true)
                .help("regex of path to ignore"),
        )
        .arg(
            Arg::with_name("report")
                .long("report")
                .help("prints a report of the analysis"),
        )
        .arg(
            Arg::with_name("neighbours")
                .long("neighbours")
                .takes_value(true)
                .help("paths to get neighbourhood of"),
        )
        .get_matches();

    let repo_path = if let Some(rel_path) = matches.value_of("repo") {
        env::current_dir()?.join(rel_path)
    } else {
        env::current_dir()?
    };

    let mut p = Parsed::default();

    format::parse(
        &mut p,
        matches.value_of("start_time").unwrap(),
        StrftimeItems::new("%Y-%m-%d"),
    )
    .unwrap();
    p.hour_mod_12 = Some(0);
    p.hour_div_12 = Some(0);
    p.minute = Some(0);
    p.second = Some(0);

    let path_filters = if let Some(filters) = matches.values_of("filter") {
        RegexSet::new(filters.collect::<Vec<&str>>()).unwrap()
    } else {
        RegexSet::empty()
    };

    let filter = GitFilter {
        start_date: Some(p.to_datetime_with_timezone(&Utc).unwrap()),
        max_commits: Some(40),
        path_filters: path_filters,
    };

    let changes = git2graph::repo_to_changesets(repo_path, &filter);
    let mut graph = git_graph::build_graph(changes);

    // Hack!! replace int weight with aliases: Distance and Similarity
    // Both are ints but Similarity has the Ord impl reversed
    let max_count = *graph.graph.edge_weights().max().unwrap();
    for count in graph.graph.edge_weights_mut() {
        *count = max_count - *count;
    }

    if matches.is_present("report") {
        bc_report(&graph);
    }

    if let Some(nb) = matches.value_of("neighbours") {
        println!("graph {{");
        let current = graph.name_table[nb];
        write_node(current, &graph, &mut stdout());
        for nb in graph.graph.neighbors(current) {
            write_node(nb, &graph, &mut stdout());
            write_edge(current, nb, &graph, &mut stdout());
        }
        println!("}}");
    }

    Ok(())
}

fn write_node<W: Write>(idx: NodeIndex, graph: &GitGraph, writer: &mut W) {
    let name = &graph.graph[idx].name;
    writeln!(
        writer,
        "\"{:?}\" [label=\"{}\" fixedsize=true fontsize=7]",
        idx, name
    )
    .unwrap();
}

fn write_edge<W: Write>(from: NodeIndex, to: NodeIndex, graph: &GitGraph, writer: &mut W) {
    let edge_idx = graph.graph.find_edge(from, to).unwrap();
    writeln!(
        writer,
        "\"{:?}\" -- \"{:?}\" [weight={}]",
        from, to, graph.graph[edge_idx]
    );
}

fn recurse_neighbors(idx: NodeIndex, graph: &GitGraph, depth: u64) -> HashSet<NodeIndex> {
    if depth == 0 {
        HashSet::default()
    } else {
        let mut result: HashSet<NodeIndex> = graph
            .graph
            .neighbors(idx)
            .flat_map(|n| recurse_neighbors(n, graph, depth - 1))
            .collect();
        result.insert(idx);
        return result;
    }
}

fn bc_report(graph: &GitGraph) {
    let mut bc = analyser::centrality::betweenness_centrality(&graph.graph);

    for (vertex, betweenness) in bc.into_iter() {
        if let Some(git_file) = graph.graph.node_weight(vertex) {
            println!("\"{}\",{:.6}", git_file.name, betweenness);
        }
    }

    eprintln!(
        "Total nodes: {} edges: {}",
        graph.graph.node_count(),
        graph.graph.edge_count()
    );
}
