# lf - List Files CLI

lf – lightning-fast project snapshotter (code + docs ⇢ clipboard / file)

## TL;DR

````bash
# everything except hidden + Node artefacts
lf . ~.git ~node_modules        # → clipboard

# whole project → file
lf . -o gist.txt

# only src/, exclude utils/
lf src ~src/utils/**/*.ts
````

## Installation

```bash
# Full build with token counting (default)
cargo build --release

# Slim build (no token counting, smaller binary)
cargo build --release --no-default-features
```

* On Windows, the default binary is ~5.0 MB
* With `--no-default-features`, it shrinks to ~1.07 MB

The executable will be in `target/release/lf`.

## Concepts

* **Include / Exclude** – glob, prefix `~` to drop.
* **Hidden rule** – any path with `/.` ignored *unless* your pattern also starts with `.` or contains `/.`.
* **Directory shorthand** – bare dir name ⇒ `<dir>/**`.

## Cheat-sheet

| Intention               | Command           |
| ----------------------- | ----------------- |
| Add single hidden file  | `lf . .env`       |
| Add hidden dir          | `lf . .obsidian`  |
| Everything, even hidden | `lf "**/*"`       |
| Only Rust tests         | `lf **/*_test.rs` |

## Examples

### Basic Usage

```bash
# Get all .rs files in the project
lf *.rs

# Get specific files
lf src/main.rs Cargo.toml README.md

# Get all files in src directory
lf src/
```

### Exclusion Examples

```bash
# All Rust files except tests
lf **/*.rs ~**/*test*

# Everything except hidden directories and node_modules
lf . ~.git ~node_modules ~.vscode
```

### Hidden Files

Hidden files (starting with `.`) are excluded by default unless explicitly
specified:

```bash
# This will NOT include .env, .gitignore, etc.
lf *

# This WILL include .env specifically
lf src/ .env .gitignore

# This will include ALL files including hidden ones
lf "**/*"
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

## Appendix

### Binary Files

Binary files are detected by extension and show metadata instead of content:

```
target/release/lf.exe
[Binary file: EXE - Size: 2.3 MB]
```

Supported binary types: executables, images, videos, audio, archives,
documents, and more.

### Hidden Path Semantics

"Hidden" means any path component beginning with dot – aligns with POSIX and
Git-ignore semantics.
