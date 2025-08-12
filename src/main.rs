use anyhow::{Context, Result};
use arboard::Clipboard;
use clap::Parser;
use globset::{Glob, GlobSet, GlobSetBuilder};
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
#[cfg(feature = "token-counting")]
use tiktoken_rs::o200k_base;
use walkdir::WalkDir;

/// Trait for abstracting tokenization
trait Tokenizer: Send + Sync {
    fn count_tokens(&self, text: &str) -> usize;
}

#[cfg(feature = "token-counting")]
/// Implementation using the o200k_base tokenizer
struct O200kTokenizer {
    bpe: tiktoken_rs::CoreBPE,
}

#[cfg(feature = "token-counting")]
impl O200kTokenizer {
    fn new() -> Result<Self> {
        let bpe = o200k_base().context("Failed to initialize o200k_base tokenizer")?;
        Ok(Self { bpe })
    }
}

#[cfg(feature = "token-counting")]
impl Tokenizer for O200kTokenizer {
    fn count_tokens(&self, text: &str) -> usize {
        self.bpe.encode_with_special_tokens(text).len()
    }
}

#[cfg(not(feature = "token-counting"))]
/// Dummy tokenizer that always returns 0
struct DummyTokenizer;

#[cfg(not(feature = "token-counting"))]
impl Tokenizer for DummyTokenizer {
    fn count_tokens(&self, _text: &str) -> usize {
        0
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

/// A glob is considered "hidden-explicit" iff it starts with '.' or contains '/.'
fn is_hidden_glob(glob: &str) -> bool {
    let g = glob.trim_start_matches("./");
    (g.starts_with('.') && g.len() > 1) || g.contains("/.")
}

fn build_glob_sets(patterns: &[String]) -> Result<(GlobSet, GlobSet, GlobSet)> {
    let mut vis_inc = GlobSetBuilder::new();     // visible includes
    let mut hid_inc = GlobSetBuilder::new();     // hidden includes
    let mut exc     = GlobSetBuilder::new();     // excludes (same as before)

    for p in patterns {
        if let Some(raw) = p.strip_prefix('~') {
            exc.add(Glob::new(raw)?);
            continue;
        }

        // normalise convenience shorthand
        let norm = match p.as_str() {
            "." | "./"                      => "**/*",
            dir if dir.ends_with('/')       => &format!("{dir}**/*"),
            // NEW – treat bare hidden dir the same way we already treat visible dirs
            dir if dir.starts_with('.') 
                 && !dir.contains(['*', '/']) => &format!("{dir}/**"),
            dir if !dir.contains(['*', '/', '.']) 
                                             => &format!("{dir}/**"),
            _ => p,
        };

        if is_hidden_glob(norm) {
            hid_inc.add(Glob::new(norm)?);
        } else {
            vis_inc.add(Glob::new(norm)?);
        }
    }

    Ok((vis_inc.build()?, hid_inc.build()?, exc.build()?))
}

#[derive(Parser)]
#[command(name = "lf")]
#[command(about = "A fast file aggregation tool with glob patterns and tokenization. Hidden paths are skipped unless a pattern containing a '.' at the first path-segment is supplied.")]
struct Args {
    /// One or more glob patterns; prefix with `~` to exclude matches
    patterns: Vec<String>,

    /// Write concatenated content to file instead of clipboard
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Do not copy content to the clipboard
    #[arg(short, long)]
    no_clipboard: bool,

    /// Replace Java import lines with `import ...`
    #[arg(long)]
    mask_java_imports: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.patterns.is_empty() {
        eprintln!("Error: At least one pattern must be provided");
        std::process::exit(1);
    }

    // Build include and exclude glob sets
    let (include_set, hidden_include_set, exclude_set) = build_glob_sets(&args.patterns)?;

    // Collect matching files
    let files: Vec<PathBuf> = WalkDir::new(".")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|path| {
            let path_str = path.to_string_lossy().replace('\\', "/");
            let stripped = path_str.strip_prefix("./").unwrap_or(&path_str);
            let file = path.file_name().map(|f| f.to_string_lossy().to_string()).unwrap_or_default();

            let hidden = stripped.split('/').any(|c| c.starts_with('.') && c != "." && c != "..");

            let inc = if hidden {
                hidden_include_set.is_match(&path_str)
                    || hidden_include_set.is_match(stripped)
                    || hidden_include_set.is_match(&file)
            } else {
                include_set.is_match(&path_str)
                    || include_set.is_match(stripped)
                    || include_set.is_match(&file)
            };

            inc && !exclude_set.is_match(&path_str) && !exclude_set.is_match(stripped) && !exclude_set.is_match(&file)
        })
        .collect();

    if files.is_empty() {
        println!("No files found matching the patterns.");
        return Ok(());
    }

    // Initialize tokenizer
    #[cfg(feature = "token-counting")]
    let tokenizer = Arc::new(O200kTokenizer::new()?);
    #[cfg(not(feature = "token-counting"))]
    let tokenizer = Arc::new(DummyTokenizer);

    // Process files in parallel
    let total_lines = Arc::new(AtomicUsize::new(0));
    let total_tokens = Arc::new(AtomicUsize::new(0));

    // Expose mask option to worker threads
    let mask_java_imports = args.mask_java_imports;

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

            // If requested, mask Java import lines before token counting
            if mask_java_imports {
                if let Some(ext) = file_path.extension().and_then(|e| e.to_str()) {
                    if ext.eq_ignore_ascii_case("java") {
                        let mut new_content = String::new();
                        let mut in_import_block = false;
                        for line in content.lines() {
                            if line.trim_start().starts_with("import ") {
                                if !in_import_block {
                                    // start of an import block -> emit single placeholder
                                    new_content.push_str("import ...\n");
                                    in_import_block = true;
                                }
                                // skip subsequent import lines
                            } else {
                                // any non-import line resets the import-block state
                                new_content.push_str(line);
                                new_content.push('\n');
                                in_import_block = false;
                            }
                        }
                        // If there were no imports, keep original content (avoid accidental removal)
                        if new_content.contains("import ...") {
                            content = new_content;
                        }
                    }
                }
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
    #[cfg(feature = "token-counting")]
    println!("Tokens (o200k_base): {}", final_tokens);

    Ok(())
}
