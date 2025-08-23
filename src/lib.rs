pub mod cli;
pub mod tokenizer;
pub mod binary;
pub mod patterns;
pub mod fs;
pub mod clipboard;
pub mod app;

pub use app::{run_app, Deps, Stats};
pub use cli::Args;