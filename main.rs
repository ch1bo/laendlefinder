use anyhow::Result;
use clap::Parser;
use std::collections::HashSet;
use crate::models::Property;

mod models;
mod parser;
mod scraper;
mod utils;

#[derive(Parser)]
#[command(author, version, about)]
struct Args {
    /// Path to output CSV file
    #[arg(short, long, default_value = "properties.csv")]
    output: String,
    
    /// Optional cookies for authenticated requests
    #[arg(short, long)]
    cookies: Option<String>,
    
    /// Maximum number of pages to scrape
    #[arg(short, long, default_value = "5")]
    max_pages: usize,
}

fn scrape_new_properties(existing_properties: &[Property], property_urls: Vec<String>, cookies: Option<&str>) -> Result<Vec<Property>> {
    let mut new_properties = Vec::new();
    
    // Create a set of existing URLs for faster lookup
    let existing_urls: HashSet<String> = existing_properties
        .iter()
        .map(|p| p.url.clone())
        .collect();
    
    // Only scrape properties that aren't already in our database
    for url in property_urls {
        if !existing_urls.contains(&url) {
            println!("Scraping new property: {}", url);
            match scraper::scrape_property_page(&url, cookies) {
                Ok(property) => {
                    new_properties.push(property);
                },
                Err(e) => {
                    eprintln!("Error scraping property {}: {}", url, e);
                }
            }
        } else {
            println!("Skipping already known property: {}", url);
        }
    }
    
    Ok(new_properties)
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Load existing properties first
    let existing_properties = utils::load_properties_from_csv(&args.output)?;
    println!("Loaded {} existing properties", existing_properties.len());
    
    // Get property URLs from all pages up to max_pages
    let property_urls = scraper::scrape_all_index_pages(args.max_pages)?;
    println!("Found {} property URLs across {} pages", property_urls.len(), args.max_pages);
    
    // Only scrape new properties
    let new_properties = scrape_new_properties(&existing_properties, property_urls, args.cookies.as_deref())?;
    println!("Scraped {} new properties", new_properties.len());
    
    // Combine existing and new properties
    let mut all_properties = existing_properties.clone();
    all_properties.extend(new_properties);
    
    // Save all properties to CSV
    utils::save_properties_to_csv(&all_properties, &args.output)?;
    
    Ok(())
}
