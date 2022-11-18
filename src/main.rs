use clap::Parser;
use itertools::sorted;
use log::{debug, error, info, trace, warn};
use petgraph::dot::{Config, Dot};
use petgraph::stable_graph::{NodeIndex, StableGraph};
use petgraph::visit::EdgeRef;
use std::fs::File;
use std::io::Write;
use std::io::{BufRead, BufReader};
use std::time::Instant;
mod graph;
mod parse_args;

/*
- value_enum for log_level
- show type of option, and not name
- read/write from file/std
- undirected graph
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

    // Read TSV into graph
    let mut graph = crate::graph::read_graph(
        args.input,
        args.header,
        args.weight_field,
        args.weight_type,
        args.weight_min,
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
        //let output = format!("{}", Dot::with_config(&graph, &[Config::NodeIndexLabel]));
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
    let mut nodes_excl = Vec::<String>::new();
    while graph.edge_count() > 0 {
        if graph.node_count() % 3000 == 0 {
            let delta_time = prev_time.elapsed();
            info!(
                "Pruned {0} nodes in {1}s ({2} nodes/s); {3} nodes remaining with {4} edges.",
                delta_n_nodes,
                delta_time.as_secs(),
                delta_n_nodes / delta_time.as_secs(),
                graph.node_count(),
                graph.edge_count()
            );
            prev_time = Instant::now();
            delta_n_nodes = 0
        }

        let (node_heavy, _node_heavy_weight) = find_heavy_node(&graph);

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

    info!("Total runtime: {}s", start_time.elapsed().as_secs());
}

fn find_heavy_node(g: &StableGraph<String, f32>) -> (NodeIndex, f32) {
    //let mut weight_max: f32 = 0.0;
    //let mut node_heavy: usize = 0;
    //for node_ix in g.node_indices() {
    //    let mut weight: f32 = 0.0;
    //    for edge in g.edges(node_ix) {
    //	    debug!("X: {:?} {:?}", g.node_weight(node_ix), g.edge_weight(edge.id()).unwrap());
    //      weight += g.edge_weight(edge.id()).unwrap();
    //    }
    //    if weight > weight_max {
    //        weight_max = weight;
    //        node_heavy = node_ix;
    //    }
    //    println!("{:?}", weight);
    //}

    let mut node_weights: Vec<(NodeIndex, f32)> = g
        .node_indices()
        .map(|node_ix| {
            (
                node_ix,
                g.edges(node_ix)
                    .map(|node_edge| -> &f32 { g.edge_weight(node_edge.id()).unwrap() })
                    .sum::<f32>(),
            )
        })
        .collect();

    node_weights.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap()
            .then(g.node_weight(a.0).cmp(&g.node_weight(b.0)))
    });

    trace!("Sorted node weights: {:?}", node_weights);
    debug!(
        "Max weight node and weight: {} {:.4}",
        g.node_weight(node_weights[0].0).unwrap(),
        node_weights[0].1
    );

    return node_weights[0];
}
