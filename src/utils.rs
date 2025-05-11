use std::fs::File;
use std::path::Path;
use anyhow::{Result, Context};
use csv::Writer;
use crate::models::Property;
use chrono::NaiveDate;

pub fn compare_properties(existing: &[Property], new: &[Property]) -> Vec<Property> {
    let mut unique_properties = Vec::new();
    
    for new_property in new {
        let mut is_unique = true;
        
        for existing_property in existing {
            // Check if the URL is the same
            if new_property.url == existing_property.url {
                // If the URL is the same, check if any other details have changed
                let existing_date_str = existing_property.date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                let new_date_str = new_property.date
                    .map(|d| d.format("%Y-%m-%d").to_string())
                    .unwrap_or_default();
                
                if new_property.price == existing_property.price &&
                   new_property.location == existing_property.location &&
                   new_property.property_type == existing_property.property_type &&
                   new_date_str == existing_date_str {
                    is_unique = false;
                    break;
                }
            }
        }
        
        if is_unique {
            unique_properties.push(new_property.clone());
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
        
        let coordinates = record.get(6).and_then(|s| {
            if s.is_empty() { 
                None 
            } else {
                let parts: Vec<&str> = s.split(',').collect();
                if parts.len() == 2 {
                    let lat = parts[0].parse::<f64>().ok()?;
                    let lng = parts[1].parse::<f64>().ok()?;
                    Some((lat, lng))
                } else {
                    None
                }
            }
        });
        
        let address = record.get(7).map(|s| {
            if s.is_empty() { None } else { Some(s.to_string()) }
        }).unwrap_or(None);
        
        let size_living = record.get(8).map(|s| {
            if s.is_empty() { None } else { Some(s.to_string()) }
        }).unwrap_or(None);
        
        properties.push(Property {
            url: record.get(0).unwrap_or_default().to_string(),
            price: record.get(1).unwrap_or_default().to_string(),
            location: record.get(2).unwrap_or_default().to_string(),
            property_type: record.get(3).unwrap_or_default().to_string(),
            date,
            description,
            coordinates,
            address,
            size_living,
        });
    }
    
    println!("Loaded {} properties from {}", properties.len(), input_path);
    Ok(properties)
}

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
            "Description",
            "Coordinates",
            "Address",
            "Size Living"
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
