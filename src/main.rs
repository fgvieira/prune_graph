use clap::Parser;
use itertools::sorted;
use log::{debug, error, info, trace, warn};
use petgraph::algo::tarjan_scc;
use petgraph::dot::Dot;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::time::Instant;
mod graph;
mod parse_args;

/*
- ARGS: value_enum for log_level
- ARGS: show type of option, and not name
- read/write from file/std
- GRAPH: undirected graph
- write vector to file
*/
fn main() {
    let start_time = Instant::now();
    // Parse command-line arguments
    let args = parse_args::Args::parse();

    // Initialize logger
    flexi_logger::Logger::try_with_str(args.log_level.as_str().to_lowercase())
        .expect("cannot initialize logger")
        .format_for_stderr(crate::parse_args::log_format)
        .start()
        .expect("cannot start logger");

    // Create threadpool
    if args.n_threads > 5 {
        warn!("High number of threads only make sense for HUGE graphs. For must uses 2/3 threads are enough.");
    }
    ThreadPoolBuilder::new()
        .num_threads(args.n_threads)
        .build_global()
        .expect("cannot create threadpool");

    // Read TSV into graph
    info!("Creating graph...");
    let (mut graph, _graph_idx) = crate::graph::read_graph(
        args.input,
        args.header,
        args.weight_field,
        args.weight_filter,
        args.weight_n_edges,
        args.weight_precision,
    );

    // Open subset file
    if args.subset.is_some() {
        info!("Subsetting nodes based on input file...");
        let mut nodes_subset = Vec::<String>::new();
        let in_reader = BufReader::new(
            File::open(args.subset.as_ref().expect("invalid subset option"))
                .expect("cannot open subset file"),
        );
        for node in in_reader.lines() {
            nodes_subset.push(node.expect("cannot read node from subset file"));
        }
        debug!("Nodes to include: {:?}", nodes_subset);

        graph.retain_nodes(|g, ix| nodes_subset.contains(&g[ix]));
        info!(
            "Graph has {0} nodes with {1} edges",
            graph.node_count(),
            graph.edge_count()
        );
    }

    if graph.node_count() == 0 {
        error!("Graph is empty!");
        std::process::exit(1);
    }

    // Print graph
    if args.out_graph.is_some() {
        info!("Saving graph as dot...");
        if graph.node_count() > 10000 {
            warn!("Plotting graphs with more than 10000 nodes can be slow and not very informative")
        }
        let mut out_graph =
            std::fs::File::create(args.out_graph.unwrap()).expect("cannot open graph file!");
        let output = format!("{}", Dot::new(&graph));
        out_graph
            .write_all(&output.as_bytes())
            .expect("cannot write to graph file!");
    }

    if args.keep_heavy {
        info!("Pruning neighbors of heaviest position...");
    } else {
        info!("Pruning heaviest position...");
    }

    let mut prev_time = Instant::now();
    let mut delta_n_nodes = 0;
    // Store deleted nodes
    let mut nodes_excl = Vec::<String>::new();
    while graph.edge_count() > 0 {
        if graph.node_count() % 1000 == 0 {
            let delta_time = prev_time.elapsed();
            info!(
                "Pruned {0} nodes in {1}s ({2:.2} nodes/s); {3} nodes remaining with {4} edges.",
                delta_n_nodes,
                delta_time.as_secs(),
                delta_n_nodes as f32 / delta_time.as_secs() as f32,
                graph.node_count(),
                graph.edge_count()
            );
            prev_time = Instant::now();
            delta_n_nodes = 0
        }

        let nodes_heavy: Vec<(petgraph::stable_graph::NodeIndex, f32)> = tarjan_scc(&graph)
            .par_iter()
            .filter(|x| x.len() > 1)
            .map(|x| crate::graph::find_heaviest_node(&graph, Some(x)))
            .collect();
        trace!("{:?}", nodes_heavy);

        for (node_heavy, _node_heavy_weight) in nodes_heavy {
            if args.keep_heavy {
                let mut nodes_del = graph.neighbors_undirected(node_heavy).detach();
                while let Some(node_neighb) = nodes_del.next_node(&graph) {
                    nodes_excl.push(graph.node_weight(node_neighb).unwrap().to_string());
                    graph.remove_node(node_neighb);
                    delta_n_nodes += 1;
                }
            } else {
                nodes_excl.push(graph.node_weight(node_heavy).unwrap().to_string());
                graph.remove_node(node_heavy);
                delta_n_nodes += 1;
            }
        }
    }
    info!(
        "Pruning complete! Pruned graph has {0} nodes with {1} edges",
        graph.node_count(),
        graph.edge_count()
    );

    info!("Saving remaining nodes...");
    let mut out_fh = File::create(args.out).expect("cannot open output file");
    //out_fh.write(graph.node_weights().map(|x| -> String {x.push('\n')}).collect().concat().as_bytes()).unwrap();
    //out_fh.write(graph.node_weights().cloned().intersperse("\n".to_string()).collect::<String>().as_bytes()).unwrap();
    for node_weight in sorted(graph.node_weights()) {
        let mut x: String = node_weight.to_string();
        x.push('\n');
        out_fh
            .write(x.as_bytes())
            .expect("cannot write to output file");
    }

    if args.out_excl.is_some() {
        info!("Saving prunned nodes to file...");
        let mut out_fh = File::create(args.out_excl.unwrap())
            .expect("cannot open output file for excluded nodes");
        for node in sorted(nodes_excl) {
            let mut x: String = node.to_string();
            x.push('\n');
            out_fh
                .write(x.as_bytes())
                .expect("cannot write excluded nodes to file");
        }
    }

    info!(
        "Total runtime: {:.2} mins",
        start_time.elapsed().as_secs() as f32 / 60.0
    );
}
