use anyhow::{Context, Result};
use arboard::Clipboard;
use clap::Parser;
use globset::{Glob, GlobSetBuilder};
use rayon::prelude::*;
use std::{
    fs::File,
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tiktoken_rs::cl100k_base;
use walkdir::WalkDir;

/// Trait for abstracting tokenization
trait Tokenizer: Send + Sync {
    fn count_tokens(&self, text: &str) -> usize;
}

/// Implementation using cl100k_base tokenizer
struct Cl100kTokenizer {
    bpe: tiktoken_rs::CoreBPE,
}

impl Cl100kTokenizer {
    fn new() -> Result<Self> {
        let bpe = cl100k_base().context("Failed to initialize cl100k_base tokenizer")?;
        Ok(Self { bpe })
    }
}

impl Tokenizer for Cl100kTokenizer {
    fn count_tokens(&self, text: &str) -> usize {
        self.bpe.encode_with_special_tokens(text).len()
    }
}

/// Check if a file should be treated as binary based on its extension
fn is_binary_file(path: &Path) -> bool {
    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        matches!(ext.as_str(),
            // Executables and libraries
            "exe" | "dll" | "so" | "dylib" | "a" | "lib" | "bin" | "o" | "obj" | "rlib" |
            // Images
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tiff" | "tga" | "ico" | "webp" |
            // Videos
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" |
            // Audio
            "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" |
            // Archives
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" |
            // Documents
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" |
            // Other binary formats
            "pdb" | "sqlite" | "db" | "class" | "pyc" | "d" |
            // IDE and tool generated files
            "idx" | "cache" | "lock" | "tmp" | "temp"
        )
    } else {
        false
    }
}

/// Generate metadata information for binary files
fn get_binary_file_info(path: &Path) -> Result<String> {
    let metadata = std::fs::metadata(path)
        .with_context(|| format!("Failed to get metadata for: {}", path.display()))?;
    
    let size = metadata.len();
    let size_str = if size < 1024 {
        format!("{} bytes", size)
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.0)
    } else if size < 1024 * 1024 * 1024 {
        format!("{:.1} MB", size as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.1} GB", size as f64 / (1024.0 * 1024.0 * 1024.0))
    };

    if let Some(ext) = path.extension() {
        let ext = ext.to_string_lossy().to_lowercase();
        match ext.as_str() {
            "dll" | "so" | "dylib" | "exe" | "bin" => {
                Ok(format!("[Binary file: {} - Size: {}]", ext.to_uppercase(), size_str))
            }
            "png" | "jpg" | "jpeg" | "gif" | "bmp" | "tiff" | "tga" | "ico" | "webp" => {
                Ok(format!("[Image file: {} - Size: {}]", ext.to_uppercase(), size_str))
            }
            "mp4" | "avi" | "mkv" | "mov" | "wmv" | "flv" | "webm" => {
                Ok(format!("[Video file: {} - Size: {}]", ext.to_uppercase(), size_str))
            }
            "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" => {
                Ok(format!("[Audio file: {} - Size: {}]", ext.to_uppercase(), size_str))
            }
            "zip" | "rar" | "7z" | "tar" | "gz" | "bz2" | "xz" => {
                Ok(format!("[Archive file: {} - Size: {}]", ext.to_uppercase(), size_str))
            }
            "pdf" | "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" => {
                Ok(format!("[Document file: {} - Size: {}]", ext.to_uppercase(), size_str))
            }
            _ => {
                Ok(format!("[Binary file: {} - Size: {}]", ext.to_uppercase(), size_str))
            }
        }
    } else {
        Ok(format!("[Binary file - Size: {}]", size_str))
    }
}

#[derive(Parser)]
#[command(name = "lf")]
#[command(about = "A fast file aggregation tool with glob patterns and tokenization")]
struct Args {
    /// One or more glob patterns; prefix with `~` to exclude matches
    patterns: Vec<String>,

    /// Write concatenated content to file instead of clipboard
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Do not copy content to the clipboard
    #[arg(short, long)]
    no_clipboard: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.patterns.is_empty() {
        eprintln!("Error: At least one pattern must be provided");
        std::process::exit(1);
    }

    // Build include and exclude glob sets
    let mut include_builder = GlobSetBuilder::new();
    let mut exclude_builder = GlobSetBuilder::new();

    for pattern in &args.patterns {
        if let Some(exclude_pattern) = pattern.strip_prefix('~') {
            // Handle different types of exclusion patterns
            let actual_exclude_pattern = if exclude_pattern.ends_with('/') {
                // Directory ending with slash - exclude everything in it
                format!("{}**", exclude_pattern)
            } else if exclude_pattern.contains('*') || exclude_pattern.contains('/') {
                // Glob pattern or contains path separator - use as is
                exclude_pattern.to_string()
            } else if exclude_pattern.starts_with('.') && exclude_pattern.chars().skip(1).all(|c| c != '.') {
                // Dotted directory like .cache, .git - exclude directory contents
                format!("{}/**", exclude_pattern)
            } else if exclude_pattern.contains('.') && exclude_pattern.rfind('.').unwrap() > 0 {
                // File with extension (dot not at start) - use as is
                exclude_pattern.to_string()
            } else {
                // Simple name without extension - could be directory, add /** to exclude directory contents
                format!("{}/**", exclude_pattern)
            };
            
            let glob = Glob::new(&actual_exclude_pattern)
                .with_context(|| format!("Invalid exclude pattern: {}", actual_exclude_pattern))?;
            exclude_builder.add(glob);
            
            // Also add the exact pattern for files in root directory
            if !exclude_pattern.contains('/') && !exclude_pattern.contains('*') {
                let root_glob = Glob::new(exclude_pattern)
                    .with_context(|| format!("Invalid exclude pattern: {}", exclude_pattern))?;
                exclude_builder.add(root_glob);
            }
        } else {
            // Handle special cases for directory patterns
            let actual_pattern = match pattern.as_str() {
                "." => "**/*",           // Current directory means all files recursively
                "./" => "**/*",          // Same as above
                path if path.ends_with('/') => &format!("{}**/*", path),  // Directory/ means dir/**/*
                path if !path.contains('*') && !path.contains('.') && !path.contains('/') => {
                    // Simple directory name like "src" - include all files in that directory
                    &format!("{}/**", path)
                }
                _ => pattern,            // Use pattern as-is for everything else
            };
            
            let glob = Glob::new(actual_pattern)
                .with_context(|| format!("Invalid include pattern: {}", actual_pattern))?;
            include_builder.add(glob);
        }
    }

    let include_set = include_builder.build()?;
    let exclude_set = exclude_builder.build()?;

    // Collect matching files
    let files: Vec<PathBuf> = WalkDir::new(".")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|path| {
            // Convert path to string and normalize separators for cross-platform compatibility
            let path_str = path.to_string_lossy().replace('\\', "/");
            
            // Also try with the path stripped of leading "./"
            let stripped_path = path_str.strip_prefix("./").unwrap_or(&path_str);
            
            // Get filename as string
            let filename = path.file_name()
                .map(|f| f.to_string_lossy().to_string())
                .unwrap_or_default();
            
            // Check both original and stripped paths
            let matches_include = include_set.is_match(&path_str) || 
                                include_set.is_match(stripped_path) ||
                                include_set.is_match(&filename);
            let matches_exclude = exclude_set.is_match(&path_str) || 
                                exclude_set.is_match(stripped_path) ||
                                exclude_set.is_match(&filename);
            
            matches_include && !matches_exclude
        })
        .collect();

    if files.is_empty() {
        println!("No files found matching the patterns.");
        return Ok(());
    }

    // Initialize tokenizer
    let tokenizer = Arc::new(Cl100kTokenizer::new()?);

    // Process files in parallel
    let total_lines = Arc::new(AtomicUsize::new(0));
    let total_tokens = Arc::new(AtomicUsize::new(0));

    // Determine output strategy
    // Use clipboard when no explicit output file and no --no-clipboard flag
    // But if stdout is redirected (like > file.txt), write to stdout instead
    let use_clipboard = !args.no_clipboard && args.output.is_none();
    let content_buffer = if use_clipboard {
        Some(Arc::new(Mutex::new(String::new())))
    } else {
        None
    };

    // Setup output writer for file or stdout
    let mut output_writer: Option<Box<dyn Write + Send>> = if let Some(output_path) = &args.output {
        let file = File::create(output_path)
            .with_context(|| format!("Failed to create output file: {}", output_path.display()))?;
        Some(Box::new(BufWriter::new(file)))
    } else if args.no_clipboard {
        // Explicitly requested no clipboard, write to stdout
        Some(Box::new(std::io::stdout()))
    } else {
        // Default behavior: try clipboard, but if it fails, write to stdout
        None
    };

    // Process files in parallel with binary file handling
    let file_contents: Result<Vec<(PathBuf, String, usize, usize)>> = files
        .par_iter()
        .map(|file_path| -> Result<(PathBuf, String, usize, usize)> {
            // Check if it's a binary file
            if is_binary_file(file_path) {
                let binary_info = get_binary_file_info(file_path)?;
                // Binary files don't contribute to line count, but we still count tokens in the metadata
                let token_count = tokenizer.count_tokens(&binary_info);
                return Ok((file_path.clone(), binary_info, 0, token_count));
            }

            // Regular text file processing
            let file = File::open(file_path)
                .with_context(|| format!("Failed to open file: {}", file_path.display()))?;
            
            let reader = BufReader::new(file);
            let mut content = String::new();
            let mut line_count = 0;

            for line_result in reader.lines() {
                let line = line_result
                    .with_context(|| format!("Failed to read line from: {}", file_path.display()))?;
                content.push_str(&line);
                content.push('\n');
                line_count += 1;
            }

            let token_count = tokenizer.count_tokens(&content);
            
            Ok((file_path.clone(), content, line_count, token_count))
        })
        .collect();

    let file_contents = file_contents?;

    // Aggregate results and write output
    for (file_path, content, line_count, token_count) in file_contents {
        total_lines.fetch_add(line_count, Ordering::Relaxed);
        total_tokens.fetch_add(token_count, Ordering::Relaxed);

        // Format path with forward slashes and remove leading "./"
        let display_path = file_path.to_string_lossy()
            .replace('\\', "/")
            .strip_prefix("./")
            .unwrap_or(&file_path.to_string_lossy())
            .to_string();

        let file_header = format!("{}\n", display_path);
        let file_output = format!("{}{}\n\n", file_header, content);

        if let Some(ref buffer) = content_buffer {
            buffer.lock().unwrap().push_str(&file_output);
        } else if let Some(ref mut writer) = output_writer {
            writer.write_all(file_output.as_bytes())
                .context("Failed to write to output")?;
        } else {
            // Fallback to stdout if no writer is set up
            print!("{}", file_output);
        }
    }

    // Handle clipboard if needed
    if let Some(buffer) = content_buffer {
        let content = buffer.lock().unwrap().clone();
        match Clipboard::new().and_then(|mut clip| clip.set_text(content.clone())) {
            Ok(_) => {
                // Successfully copied to clipboard, don't print content
            }
            Err(_) => {
                // Clipboard failed, write to stdout instead
                print!("{}", content);
            }
        }
    }

    // Close output file if needed
    if let Some(mut writer) = output_writer {
        writer.flush().context("Failed to flush final output")?;
    }

    // Print statistics to stdout (not stderr) so they appear with the output
    let final_lines = total_lines.load(Ordering::Relaxed);
    let final_tokens = total_tokens.load(Ordering::Relaxed);

    println!("Lines: {}", final_lines);
    println!("Tokens (cl100k_base): {}", final_tokens);

    Ok(())
}
