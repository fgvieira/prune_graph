use clap::Parser;
use flexi_logger::AdaptiveFormat;
use itertools::sorted;
use log::{error, info, trace, warn};
use petgraph::dot::Dot;
use rayon::ThreadPoolBuilder;
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
    if args.n_threads > 5 {
        warn!("High number of threads is only relevant for very large graphs. For must uses, 2/3 threads are usually enough.");
    }
    ThreadPoolBuilder::new()
        .num_threads(args.n_threads)
        .build_global()
        .expect("cannot create threadpool");

    // Read TSV into graph
    info!("Creating graph...");
    let (mut graph, _graph_idx) = if args.input.is_some() {
        info!("Reading from input file...");
        let reader_file =
            BufReader::new(File::open(args.input.unwrap()).expect("cannot open input file"));
        crate::graph::graph_read(
            reader_file,
            args.header,
            args.weight_field,
            args.weight_filter,
            args.weight_n_edges,
            args.weight_precision,
        )
    } else {
        info!("Reading from STDIN...");
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
        info!("Subsetting nodes based on input file...");
        crate::graph::graph_subset(&mut graph, args.subset.expect("invalid subset option"));

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
        let mut out_graph = File::create(args.out_graph.unwrap()).expect("cannot open graph file!");
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
        // Report progress
        if prev_time.elapsed().as_secs() >= 30 && delta_n_nodes != 0 {
            let delta_time = prev_time.elapsed();
            info!(
                "Pruned {0} nodes in {1}s ({2:.2} nodes/s); {3} nodes remaining with {4} edges.",
                delta_n_nodes,
                delta_time.as_secs(),
                delta_n_nodes as f32 / delta_time.as_secs_f32(),
                graph.node_count(),
                graph.edge_count()
            );
            prev_time = Instant::now();
            delta_n_nodes = 0
        }

        // Find heaviest nodes
        let (node_heavy, _node_heavy_weight) = crate::graph::find_heaviest_node(&graph);
        trace!("{:?}", node_heavy);

        // Process heaviest node
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
        "Pruning complete! Final graph has {0} nodes with {1} edges",
        graph.node_count(),
        graph.edge_count()
    );

    info!("Saving remaining nodes...");
    if args.out.is_some() {
        let mut writer_file = File::create(args.out.unwrap()).expect("cannot open output file");
        write(&mut writer_file, &mut graph.node_weights())
            .expect("cannot write results to output file");
    } else {
        write(&mut stdout().lock(), &mut graph.node_weights())
            .expect("cannot write results to stdout");
    }

    if args.out_excl.is_some() {
        info!("Saving excluded nodes to file...");
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
