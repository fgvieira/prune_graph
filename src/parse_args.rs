use clap::{ArgAction, Parser};
use std::path::PathBuf;

/// Prune nodes from a graph and output unlinked nodes.
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
pub struct Args {
    /// Number of threads.
    #[clap(short, long, default_value_t = 1, value_name = "INT")]
    pub n_threads: usize,

    /// Input file.
    ///
    /// File with edges to be pruned.
    #[clap(short, long = "in", value_name = "FILE")]
    pub input: Option<PathBuf>,

    /// Input file has header.
    #[clap(long, action)]
    pub header: bool,

    /// Node IDs to exclude.
    ///
    /// File with node IDs to include (one per line).
    #[clap(long, required = false, value_name = "FILE")]
    pub subset: Option<PathBuf>,

    /// Weight column.
    ///
    /// Column in input file to use as weight (needs to be present in header); if input file has no header you can use "column_#", where "#" stands for the column number.
    #[clap(short = 'w', long, default_value = "column_3", value_name = "STRING")]
    pub weight_field: String,

    /// Filter expression.
    ///
    /// Expression to filter edges before pruning; any expression supported by 'fasteval'.
    #[clap(short = 'f', long, required = false, value_name = "STRING")]
    pub weight_filter: Option<String>,

    /// Weight as number of edges.
    ///
    /// Node's weight as number of connected edges, instead of (default) summing over their weights.
    #[clap(long)]
    pub weight_n_edges: bool,

    /// Weight precision.
    #[clap(long, default_value_t = 4, value_name = "INT")]
    pub weight_precision: u8,

    /// Keep 'heavy' nodes
    ///
    /// Keep 'heavy' (highest total weight) nodes, instead of (default) removing them.
    #[clap(long, action)]
    pub keep_heavy: bool,

    /// Prunning mode.
    #[clap(long, default_value_t = 1, value_name = "INT")]
    pub mode: u8,

    /// Output starting graph.
    ///
    /// The file to output starting graph.
    #[clap(long, required = false, value_name = "FILE")]
    pub out_graph: Option<PathBuf>,

    /// Output starting components in JSONL format.
    ///
    /// The file to output the starting components in JSONL format.
    #[clap(long, required = false, value_name = "FILE")]
    pub out_comps: Option<PathBuf>,

    /// Excluded nodes file.
    ///
    /// File to dump excluded nodes.
    #[clap(long, required = false, value_name = "FILE")]
    pub out_excl: Option<PathBuf>,

    /// Output file.
    ///
    /// The file to output pruned nodes.
    #[clap(short, long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    /// Suppress warnings.
    ///
    /// By default, only warnings are printed. By setting this flag, warnings will be disabled.
    #[arg(short = 'q', long, global = true, conflicts_with = "verbose")]
    pub quiet: bool,

    /// Verbosity.
    ///
    /// Flag can be set multiply times to increase verbosity, or left unset for quiet mode.
    #[clap(short = 'v', long, action = ArgAction::Count, global = true)]
    pub verbose: u8,
}
