use log::{debug, error, info, trace, warn};
use petgraph::stable_graph::{NodeIndex, StableGraph};
use std::collections::HashMap;
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
    info!("Creating graph...");
    let mut graph = StableGraph::<String, f32>::new();
    //let mut graph = petgraph::stable_graph::StableGraph::<String, f32, petgraph::Undirected>::new();
    //let mut graph = petgraph::stable_graph::StableGraph::<String, f32>::new_undirected();
    //let mut graph = petgraph::stable_graph::StableUnGraph::<String, f32>::new();
    if graph.is_directed() {
        error!("Graph has to be undirected!");
    }
    let mut graph_idx = HashMap::new();

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

        // Check if node already exists
        if !graph_idx.contains_key(&edge[0]) {
            graph_idx.insert(edge[0].clone(), graph.add_node(edge[0].clone()));
        }
        if !graph_idx.contains_key(&edge[1]) {
            graph_idx.insert(edge[1].clone(), graph.add_node(edge[1].clone()));
        }
        trace!("Graph: {:?}", graph);

        // Parse weight
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
            let _e1 = graph.add_edge(graph_idx[&edge[0]], graph_idx[&edge[1]], edge_weight);
            // Add other edge, until "Undirected" is implemented
            //let _e2 = graph.add_edge(graph_idx[&edge[1]], graph_idx[&edge[0]], edge_weight);
        }
    }

    info!(
        "Input file has {0} nodes with {1} edges ({2} edges with weight >= {3})",
        graph.node_count(),
        n_lines,
        graph.edge_count(),
        weight_min
    );

    return (graph, graph_idx);
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
    fn test_find_all_edges() {
        let (graph, graph_idx) = read_graph(
            PathBuf::from("test/example.tsv"),
            false,
            7,
            "a".to_string(),
            0.2,
            4,
        );
        assert_eq!(graph.edges(graph_idx["NC_046966.1:26131"]).count(), 5);
    }

    #[test]
    fn test_find_dir_edges() {
        let (graph, graph_idx) = read_graph(
            PathBuf::from("test/example.tsv"),
            false,
            7,
            "a".to_string(),
            0.2,
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
    fn test_round() {
        assert_eq!(round(4.36534, 2), 4.37);
        assert_eq!(round(4.36, 2), 4.36);
        assert_eq!(round(4.363, 2), 4.36);
        assert_eq!(round(4.368, 2), 4.37);
    }
}
