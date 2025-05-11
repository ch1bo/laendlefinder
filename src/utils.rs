use anyhow::Result;
use csv::Writer;
use std::fs::File;
use std::fs;
use crate::models::Property;

pub fn read_cookies_file(path: &str) -> Result<String> {
    let content = fs::read_to_string(path)
        .map_err(|e| anyhow::anyhow!("Failed to read cookies file: {}", e))?;
    
    // Strip newlines and any extra whitespace
    let cleaned_cookies = content
        .lines()
        .collect::<Vec<&str>>()
        .join("")
        .trim()
        .to_string();
    
    // Further sanitize the cookie string to ensure it's a valid header value
    // Remove any characters that might cause issues in HTTP headers
    let sanitized_cookies = cleaned_cookies
        .chars()
        .filter(|&c| !c.is_control() && c != '\r' && c != '\n')
        .collect::<String>();
    
    Ok(sanitized_cookies)
}

pub fn save_to_csv(properties: &[Property], filename: &str) -> Result<()> {
    let file = File::create(filename)?;
    let mut writer = Writer::from_writer(file);
    
    // Write header
    writer.write_record(&[
        "URL", 
        "Price", 
        "Location", 
        "Property Type", 
        "Date", 
        "Description"
    ])?;
    
    // Write data
    for property in properties {
        writer.write_record(&[
            &property.url,
            &property.price.to_string(),
            &property.location,
            &property.property_type,
            &property.date.clone().unwrap_or_default(),
            &property.description.clone().unwrap_or_default(),
        ])?;
    }
    
    writer.flush()?;
    Ok(())
}
