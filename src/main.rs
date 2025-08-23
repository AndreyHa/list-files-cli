use anyhow::Result;
use clap::Parser;
use lf::{run_app, Args, Deps};
use lf::clipboard::SystemClipboard;
use lf::fs::{StdFileReader, StdWalkerFactory};
use std::sync::Arc;

#[cfg(feature = "token-counting")]
use lf::tokenizer::O200kTokenizer as TokenImpl;
#[cfg(not(feature = "token-counting"))]
use lf::tokenizer::DummyTokenizer as TokenImpl;

fn main() -> Result<()> {
    let args = Args::parse();
    if args.patterns.is_empty() {
        eprintln!("Error: At least one pattern must be provided");
        std::process::exit(1);
    }
    let tokenizer = Arc::new(TokenImpl::new().unwrap_or_else(|_| unreachable!()));
    let deps = Deps {
        walker: &StdWalkerFactory,
        reader: &StdFileReader,
        tokenizer,
        clipboard: Some(&SystemClipboard),
    };
    let _stats = run_app(
        deps,
        &args.patterns,
        args.output.as_deref(),
        args.no_clipboard,
        args.mask_java_imports,
        args.no_gitignore,
    )?;
    Ok(())
}

