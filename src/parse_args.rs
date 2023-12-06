use clap::{ArgAction, Parser};
use flexi_logger::{style, DeferredNow, Record};
use std::path::PathBuf;

#[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
#[cfg(feature = "colors")]
pub type log_format =
    fn(w: &mut dyn Write, now: &mut DeferredNow, record: &Record<'_>) -> Result<(), Error>;
pub fn log_format(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    write!(
        w,
        "[{}] {}: {}",
        style(level).paint(now.format("%Y-%m-%d %H:%M:%S").to_string()),
        style(level).paint(record.level().to_string()),
        style(level).paint(&record.args().to_string())
    )
}

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

    /// Output file.
    ///
    /// The file to output pruned nodes.
    #[clap(short, long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    /// Excluded nodes file.
    ///
    /// File to dump excluded nodes.
    #[clap(long, required = false, value_name = "FILE")]
    pub out_excl: Option<PathBuf>,

    /// Output starting graph.
    ///
    /// The file to output starting graph.
    #[clap(long, required = false, value_name = "FILE")]
    pub out_graph: Option<PathBuf>,

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

    /// Node IDs to exclude.
    ///
    /// File with node IDs to include (one per line).
    #[clap(long, required = false, value_name = "FILE")]
    pub subset: Option<PathBuf>,

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
