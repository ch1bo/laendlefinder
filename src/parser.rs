use anyhow::{Result, Context};
use regex::Regex;

pub fn extract_price(text: &str) -> Result<f64> {
    let re = Regex::new(r"um\s+([\d,.]+)\s+Euro").unwrap();
    let captures = re.captures(text)
        .context("Price not found in text")?;
    
    let price_str = captures.get(1).unwrap().as_str();
    // Convert price string to f64, handling different formats
    let price_str = price_str.replace(".", "").replace(",", ".");
    let price = price_str.parse::<f64>()
        .context("Failed to parse price as number")?;
    
    Ok(price)
}

pub fn extract_location(text: &str) -> Result<String> {
    let re = Regex::new(r"in\s+([A-Za-zÄÖÜäöüß-]+)").unwrap();
    let captures = re.captures(text)
        .context("Location not found in text")?;
    
    let location = captures.get(1).unwrap().as_str().to_string();
    Ok(location)
}

pub fn extract_property_type(text: &str) -> Result<String> {
    let re = Regex::new(r"eine\s+([A-Za-zÄÖÜäöüß-]+)").unwrap();
    let captures = re.captures(text)
        .context("Property type not found in text")?;
    
    let property_type = captures.get(1).unwrap().as_str().to_string();
    Ok(property_type)
}
