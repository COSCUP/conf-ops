use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.get(1).map(String::as_str) {
        Some("generate-api-types") => {
            println!("generate-api-types: not yet implemented");
        }
        Some(cmd) => {
            eprintln!("Unknown command: {cmd}");
            std::process::exit(1);
        }
        None => {
            eprintln!("Usage: cargo xtask <command>");
            eprintln!("Commands:");
            eprintln!("  generate-api-types  Generate Rust types from OpenAPI spec");
            std::process::exit(1);
        }
    }
}
