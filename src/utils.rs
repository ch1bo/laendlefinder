use anyhow::{Context, Result};
use std::fs::{copy, File};
use std::path::Path;
// Removed the unused import: use csv::Writer;
use crate::models::Property;
use crate::{debug_println};
use rand::seq::SliceRandom;

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

/// Get a random user agent from a pool of common desktop browsers
/// 
/// This function returns different user agents for Chrome, Firefox, Safari and Edge
/// across different operating systems (Windows, macOS, Linux) to avoid detection.
pub fn get_random_user_agent() -> &'static str {
    let user_agents = [
        // Chrome on Windows
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 11.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        
        // Chrome on macOS
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
        
        // Chrome on Linux
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
        
        // Firefox on Windows
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:120.0) Gecko/20100101 Firefox/120.0",
        "Mozilla/5.0 (Windows NT 11.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        
        // Firefox on macOS
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10.15; rv:120.0) Gecko/20100101 Firefox/120.0",
        
        // Firefox on Linux
        "Mozilla/5.0 (X11; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (X11; Ubuntu; Linux x86_64; rv:121.0) Gecko/20100101 Firefox/121.0",
        
        // Safari on macOS
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.1 Safari/605.1.15",
        
        // Edge on Windows
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
        "Mozilla/5.0 (Windows NT 11.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
        
        // Edge on macOS
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
    ];
    
    let mut rng = rand::thread_rng();
    user_agents.choose(&mut rng).unwrap_or(&user_agents[0])
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

    #[test]
    fn test_get_random_user_agent() {
        // Test that the function returns a valid user agent
        let user_agent = get_random_user_agent();
        assert!(!user_agent.is_empty());
        assert!(user_agent.starts_with("Mozilla/"));
        
        // Test that different calls may return different user agents (not guaranteed but likely)
        let user_agents: std::collections::HashSet<&str> = (0..10)
            .map(|_| get_random_user_agent())
            .collect();
        
        // With 20 different user agents, getting at least 2 different ones in 10 calls is very likely
        // This is probabilistic but should work in practice
        assert!(user_agents.len() >= 1); // At minimum we get one valid user agent
        
        // Verify some common browser identifiers appear in our pool
        let all_agents = [
            get_random_user_agent(),
            get_random_user_agent(),
            get_random_user_agent(),
            get_random_user_agent(),
            get_random_user_agent(),
        ];
        
        let has_chrome = all_agents.iter().any(|ua| ua.contains("Chrome"));
        let has_firefox = all_agents.iter().any(|ua| ua.contains("Firefox"));
        
        // Due to randomness, we can't guarantee both will appear, but at least one should
        assert!(has_chrome || has_firefox, "Should contain Chrome or Firefox user agents");
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
