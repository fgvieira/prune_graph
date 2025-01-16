use flate2::read;
use std::ffi::OsStr;
use std::path::Path;

use clap::Parser;
use flexi_logger::AdaptiveFormat;
use itertools::sorted;
use log::{error, info, trace, warn};
use petgraph::{algo::kosaraju_scc, dot::Dot};
use rayon::{prelude::*, ThreadPoolBuilder};
use std::{
    fs::File,
    io::{stdin, stdout, BufReader, Write},
    time::Instant,
};
mod graph;
mod parse_args;

fn main() {
    let start_time = Instant::now();
    // Parse command-line arguments
    let args = parse_args::Args::parse();

    // Initialize logger
    let log_level = if args.quiet {
        log::LevelFilter::Off
    } else {
        match args.verbose {
            0 => log::LevelFilter::Warn,
            1 => log::LevelFilter::Info,
            2 => log::LevelFilter::Debug,
            _ => log::LevelFilter::Trace,
        }
    };

    flexi_logger::Logger::try_with_str(log_level.as_str().to_lowercase())
        .expect("cannot initialize logger")
        .adaptive_format_for_stderr(AdaptiveFormat::WithThread)
        .start()
        .expect("cannot start logger");

    let version = env!("CARGO_PKG_VERSION");
    info!("prune_graph v{version}");

    // Create threadpool
    if args.n_threads > 20 {
        warn!("High number of threads is only relevant for very large graphs. For must uses, less than 10 threads is usually enough.");
    }

    ThreadPoolBuilder::new()
        .num_threads(args.n_threads)
        .build_global()
        .expect("cannot create threadpool");

    // Read TSV into graph
    let (mut graph, _graph_idx) = if args.input.is_some() {
        let fh = File::open(args.input.as_ref().unwrap()).expect("cannot open input file");
        if Path::new(&args.input.as_ref().unwrap()).extension() == Some(OsStr::new("gz")) {
            info!("Reading input Gzip file {:?}", &args.input.unwrap());
            let reader_file_gz = BufReader::with_capacity(128 * 1024, read::GzDecoder::new(fh));
            crate::graph::graph_read(
                reader_file_gz,
                args.header,
                args.weight_field,
                args.weight_filter,
                args.weight_n_edges,
                args.weight_precision,
            )
        } else {
            info!("Reading input file {:?}", &args.input.unwrap());
            let reader_file = BufReader::new(fh);
            crate::graph::graph_read(
                reader_file,
                args.header,
                args.weight_field,
                args.weight_filter,
                args.weight_n_edges,
                args.weight_precision,
            )
        }
    } else {
        info!("Reading from STDIN");
        let reader_stdin = stdin().lock();
        crate::graph::graph_read(
            reader_stdin,
            args.header,
            args.weight_field,
            args.weight_filter,
            args.weight_n_edges,
            args.weight_precision,
        )
    };

    // Open subset file
    if args.subset.is_some() {
        info!("Subsetting nodes based on input file");
        crate::graph::graph_subset(&mut graph, args.subset.expect("invalid subset option"));
    }
    info!(
        "Graph has {0} nodes with {1} edges ({2} components)",
        graph.node_count(),
        graph.edge_count(),
        kosaraju_scc(&graph).len(),
    );

    if graph.node_count() == 0 {
        error!("Graph is empty");
        std::process::exit(1);
    }

    // Print graph
    if args.out_graph.is_some() {
        info!("Saving graph as dot");
        if graph.node_count() > 10000 {
            warn!("Plotting graphs with more than 10000 nodes can be slow and not very informative")
        }
        let mut out_graph = File::create(args.out_graph.unwrap()).expect("cannot open graph file!");
        let output = format!("{}", Dot::new(&graph));
        out_graph
            .write_all(output.as_bytes())
            .expect("cannot write to graph file!");
    }

    if args.keep_heavy {
        info!(
            "Pruning neighbors of heaviest position ({} threads)",
            args.n_threads
        );
    } else {
        info!("Pruning heaviest position ({} threads)", args.n_threads);
    }

    let mut n_iters = 0;
    let mut prev_time = Instant::now();
    let mut delta_n_nodes = 0;
    // Store deleted nodes
    let mut nodes_excl = Vec::<String>::new();
    while graph.edge_count() > 0 {
        // Find heaviest nodes
        let nodes_heavy: Vec<(petgraph::stable_graph::NodeIndex, f32)> = if args.mode == 1 {
            kosaraju_scc(&graph)
                .par_iter()
                .filter(|x| x.len() > 1)
                .map(|x| crate::graph::find_heaviest_node(&graph, Some(x)))
                .collect()
        } else {
            vec![crate::graph::find_heaviest_node(&graph, None)]
        };
        trace!("{:?}", nodes_heavy);

        // Process heaviest node
        for (node_heavy, _node_heavy_weight) in &nodes_heavy {
            if args.keep_heavy {
                let mut nodes_del = graph.neighbors_undirected(*node_heavy).detach();
                while let Some(node_neighb) = nodes_del.next_node(&graph) {
                    nodes_excl.push(graph.node_weight(node_neighb).unwrap().to_string());
                    graph.remove_node(node_neighb);
                    delta_n_nodes += 1;
                }
            } else {
                nodes_excl.push(graph.node_weight(*node_heavy).unwrap().to_string());
                graph.remove_node(*node_heavy);
                delta_n_nodes += 1;
            }
        }
        n_iters += 1;

        // Report progress
        if n_iters % 100 == 0 {
            let delta_time = prev_time.elapsed();
            info!(
                "Pruned {0} nodes in {1}s ({2:.2} nodes/s); {3} nodes remaining with {4} edges ({5} components)",
                delta_n_nodes,
                delta_time.as_secs(),
                delta_n_nodes as f32 / delta_time.as_secs_f32(),
                graph.node_count(),
                graph.edge_count(),
		nodes_heavy.len(),
            );
            prev_time = Instant::now();
            delta_n_nodes = 0
        }
    }

    info!(
        "Pruning complete in {0} iterations! Final graph has {1} nodes with {2} edges",
        n_iters,
        graph.node_count(),
        graph.edge_count()
    );

    info!("Saving remaining nodes");
    if args.out.is_some() {
        let mut writer_file = File::create(args.out.unwrap()).expect("cannot open output file");
        write(&mut writer_file, &mut graph.node_weights())
            .expect("cannot write results to output file");
    } else {
        write(&mut stdout().lock(), &mut graph.node_weights())
            .expect("cannot write results to stdout");
    }

    if args.out_excl.is_some() {
        info!("Saving excluded nodes to file");
        let mut writer_file = File::create(args.out_excl.unwrap())
            .expect("cannot open output file for excluded nodes");
        write(&mut writer_file, &mut sorted(nodes_excl))
            .expect("cannot write excluded nodes to file");
    }

    info!(
        "Total runtime: {:.2} mins",
        start_time.elapsed().as_secs() as f32 / 60.0
    );
}

fn write<W, T>(writer: &mut W, vec: &mut T) -> std::io::Result<()>
where
    W: std::io::Write,
    T: std::iter::Iterator,
    <T as Iterator>::Item: std::fmt::Display,
    <T as Iterator>::Item: Ord,
{
    for item in sorted(vec) {
        writeln!(writer, "{item}")?;
    }

    Ok(())
}
