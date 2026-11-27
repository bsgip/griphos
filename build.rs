use std::fs;
use std::io::Write;
use std::path::Path;
use schemars::schema::RootSchema;
use typify::{TypeSpace, TypeSpaceSettings};

fn main() {
    // Read Cargo.toml
    let cargo_toml = fs::read_to_string("Cargo.toml")
        .expect("Failed to read Cargo.toml");
    let cargo: toml::Value = toml::from_str(&cargo_toml)
        .expect("Failed to parse Cargo.toml");

    // Get the e-JSON schema URL
    let e_json_schema_url = cargo["package"]["metadata"]["e_json_schema"]["url"]
        .as_str()
        .expect("Missing schema URL in Cargo.toml");

    // Read the schema from E_JSON_SCHEMA_PATH if set (e.g. for offline builds), otherwise fetch it.
    println!("cargo:rerun-if-env-changed=E_JSON_SCHEMA_PATH");
    let e_json_schema = match std::env::var("E_JSON_SCHEMA_PATH") {
        Ok(path) => {
            println!("cargo:rerun-if-changed={path}");
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("Failed to read E_JSON_SCHEMA_PATH {path}: {e}"))
        }
        Err(_) => ureq::get(e_json_schema_url)
            .call()
            .expect("Failed to fetch schema")
            .body_mut()
            .read_to_string()
            .expect("Failed to read e_json_schema"),
    };

    // For debugging:
    // let mut file = fs::File::create("e-json-schema.json").expect("Failed to create file");
    // write!(file, "{}", e_json_schema).unwrap();
    let root: RootSchema = serde_json::from_str(&e_json_schema).expect("Failed to parse e_json_schema");

    // `component` discriminates the seven component kinds (Node, Infeeder, Gen,
    // Load, Line, Connector, Transformer) via if/then, which typify can't
    // process - it's excluded here and hand-written instead as a
    // `#[serde(tag = "type")]` enum (see src/....rs).
    let defs: Vec<_> = root.definitions.into_iter().filter(|(name, _)| name != "component").collect();

    let mut type_space = TypeSpace::new(&TypeSpaceSettings::default());
    type_space.add_ref_types(defs).expect("Failed to process schema definitions");
    // Write to file
    let mut file = fs::File::create("src/e_json_generated_types.rs")
        .expect("Failed to create file");
    write!(file, "{}", type_space.to_stream())
        .expect("Failed to write generated types");

    println!("cargo:rerun-if-changed=Cargo.toml");
    println!("cargo:rerun-if-changed=build.rs");

    // existing detection
    let dir_path = Path::new("griphos_extras/src");
    if dir_path.is_dir() {
        println!("cargo:rustc-cfg=has_griphos_extras");
        println!("cargo:rerun-if-changed=griphos_extras/src");
    }

    println!("cargo:rerun-if-changed=griphos_extras/src");
    let mod_path = Path::new("griphos_extras/src/mod.rs");
    if mod_path.is_file() {
        println!("cargo:rustc-cfg=has_griphos_extras");
    }
    println!("cargo:rustc-check-cfg=cfg(has_griphos_extras)");
}
