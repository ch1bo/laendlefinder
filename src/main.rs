mod scraper;
mod parser;
mod models;
mod utils;

use anyhow::Result;
use std::path::Path;
use std::env;
use std::fs;

fn main() -> Result<()> {
    println!("Starting LeandleFinder scraper...");
    
    // Get cookies file path from command line arguments or use default
    let cookies_path = env::args().nth(1).unwrap_or_else(|| "cookies.txt".to_string());
    let cookies_path = Path::new(&cookies_path);
    
    // Read cookies from file if it exists
    let cookies = if cookies_path.exists() {
        println!("Using cookies from {:?}", cookies_path);
        Some(fs::read_to_string(cookies_path)?)
    } else {
        eprintln!("Warning: Cookies file not found at {:?}. Proceeding without authentication.", cookies_path);
        None
    };
    
    // Scrape the index page
    let listings = scraper::scrape_index_page()?;
    println!("Found {} property listings", listings.len());
    
    // Process each listing
    let mut properties = Vec::new();
    for listing_url in listings {
        match scraper::scrape_property_page(&listing_url, cookies.as_deref()) {
            Ok(property) => {
                println!("Scraped property: {} in {} for {} Euro", 
                    property.property_type, property.location, property.price);
                properties.push(property);
            },
            Err(e) => {
                eprintln!("Error scraping {}: {}", listing_url, e);
            }
        }
    }
    
    // Save to CSV
    utils::save_to_csv(&properties, "properties.csv")?;
    println!("Saved {} properties to properties.csv", properties.len());
    
    Ok(())
}
