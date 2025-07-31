# List Files executable (lf)
CLI tool to copy contents of specified files and folders to the clipboard or a file.

### Building
To build the `lf` executable, ensure you have Rust installed and run:
```bash
cargo build --release
```
The executable will be located in `target/release/lf`.

### Example Usage

List everything in the current directory:
```bash
lf .
```

List all files and exclude node_modules and .git directories:
```bash
lf . ~.git ~node_modules
```
