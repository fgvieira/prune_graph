use flate2::read;
use std::ffi::OsStr;
use std::{
    fs::File,
    io::{stdin, BufReader, Write},
    path::{Path, PathBuf},
};

use petgraph::algo::kosaraju_scc;
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::{dot::Dot, Undirected};

use tracing::{info, warn};

use crate::graph;

pub fn graph_load(
    input: Option<PathBuf>,
    has_header: bool,
    weight_field: String,
    weight_filter: Option<String>,
    weight_n_edges: bool,
    weight_precision: u8,
) -> graph::GraphX {
    if let Some(_input) = input {
        let fh = File::open(&_input).expect("cannot open input file");
        if Path::new(&_input).extension() == Some(OsStr::new("gz")) {
            info!("Reading input Gzip file {:?}", &_input);
            let reader_file_gz = BufReader::with_capacity(128 * 1024, read::GzDecoder::new(fh));
            crate::graph::graph_read(
                reader_file_gz,
                has_header,
                weight_field,
                weight_filter,
                weight_n_edges,
                weight_precision,
            )
        } else {
            info!("Reading input file {:?}", &_input);
            let reader_file = BufReader::new(fh);
            crate::graph::graph_read(
                reader_file,
                has_header,
                weight_field,
                weight_filter,
                weight_n_edges,
                weight_precision,
            )
        }
    } else {
        info!("Reading from STDIN");
        let reader_stdin = stdin().lock();
        crate::graph::graph_read(
            reader_stdin,
            has_header,
            weight_field,
            weight_filter,
            weight_n_edges,
            weight_precision,
        )
    }
}

pub fn graph_save_dot(
    graph: &StableGraph<String, f32, Undirected, graph::GraphIdx>,
    out_graph: Option<PathBuf>,
) {
    if let Some(_out_graph) = out_graph {
        info!("Saving graph as dot");
        if graph.node_count() > 10000 {
            warn!("Plotting graphs with more than 10000 nodes can be slow and not very informative")
        }
        let mut out_graph = File::create(_out_graph).expect("cannot open graph file!");
        let output = format!("{}", Dot::new(&graph));
        out_graph
            .write_all(output.as_bytes())
            .expect("cannot write to graph file!");
    }
}

pub fn graph_save_components(
    graph: &StableGraph<String, f32, Undirected>,
    out_comps: PathBuf,
    has_header: bool,
) {
    let init_comps = kosaraju_scc(graph);
    let comps_file = File::create(&out_comps).expect("Cannot create components file!");
    if Path::new(&out_comps).extension() == Some(OsStr::new("jsonl")) {
        graph_save_components_jsonl(graph, init_comps, comps_file)
    } else {
        graph_save_components_tsv(graph, init_comps, comps_file, has_header)
    }
}

pub fn graph_save_components_jsonl(
    graph: &StableGraph<String, f32, Undirected>,
    init_comps: Vec<Vec<NodeIndex>>,
    mut comps_file: File,
) {
    info!("Writing {} component(s) to JSONL file", init_comps.len());
    for comp in init_comps.iter() {
        comps_file
            .write_all(b"[\"")
            .expect("Cannot write opening bracket to components file.");
        comps_file
            .write_all(
                comp.iter()
                    .map(|x| {
                        graph
                            .node_weight(*x)
                            .expect("Cannot find node in graph.")
                            .to_string()
                    })
                    .collect::<Vec<String>>()
                    .join("\", \"")
                    .as_bytes(),
            )
            .expect("Cannot write component file.");
        comps_file
            .write_all(b"\"]\n")
            .expect("Cannot write closing bracket to components file.");
    }
}

pub fn graph_save_components_tsv(
    graph: &StableGraph<String, f32, Undirected>,
    init_comps: Vec<Vec<NodeIndex>>,
    mut comps_file: File,
    has_header: bool,
) {
    info!("Writing {} component(s) to TSV file", init_comps.len());
    // Print header
    if has_header {
        comps_file
            .write_all(b"node1\tnode2\tweight\tcomponent\n")
            .expect("Cannot write header to components file.");
    }
    // Print components
    for (idx, comp) in init_comps.iter().enumerate() {
        let mut node_list = Vec::new();
        for source in comp.iter() {
            for target in graph.neighbors(*source) {
                if node_list.contains(&target) {
                    continue;
                }
                let edge = graph.find_edge(*source, target).unwrap();
                comps_file
                    .write_all(
                        format!(
                            "{}\t{}\t{}\t{}\n",
                            graph.node_weight(*source).unwrap(),
                            graph.node_weight(target).unwrap(),
                            graph.edge_weight(edge).unwrap(),
                            idx,
                        )
                        .as_bytes(),
                    )
                    .expect("Cannot write to components output file.");
            }
            node_list.push(*source);
        }
    }
}
