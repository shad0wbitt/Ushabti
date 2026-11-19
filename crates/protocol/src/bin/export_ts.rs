//! TS export binary (run manually).

use kestrel_protocol::*;
use std::fs;
use std::path::Path;

fn main() {
    let out = Path::new("./bindings");
    let _ = fs::create_dir_all(out);
    // ts-rs generates via #[derive(TS)] on types; here we just touch the dir.
    // Types must derive TS for export; left as TODO.
    println!("export dir ready: {}", out.display());
}
EOF