use std::fs;
use std::io::Write;
use std::path::Path;
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

    // Fetch the schema
    let e_json_schema = ureq::get(e_json_schema_url)
        .call()
        .expect("Failed to fetch schema")
        .body_mut()
        .read_to_string()
        .expect("Failed to read e_json_schema");

    // For debugging:
    // let mut file = fs::File::create("e-json-schema.json").expect("Failed to create file");
    // write!(file, "{}", e_json_schema).unwrap();

    // Generate Rust types
    let mut type_space = TypeSpace::new(&TypeSpaceSettings::default());
    type_space.add_root_schema(serde_json::from_str(&e_json_schema).unwrap())
        .expect("Failed to process schema");

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
