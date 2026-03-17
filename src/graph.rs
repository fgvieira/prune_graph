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
use tracing::{debug, enabled, error, info, info_span, trace, warn, Level};
use tracing_indicatif::span_ext::IndicatifSpanExt;

#[cfg(not(feature = "large_graph"))]
pub type _GraphIdx = u32;
#[cfg(feature = "large_graph")]
pub type _GraphIdx = usize;

pub type _Graph = StableGraph<String, f32, Undirected, _GraphIdx>;
pub type _NodeIdx = NodeIndex<_GraphIdx>;

pub fn graph_read<R: BufRead>(
    reader: R,
    has_header: bool,
    weight_field: Option<String>,
    weight_filter: Option<String>,
    weight_precision: u8,
) -> (_Graph, HashMap<String, _NodeIdx>) {
    // Create graph
    let mut graph = _Graph::default();
    debug!(
        "Creating graph with GraphIdx = {}",
        std::any::type_name::<_GraphIdx>()
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
            graph_span.pb_set_message(&format!(
                "for graph with {0} nodes and {1} edges",
                graph.node_count(),
                graph.edge_count()
            ));
        }

        //let edge: Vec<&str> = line.split('\t').collect();
        let edge: Vec<String> = line.split('\t').map(str::to_string).collect();
        let mut _keep_edge = true;

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
            if has_header {
                continue;
            }
        }
        n_lines += 1;
        debug!("Edge: {:?}", edge);

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

        // Add nodes (check if exist and add them if not)
        // Node label is stored as its "weight"
        for n in [0, 1] {
            if ["NA", "N/A", "Na", "na", "n/a", "N/a"].contains(&edge[n].as_str()) {
                warn!("Edge node {} is N/A. Skipping edge!", n + 1);
                _keep_edge = false;
            } else if !graph_idx.contains_key(&edge[n]) {
                graph_idx.insert(edge[n].clone(), graph.add_node(edge[n].clone()));
                debug!(
                    "Node{} weight: {}",
                    n + 1,
                    graph
                        .node_weight(graph_idx[&edge[n]])
                        .expect("cannot find node weight")
                );
            }
        }
        trace!("Graph: {:?}", graph);

        // Parse weights and prepare dict for ez_eval
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
        trace!("Edge weights: {:?}", edge_weights);

        // Eval edge
        if !weight_filter.is_none()
            && fasteval::ez_eval(weight_filter.as_ref().unwrap(), &mut edge_weights)
                .expect("cannot evaluate expression")
                == 0.0
        {
            _keep_edge = false;
            debug!("Edge eval failed. Skipping edge!");
        }

        // Remove NaN
        let edge_weight = if let Some(ref _weight_field) = weight_field {
            if edge_weights[_weight_field].is_nan() {
                _keep_edge = false;
                warn!("Edge weight is NaN. Skipping edge!");
            }
            edge_weights[_weight_field] as f32
        } else {
            1.0
        };

        // Add edge to graph
        if _keep_edge {
            let e1 = graph.add_edge(graph_idx[&edge[0]], graph_idx[&edge[1]], edge_weight);
            debug!(
                "Added edge {:?} with weight {}",
                e1,
                graph.edge_weight(e1).expect("cannot find edge weight")
            );
        }
    }
    std::mem::drop(graph_span_enter);
    std::mem::drop(graph_span);

    info!(
        "Input file has {0} nodes with {1} edges{2}",
        graph.node_count(),
        n_lines,
        if let Some(_weight_filter) = weight_filter {
            format!(" ({0} edges with {1})", graph.edge_count(), _weight_filter)
        } else {
            "".to_string()
        }
    );

    (graph, graph_idx)
}

pub fn graph_subset(graph: &mut _Graph, subset: PathBuf) -> usize {
    let mut nodes_subset = Vec::<String>::new();
    let reader_file = BufReader::new(File::open(subset).expect("cannot open subset file"));
    for node in reader_file.lines() {
        nodes_subset.push(node.expect("cannot read node from subset file"));
    }
    info!("Nodes to include: {:?}", nodes_subset);

    graph.retain_nodes(|g, ix| nodes_subset.contains(&g[ix]));

    nodes_subset.len()
}

fn get_node_weight(node_idx: _NodeIdx, g: &_Graph) -> (_NodeIdx, f32) {
    (
        node_idx,
        g.edges(node_idx)
            .map(|edge| -> &f32 { edge.weight() })
            .sum::<f32>(),
    )
}

fn get_nodes_weight<I>(iter: I, g: &_Graph) -> Vec<(_NodeIdx, f32)>
where
    I: Iterator<Item = _NodeIdx>,
{
    iter.collect::<Vec<_NodeIdx>>()
        .par_iter()
        .map(|node_idx| get_node_weight(*node_idx, g))
        .collect()
}

pub fn find_heaviest_node(g: &_Graph, nodes_idx: Option<&Vec<_NodeIdx>>) -> (_NodeIdx, f32) {
    // Calculate each node's weight
    let mut nodes_weight = nodes_idx.map_or_else(
        || get_nodes_weight(g.node_indices(), g),
        |vec| get_nodes_weight(vec.iter().copied(), g),
    );
    trace!("Node weights: {:?}", nodes_weight);

    //Sort nodes based on connected edge weight and then alphabetically
    nodes_weight.sort_by(|a, b| {
        b.1.total_cmp(&a.1)
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
            Some("r2".to_string()),
            Some("r2 > 0.2".to_string()),
            4,
        );
        assert_eq!(graph.is_directed(), false);
        assert_eq!(graph.node_count(), 65);
        assert_eq!(graph.edge_count(), 103);
    }

    #[test]
    fn test_graph_subset() {
        let (mut graph, _graph_idx) = graph_read(
            BufReader::new(File::open("test/example.tsv").expect("cannot open input file")),
            true,
            Some("r2".to_string()),
            Some("r2 > 0.2".to_string()),
            4,
        );
        assert_eq!(graph.is_directed(), false);
        graph_subset(&mut graph, PathBuf::from("test/example.subset"));
        assert_eq!(graph.node_count(), 11);
        assert_eq!(graph.edge_count(), 21);
    }

    #[test]
    fn test_find_all_edges() {
        let (graph, graph_idx) = graph_read(
            BufReader::new(File::open("test/example.tsv").expect("cannot open input file")),
            true,
            Some("r2".to_string()),
            Some("r2 > 0.2".to_string()),
            4,
        );
        assert_eq!(graph.edges(graph_idx["NC_046966.1:26131"]).count(), 6);
    }

    #[test]
    fn test_get_node_weight() {
        let (graph, graph_idx) = graph_read(
            BufReader::new(File::open("test/example.tsv").expect("cannot open input file")),
            true,
            Some("r2".to_string()),
            Some("r2 > 0.2".to_string()),
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
            Some("r2".to_string()),
            Some("r2 > 0.2".to_string()),
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
            Some("r2".to_string()),
            Some("r2 > 0.2".to_string()),
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
            Some("r2".to_string()),
            Some("r2 > 0.2".to_string()),
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
