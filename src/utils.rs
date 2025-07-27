use anyhow::{Context, Result};
use std::fs::{copy, File};
use std::path::Path;
// Removed the unused import: use csv::Writer;
use crate::models::Property;
use crate::{debug_println};

// Since this function is never used, we can either remove it or keep it for future use
// I'll keep it but add an allow attribute to suppress the warning
#[allow(dead_code)]
pub fn compare_properties(existing: &[Property], new: &[Property]) -> Vec<Property> {
    let existing_urls: std::collections::HashSet<String> =
        existing.iter().map(|p| p.url.clone()).collect();

    new.iter()
        .filter(|p| !existing_urls.contains(&p.url))
        .cloned()
        .collect()
}

pub fn load_properties_from_csv(path: &str) -> Result<Vec<Property>> {
    let path = Path::new(path);

    // If the file doesn't exist, return an empty vector
    if !path.exists() {
        debug_println!(
            "CSV file {} does not exist, creating a new one",
            path.display()
        );
        return Ok(Vec::new());
    }

    let file =
        File::open(path).with_context(|| format!("Failed to open CSV file: {}", path.display()))?;

    let mut reader = csv::Reader::from_reader(file);
    let mut properties = Vec::new();

    for result in reader.deserialize() {
        let property: Property =
            result.with_context(|| "Failed to deserialize property from CSV")?;
        properties.push(property);
    }

    debug_println!(
        "Loaded {} properties from {}",
        properties.len(),
        path.display()
    );

    Ok(properties)
}

pub fn save_properties_to_csv(properties: &[Property], path: &str) -> Result<()> {
    let path_obj = Path::new(path);

    // Create backup if file exists
    if path_obj.exists() {
        let backup_path = "properties_backup.csv";

        copy(path, backup_path)
            .with_context(|| format!("Failed to create backup: {}", backup_path))?;

        debug_println!("Created backup: {}", backup_path);
    }

    let file =
        File::create(path).with_context(|| format!("Failed to create CSV file: {}", path))?;

    let mut writer = csv::Writer::from_writer(file);

    for property in properties {
        writer
            .serialize(property)
            .with_context(|| "Failed to serialize property to CSV")?;
    }

    writer
        .flush()
        .with_context(|| "Failed to flush CSV writer")?;

    debug_println!("Saved {} properties to {}", properties.len(), path);

    Ok(())
}
