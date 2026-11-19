// Reads a JSON array of manifests from stdin and prints a markdown table.
// Run later with: cargo run --example print_manifests
use std::io::{self, Read};
fn main() {
    let mut buf = String::new();
    io::stdin().read_to_string(&mut buf).unwrap();
    println!("{}", buf);
}
