use clap::Parser;
use indicatif::ProgressStyle;
use itertools::sorted;
use petgraph::algo::kosaraju_scc;
use rayon::{prelude::*, ThreadPoolBuilder};

use std::{fs::File, io::stdout, time::Instant};

use tracing::{debug, enabled, error, info, info_span, trace, warn, Level};
mod graph;
mod io;
mod parse_args;
use tracing_indicatif::span_ext::IndicatifSpanExt;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::{fmt, prelude::*};

fn main() {
    let start_time = Instant::now();
    // Parse command-line arguments
    let args = parse_args::Args::parse();

    // Initialize logger
    let log_level = if args.quiet {
        LevelFilter::OFF
    } else {
        match args.verbose {
            0 => LevelFilter::WARN,
            1 => LevelFilter::INFO,
            2 => LevelFilter::DEBUG,
            _ => LevelFilter::TRACE,
        }
    };

    // Define format layer
    let indicatif_layer = IndicatifLayer::new();
    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_timer(fmt::time::ChronoLocal::rfc_3339())
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_target(true)
        .with_line_number(true)
        .with_writer(indicatif_layer.get_stderr_writer());

    // Register subscriber
    tracing_subscriber::registry()
        .with(log_level)
        .with(indicatif_layer)
        .with(fmt_layer)
        .try_init()
        .expect("cannot register tracing subscriber");

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
    let (mut graph, _graph_idx) = io::graph_load(
        args.input,
        args.header,
        args.weight_field,
        args.weight_filter,
        args.weight_n_edges,
        args.weight_precision,
    );

    // Open subset file
    if let Some(_subset) = args.subset {
        info!("Subsetting nodes based on input file");
        crate::graph::graph_subset(&mut graph, _subset);
    }

    if graph.node_count() == 0 {
        error!("Graph is empty");
        std::process::exit(1);
    }

    info!(
        "Graph has {0} nodes with {1} edges [{2} component(s)]",
        graph.node_count(),
        graph.edge_count(),
        kosaraju_scc(&graph).len(),
    );

    // Saving components to file
    if let Some(_out_comps) = args.out_comps {
        io::graph_save_components(&graph, _out_comps, args.header);
    }

    // Save graph to file
    io::graph_save_dot(&graph, args.out_graph);

    let mut n_iters = 0;
    let mut delta_n_nodes = 0;
    let mut delta_n_edges = graph.edge_count() as u64;
    // Initialize progress bar
    let prune_span = info_span!("prune");
    prune_span.pb_set_length(delta_n_edges);
    prune_span.pb_set_style(
        &ProgressStyle::with_template(
            "{bar:50} {pos:>10}/{len} edges pruned in {elapsed} ({per_sec:>0}) {msg}",
        )
        .unwrap(),
    );
    let prune_span_enter = prune_span.enter();

    // Start graph pruning
    if args.keep_heavy {
        info!(
            "Pruning neighbors of heaviest position ({} threads)",
            args.n_threads
        );
    } else {
        info!("Pruning heaviest position ({} threads)", args.n_threads);
    }

    // Store deleted nodes
    let mut nodes_excl = Vec::<String>::new();
    while graph.edge_count() > 0 {
        // Find heaviest nodes
        let nodes_heavy = if args.mode == 1 {
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

        // Update progress bar
        prune_span.pb_inc(delta_n_edges - graph.edge_count() as u64);
        delta_n_edges = graph.edge_count() as u64;
        if enabled!(Level::DEBUG) {
            prune_span.pb_set_message(&format!(
                "from {0} nodes ({1:.2}/s) [{2} iters.]",
                delta_n_nodes,
                delta_n_nodes as f32 / prune_span.pb_elapsed().as_secs_f32(),
                n_iters,
            ));
        }
    }
    std::mem::drop(prune_span_enter);
    std::mem::drop(prune_span);

    info!("Pruning complete!");
    debug!(
        "Final graph has {0} nodes with {1} edges",
        graph.node_count(),
        graph.edge_count()
    );

    info!("Saving remaining nodes");
    if let Some(_out) = args.out {
        let mut writer_file = File::create(_out).expect("cannot open output file");
        write(&mut writer_file, &mut graph.node_weights())
            .expect("cannot write results to output file");
    } else {
        write(&mut stdout().lock(), &mut graph.node_weights())
            .expect("cannot write results to stdout");
    }

    if let Some(_out_excl) = args.out_excl {
        info!("Saving excluded nodes to file");
        let mut writer_file =
            File::create(_out_excl).expect("cannot open output file for excluded nodes");
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
