use clap::Parser;
use std::path::PathBuf;

#[derive(Parser, Debug, Clone)]
#[command(name = "lf")]
#[command(about = "A fast file aggregation tool with glob patterns and tokenization. Supports tree mode (--tree) to output a text tree followed by files in tree order. Hidden paths are skipped unless a pattern containing a '.' at the first path-segment is supplied.")]
pub struct Args {
    pub patterns: Vec<String>,
    #[arg(short, long)]
    pub output: Option<PathBuf>,
    #[arg(short, long)]
    pub no_clipboard: bool,
    #[arg(long)]
    pub mask_java_imports: bool,
    #[arg(long)]
    pub no_gitignore: bool,
    #[arg(long, short='T', help="Print a text file tree first, then files in that tree order")]
    pub tree: bool,
}