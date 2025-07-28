use anyhow::{Context, Result};
use std::fs::{copy, File};
use std::path::Path;
// Removed the unused import: use csv::Writer;
use crate::models::Property;
use crate::{debug_println};

/// Sanitize URL by removing query parameters and fragments to avoid duplicates
/// 
/// This function removes everything after '?' (query parameters) and '#' (fragments)
/// to ensure that URLs with different parameters are treated as the same property.
pub fn sanitize_url(url: &str) -> String {
    // Find the position of '?' or '#' and take everything before it
    let url = url.split('?').next().unwrap_or(url);
    let url = url.split('#').next().unwrap_or(url);
    url.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_url() {
        // Test URL with query parameters
        assert_eq!(
            sanitize_url("https://example.com/property/123?utm_source=google&ref=search"),
            "https://example.com/property/123"
        );

        // Test URL with fragment
        assert_eq!(
            sanitize_url("https://example.com/property/123#details"),
            "https://example.com/property/123"
        );

        // Test URL with both query parameters and fragment
        assert_eq!(
            sanitize_url("https://example.com/property/123?page=2&sort=price#gallery"),
            "https://example.com/property/123"
        );

        // Test URL without query parameters or fragments
        assert_eq!(
            sanitize_url("https://example.com/property/123"),
            "https://example.com/property/123"
        );

        // Test empty URL
        assert_eq!(sanitize_url(""), "");

        // Test laendleimmo URL example
        assert_eq!(
            sanitize_url("https://www.laendleimmo.at/immobilien/haus/villa/vorarlberg/bregenz/123456?source=feed&campaign=winter"),
            "https://www.laendleimmo.at/immobilien/haus/villa/vorarlberg/bregenz/123456"
        );
    }

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
        let mut property: Property =
            result.with_context(|| "Failed to deserialize property from CSV")?;
        // Sanitize URL to remove query parameters and fragments for deduplication
        property.url = sanitize_url(&property.url);
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
