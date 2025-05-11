mod scraper;
mod parser;
mod models;
mod utils;

use anyhow::Result;

fn main() -> Result<()> {
    println!("Starting LeandleFinder scraper...");
    
    // Scrape the index page
    let listings = scraper::scrape_index_page()?;
    println!("Found {} property listings", listings.len());
    
    // Process each listing
    let mut properties = Vec::new();
    for listing_url in listings {
        match scraper::scrape_property_page(&listing_url) {
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
