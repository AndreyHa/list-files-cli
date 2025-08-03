# lf - List Files CLI

A fast file aggregation tool that concatenates multiple files based on glob patterns and copies the result to clipboard or file. Perfect for sharing code with AI assistants or creating documentation.

## Features

- 🚀 **Fast**: Parallel processing with Rust
- 📋 **Clipboard integration**: Automatically copies output to clipboard
- 🔍 **Glob patterns**: Flexible file matching with include/exclude patterns
- 🔢 **Token counting**: Built-in token counting for AI context limits
- 📊 **Binary file handling**: Smart detection and metadata for binary files
- 🙈 **Hidden file protection**: Excludes hidden files/directories by default

## Installation

```bash
cargo build --release
```

The executable will be in `target/release/lf`.

## Usage

```bash
lf [PATTERNS...] [OPTIONS]
```

### Basic Examples

```bash
# Get all .rs files in the project
lf *.rs

# Get specific files
lf src/main.rs Cargo.toml README.md

# Get all files in src directory
lf src/

# Get all markdown files recursively
lf **/*.md
```

### Exclusion Examples

Use `~` prefix to exclude patterns:

```bash
# All Rust files except tests
lf **/*.rs ~**/*test*

# Everything except hidden directories and node_modules
lf . ~.git ~node_modules ~.vscode

# All files except specific extensions
lf **/* ~*.log ~*.tmp
```

### Include Hidden Files

Hidden files (starting with `.`) are excluded by default unless explicitly specified:

```bash
# This will NOT include .env, .gitignore, etc.
lf *

# This WILL include .env specifically
lf src/ .env .gitignore

# This will include ALL files including hidden ones
lf .
```

### Output Options

```bash
# Write to file instead of clipboard
lf *.rs -o output.txt

# Disable clipboard (write to stdout)
lf *.rs --no-clipboard

# Pipe to other commands
lf *.rs --no-clipboard | grep "TODO"
```

## Binary Files

Binary files are detected by extension and show metadata instead of content:

```
target/release/lf.exe
[Binary file: EXE - Size: 2.3 MB]
```

Supported binary types: executables, images, videos, audio, archives, documents, and more.
