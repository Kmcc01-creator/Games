pub mod schema;

use anyhow::Result;
use schema::AssetDNA;

pub fn load_from_yaml_str(s: &str) -> Result<AssetDNA> {
    let dna: AssetDNA = serde_yaml::from_str(s)?;
    Ok(dna)
}

pub fn load_from_path<P: AsRef<std::path::Path>>(path: P) -> Result<AssetDNA> {
    let data = std::fs::read_to_string(path)?;
    load_from_yaml_str(&data)
}

