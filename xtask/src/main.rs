use std::env;
use std::fs;
use std::path::PathBuf;

use conf_ops::api::openapi::generate_openapi_json;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {
    let args: Vec<String> = env::args().collect();
    let check = args.iter().any(|a| a == "--check");

    match args.get(1).map(String::as_str) {
        Some("generate-openapi") => {
            if let Err(e) = generate_openapi(check) {
                eprintln!("error: {e}");
                std::process::exit(1);
            }
        }
        Some(cmd) => {
            eprintln!("Unknown command: {cmd}");
            std::process::exit(1);
        }
        None => {
            eprintln!("Usage: cargo xtask <command>");
            eprintln!("Commands:");
            eprintln!(
                "  generate-openapi [--check]  Generate OpenAPI spec from utoipa annotations"
            );
            std::process::exit(1);
        }
    }
}

fn generate_openapi(check: bool) -> Result<()> {
    let workspace_root = find_workspace_root()?;
    let out_path = workspace_root.join("docs/api/openapi-generated.yaml");

    let json_str =
        generate_openapi_json().map_err(|e| format!("failed to generate OpenAPI: {e}"))?;

    // Convert JSON → YAML
    let json_value: serde_json::Value =
        serde_json::from_str(&json_str).map_err(|e| format!("failed to parse JSON: {e}"))?;
    let yaml_str = serde_yaml::to_string(&json_value)
        .map_err(|e| format!("failed to convert to YAML: {e}"))?;

    if check {
        let existing = fs::read_to_string(&out_path).map_err(|_| {
            "docs/api/openapi-generated.yaml not found — run `cargo xtask generate-openapi` first"
        })?;
        if existing == yaml_str {
            println!("generate-openapi: spec is up-to-date");
            Ok(())
        } else {
            Err("docs/api/openapi-generated.yaml is out of date — run `cargo xtask generate-openapi` to regenerate".into())
        }
    } else {
        fs::create_dir_all(out_path.parent().expect("output path should have parent"))?;
        fs::write(&out_path, &yaml_str)?;
        println!("generate-openapi: wrote {}", out_path.display());
        Ok(())
    }
}

fn find_workspace_root() -> Result<PathBuf> {
    let mut dir = env::current_dir()?;
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists() {
            let content = fs::read_to_string(&cargo_toml)?;
            if content.contains("[workspace]") {
                return Ok(dir);
            }
        }
        if !dir.pop() {
            return Err("could not find workspace root (no Cargo.toml with [workspace])".into());
        }
    }
}
