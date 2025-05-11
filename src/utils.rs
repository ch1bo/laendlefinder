use crate::models::Property;
use anyhow::{Result, Context};
use std::fs::File;
use std::path::Path;
use chrono::NaiveDate;

pub fn save_properties_to_csv(properties: &[Property], output_path: &str) -> Result<()> {
    let path = Path::new(output_path);
    let file_exists = path.exists();
    
    let file = File::create(path)
        .context(format!("Failed to create output file: {}", output_path))?;
    
    let mut writer = csv::Writer::from_writer(file);
    
    // Write header if the file is new
    if !file_exists {
        writer.write_record(&[
            "URL", 
            "Price", 
            "Location", 
            "Type", 
            "Date", 
            "Description"
        ])?;
    }
    
    // Write property records
    for property in properties {
        writer.write_record(&property.to_csv_record())?;
    }
    
    writer.flush()?;
    println!("Saved {} properties to {}", properties.len(), output_path);
    
    Ok(())
}

pub fn compare_properties(old_properties: &[Property], new_properties: &[Property]) -> Vec<Property> {
    let mut unique_properties = Vec::new();
    
    for new_prop in new_properties {
        let is_unique = old_properties.iter().all(|old_prop| {
            // Compare the key fields to determine if it's a new property
            // We consider a property unique if any of these fields differ
            new_prop.url != old_prop.url ||
            new_prop.price != old_prop.price ||
            new_prop.location != old_prop.location ||
            new_prop.property_type != old_prop.property_type ||
            new_prop.date.map(|d| d.format("%Y-%m-%d").to_string()) != 
                old_prop.date.map(|d| d.format("%Y-%m-%d").to_string())
        });
        
        if is_unique {
            unique_properties.push(new_prop.clone());
        }
    }
    
    unique_properties
}

pub fn load_properties_from_csv(input_path: &str) -> Result<Vec<Property>> {
    let path = Path::new(input_path);
    
    if !path.exists() {
        println!("CSV file does not exist: {}", input_path);
        return Ok(Vec::new());
    }
    
    let file = File::open(path)
        .context(format!("Failed to open input file: {}", input_path))?;
    
    let mut reader = csv::Reader::from_reader(file);
    let mut properties = Vec::new();
    
    for result in reader.records() {
        let record = result?;
        
        if record.len() < 5 {
            println!("Warning: Skipping record with insufficient fields: {:?}", record);
            continue;
        }
        
        let date_str = record.get(4).unwrap_or_default();
        let date = if !date_str.is_empty() {
            NaiveDate::parse_from_str(date_str, "%Y-%m-%d").ok()
        } else {
            None
        };
        
        let description = record.get(5).map(|s| {
            if s.is_empty() { None } else { Some(s.to_string()) }
        }).unwrap_or(None);
        
        properties.push(Property {
            url: record.get(0).unwrap_or_default().to_string(),
            price: record.get(1).unwrap_or_default().to_string(),
            location: record.get(2).unwrap_or_default().to_string(),
            property_type: record.get(3).unwrap_or_default().to_string(),
            date,
            description,
        });
    }
    
    println!("Loaded {} properties from {}", properties.len(), input_path);
    Ok(properties)
}
