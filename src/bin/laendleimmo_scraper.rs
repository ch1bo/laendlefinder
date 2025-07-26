use anyhow::Result;
use clap::Parser;
use laendlefinder::models::Property;
use laendlefinder::laendleimmo_scraper;
use laendlefinder::utils;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Laendleimmo.at Property Scraper")]
struct Args {
    /// Path to output CSV file
    #[clap(short, long, default_value = "properties.csv")]
    output: String,
    
    /// Maximum number of pages to scrape
    #[clap(short, long, default_value = "1")]
    max_pages: usize,
    
    /// Maximum number of items to scrape (if not set, scrape all available items)
    #[clap(short = 'i', long)]
    max_items: Option<usize>,
}

fn scrape_new_properties(existing_properties: &[Property], property_urls: Vec<String>, max_items: Option<usize>) -> Result<Vec<Property>> {
    let mut new_properties = Vec::new();
    
    // Create a set of existing URLs for faster lookup
    let existing_urls: std::collections::HashSet<String> = existing_properties
        .iter()
        .map(|p| p.url.clone())
        .collect();
    
    // Only scrape properties that aren't already in our database
    for (_index, url) in property_urls.into_iter().enumerate() {
        // Check if we've reached the maximum number of items
        if let Some(max) = max_items {
            if new_properties.len() >= max {
                println!("Reached maximum number of items ({}), stopping", max);
                break;
            }
        }
        
        if !existing_urls.contains(&url) {
            println!("Scraping new property: {}", url);
            match laendleimmo_scraper::scrape_property_page(&url) {
                Ok(property) => {
                    new_properties.push(property);
                },
                Err(e) => {
                    eprintln!("Error scraping property {}: {}", url, e);
                }
            }
            
            // Add a small delay to be respectful to the server
            std::thread::sleep(std::time::Duration::from_millis(500));
        } else {
            println!("Skipping already known property: {}", url);
        }
    }
    
    Ok(new_properties)
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("Laendleimmo.at Property Scraper");
    println!("===============================");
    
    // Load existing properties first
    let existing_properties = utils::load_properties_from_csv(&args.output)?;
    println!("Loaded {} existing properties", existing_properties.len());
    
    // Get property URLs from listing pages up to max_pages
    let property_urls = laendleimmo_scraper::scrape_all_listing_pages(args.max_pages)?;
    println!("Found {} property URLs", property_urls.len());
    
    if property_urls.is_empty() {
        println!("No properties found. Exiting.");
        return Ok(());
    }
    
    // Only scrape new properties
    let new_properties = scrape_new_properties(
        &existing_properties, 
        property_urls, 
        args.max_items
    )?;
    println!("Scraped {} new properties", new_properties.len());
    
    if new_properties.is_empty() {
        println!("No new properties to add.");
        return Ok(());
    }
    
    // Combine existing and new properties
    let mut all_properties = existing_properties.clone();
    all_properties.extend(new_properties);
    
    // Save all properties to CSV (with backup)
    utils::save_properties_to_csv(&all_properties, &args.output)?;
    
    println!("Total properties in database: {}", all_properties.len());
    
    Ok(())
}