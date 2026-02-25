use indicatif::ProgressStyle;
use petgraph::{
    stable_graph::{NodeIndex, StableGraph},
    Undirected,
};
use rayon::iter::IntoParallelRefIterator;
use rayon::iter::ParallelIterator;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
};
use tracing::{debug, enabled, error, info_span, trace, warn, Level};
use tracing_indicatif::span_ext::IndicatifSpanExt;
#[cfg(not(feature = "large_graph"))]
type GraphIdx = u32;
#[cfg(feature = "large_graph")]
type GraphIdx = usize;

pub fn graph_read<R: BufRead>(
    reader: R,
    has_header: bool,
    weight_field: String,
    weight_filter: Option<String>,
    weight_n_edges: bool,
    weight_precision: u8,
) -> (
    StableGraph<String, f32, Undirected, GraphIdx>,
    HashMap<String, NodeIndex<GraphIdx>>,
) {
    // Create graph
    let mut graph = StableGraph::<String, f32, Undirected, GraphIdx>::default();
    debug!(
        "Creating graph with GraphIdx = {}",
        std::any::type_name::<GraphIdx>()
    );
    let mut graph_idx = HashMap::new();

    // Initialize span and progress bar
    let graph_span = info_span!("graph");
    graph_span.pb_set_style(
        &ProgressStyle::with_template(
            "{spinner}: Read {pos} edges in {elapsed} ({per_sec:>0}) {msg}",
        )
        .unwrap()
        .tick_chars("||//--\\\\"),
    );
    let graph_span_enter = graph_span.enter();

    // Read the file line by line
    let mut header: Vec<String> = Vec::new();
    let mut n_lines: usize = 0;
    for (index, line) in reader.lines().enumerate() {
        let line = line.expect("cannot read line from input file");
        // Update progress bar
        graph_span.pb_inc(1);
        if enabled!(Level::DEBUG) {
            graph_span.pb_set_message(&*format!(
                "for graph with {0} nodes and {1} edges",
                graph.node_count(),
                graph.edge_count()
            ));
        }

        //let edge: Vec<&str> = line.split('\t').collect();
        let edge: Vec<String> = line.split('\t').map(str::to_string).collect();

        // Define header
        if index == 0 {
            header = if has_header {
                edge.clone()
            } else {
                (1..edge.len() + 1)
                    .map(|h| format!("column_{}", h))
                    .collect()
            };
            debug!("HEADER = {:?}", header);
            if !header.iter().any(|h| h == &weight_field) {
                error!("weight_field '{weight_field}' is not present in the header");
                std::process::exit(-1);
            }
            if has_header {
                continue;
            }
        }
        n_lines += 1;

        // Check number of fields
        if edge.len() != header.len() {
            error!(
                "edge {0} has {1} fields, while header has {2}",
                n_lines,
                edge.len(),
                header.len()
            );
            std::process::exit(-1);
        }

        // Check if nodes exist and add them if not
        // Node label is stored as its "weight"
        if !graph_idx.contains_key(&edge[0]) {
            graph_idx.insert(edge[0].clone(), graph.add_node(edge[0].clone()));
        }
        if !graph_idx.contains_key(&edge[1]) {
            graph_idx.insert(edge[1].clone(), graph.add_node(edge[1].clone()));
        }
        trace!("Graph: {:?}", graph);

        // Prepare dict for ez_eval
        use std::collections::BTreeMap;
        let mut edge_weights: BTreeMap<String, f64> = BTreeMap::from_iter(
            edge.iter()
                .skip(2)
                .map(|x| {
                    round(
                        x.parse::<f32>()
                            .unwrap_or_else(|_| panic!("cannot convert weight '{x}' to float32")),
                        weight_precision.into(),
                    ) as f64
                })
                .enumerate()
                .map(|(i, w)| (header[i + 2].clone(), w)),
        );

        // Debug
        if index < 20 {
            debug!("Edge: {:?}", edge);
            debug!("Node1 weight: {:?}", graph.node_weight(graph_idx[&edge[0]]));
            debug!("Node2 weight: {:?}", graph.node_weight(graph_idx[&edge[1]]));
            debug!("Edge weight: {:?}", edge_weights);
        }

        // Skip edge if NaN
        if edge_weights[&weight_field].is_nan() {
            warn!("NaN found:\n\t{:?}", edge);
            continue;
        }

        // Add edge to graph
        if weight_filter.is_none()
            || fasteval::ez_eval(weight_filter.as_ref().unwrap(), &mut edge_weights)
                .expect("cannot evaluate expression")
                != 0.0
        {
            // Add edge
            let e1 = graph.add_edge(
                graph_idx[&edge[0]],
                graph_idx[&edge[1]],
                if weight_n_edges {
                    1.0
                } else {
                    edge_weights[&weight_field] as f32
                },
            );
            // Debug
            if index < 20 {
                debug!("Added edge: {:?}", e1);
            }
        }
    }
    std::mem::drop(graph_span_enter);
    std::mem::drop(graph_span);

    debug!(
        "Input file has {0} nodes with {1} edges{2}",
        graph.node_count(),
        n_lines,
        if weight_filter.is_some() {
            format!(
                " ({0} edges with {1})",
                graph.edge_count(),
                weight_filter.unwrap()
            )
        } else {
            "".to_string()
        }
    );

    (graph, graph_idx)
}

pub fn graph_subset(
    graph: &mut StableGraph<String, f32, Undirected, GraphIdx>,
    subset: PathBuf,
) -> usize {
    let mut nodes_subset = Vec::<String>::new();
    let reader_file = BufReader::new(File::open(subset).expect("cannot open subset file"));
    for node in reader_file.lines() {
        nodes_subset.push(node.expect("cannot read node from subset file"));
    }
    debug!("Nodes to include: {:?}", nodes_subset);

    graph.retain_nodes(|g, ix| nodes_subset.contains(&g[ix]));

    nodes_subset.len()
}

fn get_node_weight(
    node_idx: NodeIndex<GraphIdx>,
    g: &StableGraph<String, f32, Undirected, GraphIdx>,
) -> (NodeIndex<GraphIdx>, f32) {
    (
        node_idx,
        g.edges(node_idx)
            .map(|edge| -> &f32 { edge.weight() })
            .sum::<f32>(),
    )
}

fn get_nodes_weight<I>(
    iter: I,
    g: &StableGraph<String, f32, Undirected, GraphIdx>,
) -> Vec<(NodeIndex<GraphIdx>, f32)>
where
    I: Iterator<Item = NodeIndex<GraphIdx>>,
{
    iter.collect::<Vec<NodeIndex<GraphIdx>>>()
        .par_iter()
        .map(|node_idx| get_node_weight(*node_idx, g))
        .collect()
}

pub fn find_heaviest_node(
    g: &StableGraph<String, f32, Undirected, GraphIdx>,
    nodes_idx: Option<&Vec<NodeIndex<GraphIdx>>>,
) -> (NodeIndex<GraphIdx>, f32) {
    // Calculate each node's weight
    let mut nodes_weight = nodes_idx.map_or_else(
        || get_nodes_weight(g.node_indices(), g),
        |vec| get_nodes_weight(vec.iter().copied(), g),
    );

    //Sort nodes based on connected edge weight and then alphabetically
    nodes_weight.sort_by(|a, b| {
        b.1.partial_cmp(&a.1)
            .unwrap()
            .then(g.node_weight(a.0).cmp(&g.node_weight(b.0)))
    });

    trace!("Sorted node weights: {:?}", nodes_weight);
    debug!(
        "Heaviest node and weight: {} [{:?}] => {}",
        g.node_weight(nodes_weight[0].0).unwrap(),
        nodes_weight[0].0,
        nodes_weight[0].1
    );

    nodes_weight[0]
}

fn round(x: f32, decimals: i32) -> f32 {
    let y = 10f32.powi(decimals);
    (x * y).round() / y
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    #[test]
    fn test_round() {
        assert_eq!(round(4.36, 2), 4.36);
        assert_eq!(round(4.363, 2), 4.36);
        assert_eq!(round(4.368, 2), 4.37);
        assert_eq!(round(4.36534, 2), 4.37);
        assert_eq!(round(0.999670, 4), 0.9997);
        assert_eq!(round(0.999719, 4), 0.9997);
        assert_eq!(round(0.999800, 4), 0.9998);
    }

    #[test]
    fn test_graph_read() {
        let (graph, _graph_idx) = graph_read(
            BufReader::new(File::open("test/example.tsv").expect("cannot open input file")),
            true,
            "r2".to_string(),
            Some("r2 > 0.2".to_string()),
            false,
            4,
        );
        assert_eq!(graph.is_directed(), false);
        assert_eq!(graph.node_count(), 65);
        assert_eq!(graph.edge_count(), 104);
    }

    #[test]
    fn test_graph_subset() {
        let (mut graph, _graph_idx) = graph_read(
            BufReader::new(File::open("test/example.tsv").expect("cannot open input file")),
            true,
            "r2".to_string(),
            Some("r2 > 0.2".to_string()),
            false,
            4,
        );
        assert_eq!(graph.is_directed(), false);
        graph_subset(&mut graph, PathBuf::from("test/example.subset"));
        assert_eq!(graph.node_count(), 11);
        assert_eq!(graph.edge_count(), 22);
    }

    #[test]
    fn test_find_all_edges() {
        let (graph, graph_idx) = graph_read(
            BufReader::new(File::open("test/example.tsv").expect("cannot open input file")),
            true,
            "r2".to_string(),
            Some("r2 > 0.2".to_string()),
            false,
            4,
        );
        assert_eq!(graph.edges(graph_idx["NC_046966.1:26131"]).count(), 6);
    }

    #[test]
    fn test_get_node_weight() {
        let (graph, graph_idx) = graph_read(
            BufReader::new(File::open("test/example.tsv").expect("cannot open input file")),
            true,
            "r2".to_string(),
            Some("r2 > 0.2".to_string()),
            false,
            4,
        );

        let nodes_weight = get_node_weight(graph_idx["NC_046966.1:12856"], &graph);
        assert_eq!(
            graph.node_weight(nodes_weight.0).unwrap(),
            "NC_046966.1:12856"
        );
        assert_eq!(nodes_weight.1, 0.9998);
    }

    #[test]
    fn test_get_nodes_weight() {
        let (graph, _graph_idx) = graph_read(
            BufReader::new(File::open("test/example.tsv").expect("cannot open input file")),
            true,
            "r2".to_string(),
            Some("r2 > 0.2".to_string()),
            false,
            4,
        );

        let nodes_weight = get_nodes_weight(graph.node_indices(), &graph);
        assert_eq!(
            graph.node_weight(nodes_weight[0].0).unwrap(),
            "NC_046966.1:12856"
        );
        assert_eq!(nodes_weight[0].1, 0.9998);
        assert_eq!(
            graph.node_weight(nodes_weight[1].0).unwrap(),
            "NC_046966.1:13197"
        );
        assert_eq!(nodes_weight[1].1, 0.8519);
        assert_eq!(
            graph.node_weight(nodes_weight[2].0).unwrap(),
            "NC_046966.1:13594"
        );
        assert_eq!(nodes_weight[2].1, 1.5552);
        assert_eq!(
            graph.node_weight(nodes_weight[3].0).unwrap(),
            "NC_046966.1:7391"
        );
        assert_eq!(nodes_weight[3].1, 1.0504);
        assert_eq!(
            graph.node_weight(nodes_weight[4].0).unwrap(),
            "NC_046966.1:7468"
        );
        assert_eq!(nodes_weight[4].1, 0.4336);
    }

    #[test]
    fn test_find_heaviest_node() {
        let (mut graph, graph_idx) = graph_read(
            BufReader::new(File::open("test/example.tsv").expect("cannot open input file")),
            true,
            "r2".to_string(),
            Some("r2 > 0.2".to_string()),
            false,
            4,
        );

        // Round #1
        let (node_heaviest, node_weight) = find_heaviest_node(&graph, None);
        assert_eq!(
            graph.node_weight(node_heaviest).unwrap(),
            "NC_046966.1:10729"
        );
        assert_eq!(round(node_weight, 4), f32::INFINITY);
        // Round #2
        graph.remove_node(graph_idx["NC_046966.1:10729"]);
        let (node_heaviest, node_weight) = find_heaviest_node(&graph, None);
        assert_eq!(
            graph.node_weight(node_heaviest).unwrap(),
            "NC_046966.1:26131"
        );
        assert_eq!(round(node_weight, 4), f32::INFINITY);
        // Round #3
        graph.remove_node(graph_idx["NC_046966.1:26131"]);
        let (node_heaviest, node_weight) = find_heaviest_node(&graph, None);
        assert_eq!(
            graph.node_weight(node_heaviest).unwrap(),
            "NC_046966.1:31878"
        );
        assert_eq!(round(node_weight, 4), f32::INFINITY);
        // Round #4
        graph.remove_node(graph_idx["NC_046966.1:31878"]);
        let (node_heaviest, node_weight) = find_heaviest_node(&graph, None);
        assert_eq!(
            graph.node_weight(node_heaviest).unwrap(),
            "NC_046966.1:42518"
        );
        assert_eq!(round(node_weight, 4), f32::INFINITY);
        // Round #5
        graph.remove_node(graph_idx["NC_046966.1:42518"]);
        let (node_heaviest, node_weight) = find_heaviest_node(&graph, None);
        assert_eq!(
            graph.node_weight(node_heaviest).unwrap(),
            "NC_046966.1:45910"
        );
        assert_eq!(round(node_weight, 4), f32::INFINITY);
        // Round #6
        graph.remove_node(graph_idx["NC_046966.1:45910"]);
        let (node_heaviest, node_weight) = find_heaviest_node(&graph, None);
        assert_eq!(
            graph.node_weight(node_heaviest).unwrap(),
            "NC_046966.1:38024"
        );
        assert_eq!(round(node_weight, 4), 8.2862);
    }

    #[test]
    fn test_find_connected_components() {
        use petgraph::algo::{kosaraju_scc, tarjan_scc};
        let (graph, _graph_idx) = graph_read(
            BufReader::new(File::open("test/example.tsv").expect("cannot open input file")),
            true,
            "r2".to_string(),
            Some("r2 > 0.2".to_string()),
            false,
            4,
        );
        let ccs = tarjan_scc(&graph);
        assert_eq!(ccs.len(), 9);
        for (i, n) in Vec::<usize>::from([54, 1, 3, 2, 1, 1, 1, 1, 1])
            .iter()
            .enumerate()
        {
            assert_eq!(ccs[i].len(), *n);
        }
        let ccs = kosaraju_scc(&graph);
        assert_eq!(ccs.len(), 9);
        for (i, n) in Vec::<usize>::from([1, 1, 1, 1, 1, 2, 3, 1, 54])
            .iter()
            .enumerate()
        {
            assert_eq!(ccs[i].len(), *n);
        }
    }
}
