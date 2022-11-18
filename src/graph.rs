use log::{debug, error, info, trace, warn};
use petgraph::stable_graph::{NodeIndex, StableGraph};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;

pub fn read_graph(
    tsv: PathBuf,
    header: bool,
    weight_field: usize,
    weight_type: String,
    weight_min: f32,
    weight_precision: u8,
) -> StableGraph<String, f32> {
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
    info!("Creating graph...");
    let mut graph = StableGraph::<String, f32>::new();
    //let mut graph = petgraph::stable_graph::StableGraph::<String, f32, petgraph::Undirected>::new();
    //let mut graph = petgraph::stable_graph::StableGraph::<String, f32>::new_undirected();
    //let mut graph = petgraph::stable_graph::StableUnGraph::<String, f32>::new();
    if graph.is_directed() {
        error!("Graph has to be undirected!");
    }

    // Read the file line by line
    let mut n_lines: usize = 0;
    for (index, line) in in_reader.lines().enumerate() {
        let line = line.expect("cannot read line from input file");

        // Check for header
        if header && index == 0 {
            continue;
        }
        n_lines += 1;

        //let edge: Vec<&str> = line.splitn(weight_field + 1, '\t').collect();
        let edge: Vec<String> = line
            .splitn(weight_field + 1, '\t')
            .map(str::to_string)
            .collect();

        // Debug
        if index < 10 {
            debug!("{:?}", edge);
        }

        let find_n1 = find_node_by_weight(&graph, edge[0].clone());
        let n1 = if find_n1.is_none() {
            graph.add_node(edge[0].clone())
        } else {
            find_n1.unwrap()
        };
        let find_n2 = find_node_by_weight(&graph, edge[1].clone());
        let n2 = if find_n2.is_none() {
            graph.add_node(edge[1].clone())
        } else {
            find_n2.unwrap()
        };
        trace!("Graph: {:?}", graph);

        let mut edge_weight: f32 = edge[weight_field - 1]
            .parse()
            .expect("cannot convert weight to float");
        if edge_weight.is_nan() {
            warn!("NaN found:\n\t{:?}", edge);
            continue;
        } else if weight_type == "p" {
            // set negative values to zero
            edge_weight = if edge_weight < 0.0 { 0.0 } else { edge_weight };
        } else if weight_type == "a" {
            // use absolute weight
            edge_weight = edge_weight.abs();
        }

        // Round edge weights
        edge_weight = round(edge_weight, weight_precision.into());

        // Add edge to graph
        if edge_weight >= weight_min {
            // Convert weights to number of edges
            if weight_type == "n" {
                edge_weight = 1.0;
            }
            // Add edge
            let _e1 = graph.add_edge(n1, n2, edge_weight);
            // Add other edge, until "Undirected" is implemented
            let _e2 = graph.add_edge(n2, n1, edge_weight);
        }
    }

    info!(
        "Input file has {0} nodes with {1} edges ({2} edges with weight >= {3})",
        graph.node_count(),
        n_lines,
        graph.edge_count(),
        weight_min
    );

    return graph;
}

fn round(x: f32, decimals: i32) -> f32 {
    let y = 10f32.powi(decimals);
    (x * y).round() / y
}

fn find_node_by_weight(g: &StableGraph<String, f32>, weight: String) -> Option<NodeIndex> {
    g.node_indices()
        .find(|n| g.node_weight(*n) == Some(&weight))
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    fn get_graph() -> (StableGraph<std::string::String, f32>, NodeIndex) {
        let mut graph = petgraph::stable_graph::StableGraph::<String, f32>::new();
        let origin = graph.add_node("Denver".to_string());
        let dest_1 = graph.add_node("San Diego".to_string());
        let _cost_1 = graph.add_edge(origin, dest_1, 50.45);
        let dest_11 = graph.add_node("Washington".to_string());
        let dest_12 = graph.add_node("New York".to_string());
        let _cost_1 = graph.add_edge(dest_1, dest_11, 250.45);
        let _cost_2 = graph.add_edge(dest_1, dest_12, 1099.34);
        return (graph, dest_1);
    }

    #[test]
    fn test_find_node_by_weight() {
        let (graph, node) = get_graph();
        assert_eq!(
            find_node_by_weight(&graph, "San Diego".to_string()).unwrap(),
            node
        );
    }

    #[test]
    fn test_find_all_edges() {
        let (graph, node) = get_graph();
        assert_eq!(graph.edges(node).count(), 3);
    }

    #[test]
    fn test_find_dir_edges() {
        let (graph, node) = get_graph();
        assert_eq!(graph.edges(node).count(), 2);
    }

    #[test]
    fn test_round() {
        assert_eq!(round(4.36534, 2), 4.37);
        assert_eq!(round(4.36, 2), 4.36);
        assert_eq!(round(4.363, 2), 4.36);
        assert_eq!(round(4.368, 2), 4.37);
    }
}
