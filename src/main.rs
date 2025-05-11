use crate::models::Property;
use anyhow::Result;
use std::env;

mod models;
mod parser;
mod scraper;
mod utils;

fn main() -> Result<()> {
    // Get command line arguments
    let args: Vec<String> = env::args().collect();
    
    // Check if a URL was provided as an argument
    let url = if args.len() > 1 {
        &args[1]
    } else {
        // If no URL provided, scrape the index page to find property listings
        let property_urls = scraper::scrape_index_page()?;
        
        if property_urls.is_empty() {
            eprintln!("No property URLs found on the index page");
            return Ok(());
        }
        
        // Use the first property URL found
        &property_urls[0]
    };
    
    println!("Processing property URL: {}", url);
    
    // Optional cookie string can be provided as the second argument
    let cookies = args.get(2);
    
    // Scrape the property page
    let property = scraper::scrape_property_page(url, cookies.map(|s| s.as_str()))?;
    
    println!("Scraped property: {:?}", property);
    
    // Load existing properties from CSV
    let existing_properties = utils::load_properties_from_csv("properties.csv")?;
    
    // Check if this property is new or different from existing ones
    let properties = vec![property];
    let unique_properties = utils::compare_properties(&existing_properties, &properties);
    
    if unique_properties.is_empty() {
        println!("No new properties found");
        return Ok(());
    }
    
    // Save the properties to CSV
    utils::save_properties_to_csv(&properties, "properties.csv")?;
    
    println!("Done!");
    Ok(())
}
