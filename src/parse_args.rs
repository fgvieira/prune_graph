use clap::Parser;
use flexi_logger::{style, DeferredNow, Record};
use log::LevelFilter;
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
    /// Number of threads
    #[clap(short, long, default_value_t = 1)]
    pub threads: u8,

    /// File with edges to be pruned
    #[clap(short, long = "in", default_value = "-")]
    pub input: PathBuf,

    /// Input file has header
    #[clap(long, action)]
    pub header: bool,

    /// The file to output pruned nodes
    #[clap(short, long, default_value = "-")]
    pub out: PathBuf,

    /// File to dump excluded nodes
    #[clap(long, required = false)]
    pub out_excl: Option<PathBuf>,

    /// The file to output starting graph
    #[clap(long, required = false)]
    pub out_graph: Option<PathBuf>,

    /// Field from input with weight
    #[clap(short = 'f', long, default_value_t = 3)]
    pub weight_field: usize,

    /// Minimum weight between two nodes to assume they are related
    #[clap(short = 'm', long)]
    pub weight_min: f32,

    /// Sum of (w)eights, sum of (a)bsolute weights, sum of (p)ositive weights, or (n)umber of connections
    #[clap(short = 'w', long, default_value = "a")]
    pub weight_type: String,

    #[clap(long, default_value_t = 4)]
    pub weight_precision: u8,

    /// Keep 'heaviest' nodes (instead of removing them)
    #[clap(long, action)]
    pub keep_heavy: bool,

    /// File with node IDs to include (one per line)
    #[clap(long, required = false)]
    pub subset: Option<PathBuf>,

    /// Log level
    #[clap(short, long, default_value_t = LevelFilter::Info)]
    pub log_level: LevelFilter,
}
