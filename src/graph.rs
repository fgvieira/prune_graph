use log::{debug, error, info, trace, warn};
use petgraph::stable_graph::{NodeIndex, StableGraph};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

pub fn graph_read(
    tsv: PathBuf,
    has_header: bool,
    weight_field: String,
    weight_filter: Option<String>,
    weight_n_edges: bool,
    weight_precision: u8,
) -> (StableGraph<String, f32>, HashMap<String, NodeIndex>) {
    // Open input file
    let input: Box<dyn std::io::Read + 'static> = if tsv.as_os_str().eq("-") {
        Box::new(std::io::stdin())
    } else {
        //match File::open(&tsv) {
        //    Ok(file) => Box::new(file),
        //    Err(err) => {
        //        error!("{}: {}", tsv.display(), err);
        //    }
        //}
        Box::new(File::open(tsv).expect("ERROR MESSAGE NOT WORKING!"))
    };
    let in_reader = BufReader::new(input);
    //let in_reader = BufReader::new(File::open(tsv).expect("cannot open input file"));

    // Create graph
    let mut graph = StableGraph::<String, f32>::new();
    //let mut graph = petgraph::stable_graph::StableGraph::<String, f32, petgraph::Undirected>::new();
    //let mut graph = petgraph::stable_graph::StableGraph::<String, f32>::new_undirected();
    //let mut graph = petgraph::stable_graph::StableUnGraph::<String, f32>::new();
    if graph.is_directed() {
        error!("Graph has to be undirected!");
    }
    let mut graph_idx = HashMap::new();

    // Read the file line by line
    let mut header: Vec<String> = Vec::new();
    let mut n_lines: usize = 0;
    for (index, line) in in_reader.lines().enumerate() {
        let line = line.expect("cannot read line from input file");

        //let edge: Vec<&str> = line.split('\t').collect();
        let edge: Vec<String> = line.split('\t').map(str::to_string).collect();

        // Define header
        if index == 0 {
            header = if has_header {
                edge.clone()
            } else {
                (1..edge.len() + 1)
                    .map(|h| format!("column_{}", h.to_string()))
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

        // Check if nodes exist and add them if not
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
                        x.parse::<f64>().expect("cannot convert weight to float64"),
                        weight_precision.into(),
                    )
                })
                .enumerate()
                .map(|(i, w)| (header[i + 2].clone(), w))
                .into_iter(),
        );

        // Debug
        if index < 10 {
            debug!("{:?}", edge);
            debug!("{:?}", edge_weights);
        }

        // Skip edge if NaN
        if edge_weights[&weight_field].is_nan() {
            warn!("NaN found:\n\t{:?}", edge);
            continue;
        }

        // Add edge to graph
        if weight_filter.is_none()
            || fasteval::ez_eval(&weight_filter.as_ref().unwrap(), &mut edge_weights)
                .expect("cannot evaluate expression")
                != 0.0
        {
            // Add edge
            let _e1 = graph.add_edge(
                graph_idx[&edge[0]],
                graph_idx[&edge[1]],
                if weight_n_edges {
                    1.0
                } else {
                    edge_weights[&weight_field].clone() as f32
                },
            );
            // Add other edge, until "Undirected" is implemented
            let _e2 = graph.add_edge(
                graph_idx[&edge[1]],
                graph_idx[&edge[0]],
                if weight_n_edges {
                    1.0
                } else {
                    edge_weights[&weight_field].clone() as f32
                },
            );
        }
    }

    info!(
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

    return (graph, graph_idx);
}

pub fn graph_subset(graph: &mut StableGraph<String, f32>, subset: PathBuf) -> usize {
    let mut nodes_subset = Vec::<String>::new();
    let in_reader = BufReader::new(File::open(subset).expect("cannot open subset file"));
    for node in in_reader.lines() {
        nodes_subset.push(node.expect("cannot read node from subset file"));
    }
    debug!("Nodes to include: {:?}", nodes_subset);

    graph.retain_nodes(|g, ix| nodes_subset.contains(&g[ix]));

    return nodes_subset.len();
}

fn get_nodes_weight<I>(iter: I, g: &StableGraph<String, f32>) -> Vec<(NodeIndex, f32)>
where
    I: Iterator<Item = NodeIndex>,
{
    iter.map(|node_idx| {
        (
            node_idx,
            g.edges(node_idx)
                .map(|edge| -> &f32 { edge.weight() })
                .sum::<f32>(),
        )
    })
    .collect()
}

pub fn find_heaviest_node(
    g: &StableGraph<String, f32>,
    nodes_idx: Option<&Vec<NodeIndex>>,
) -> (NodeIndex, f32) {
    // Calculate each node's weight
    let mut nodes_weight = nodes_idx.map_or_else(
        || get_nodes_weight(g.node_indices(), g),
        |vec| get_nodes_weight(vec.into_iter().copied(), g),
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

    return nodes_weight[0];
}

fn round(x: f64, decimals: i32) -> f64 {
    let y = 10f64.powi(decimals);
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
            PathBuf::from("test/example.tsv"),
            false,
            "column_7".to_string(),
            Some("column_7 > 0.2".to_string()),
            false,
            4,
        );
        assert_eq!(graph.node_count(), 65);
        assert_eq!(graph.edge_count(), 196);
    }

    #[test]
    fn test_graph_subset() {
        let (mut graph, _graph_idx) = graph_read(
            PathBuf::from("test/example.tsv"),
            false,
            "column_7".to_string(),
            Some("column_7 > 0.2".to_string()),
            false,
            4,
        );
        graph_subset(&mut graph, PathBuf::from("test/example.subset"));
        assert_eq!(graph.node_count(), 11);
        assert_eq!(graph.edge_count(), 44);
    }

    #[test]
    fn test_find_all_edges() {
        let (graph, graph_idx) = graph_read(
            PathBuf::from("test/example.tsv"),
            false,
            "column_7".to_string(),
            Some("column_7 > 0.2".to_string()),
            false,
            4,
        );
        assert_eq!(graph.edges(graph_idx["NC_046966.1:26131"]).count(), 5);
    }

    #[test]
    fn test_find_dir_edges() {
        let (graph, graph_idx) = graph_read(
            PathBuf::from("test/example.tsv"),
            false,
            "column_7".to_string(),
            Some("column_7 > 0.2".to_string()),
            false,
            4,
        );
        assert_eq!(
            graph
                .edges_directed(graph_idx["NC_046966.1:26131"], petgraph::Outgoing)
                .count(),
            3
        );
    }

    #[test]
    fn test_get_nodes_weight() {
        let (graph, _graph_idx) = graph_read(
            PathBuf::from("test/example.tsv"),
            false,
            "column_7".to_string(),
            Some("column_7 > 0.2".to_string()),
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
        let (graph, _graph_idx) = graph_read(
            PathBuf::from("test/example.tsv"),
            false,
            "column_7".to_string(),
            Some("column_7 > 0.2".to_string()),
            false,
            4,
        );

        let (node_heaviest, node_weight) = find_heaviest_node(&graph, None);
        assert_eq!(
            graph.node_weight(node_heaviest).unwrap(),
            "NC_046966.1:38024"
        );
        assert_eq!(node_weight, 9.2859);
    }

    #[test]
    fn test_find_connected_components() {
        use petgraph::algo::{kosaraju_scc, tarjan_scc};

        let (graph, _graph_idx) = graph_read(
            PathBuf::from("test/example.tsv"),
            false,
            "column_7".to_string(),
            Some("column_7 > 0.2".to_string()),
            false,
            4,
        );

        let ccs = tarjan_scc(&graph);
        assert_eq!(ccs.len(), 13);
        let ccs = kosaraju_scc(&graph);
        assert_eq!(ccs.len(), 13);
    }
}
