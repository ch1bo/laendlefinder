use anyhow::Result;
use clap::Parser;
use laendlefinder::models::{Property, ListingType};
use laendlefinder::{scraper, laendleimmo_scraper, utils};
use std::collections::HashSet;

#[derive(Parser, Debug)]
#[clap(author, version, about = "Laendlefinder - Property Scraper for Vorarlberg")]
struct Args {
    /// Path to output CSV file
    #[clap(short, long, default_value = "properties.csv")]
    output: String,
    
    /// Optional cookies for vol.at authenticated requests
    #[clap(short, long, default_value = "cookies.txt")]
    cookies: Option<String>,
    
    /// Maximum number of pages to scrape per platform
    #[clap(short, long, default_value = "1")]
    max_pages: usize,
    
    /// Maximum number of items to scrape per platform (if not set, scrape all available items)
    #[clap(short = 'i', long)]
    max_items: Option<usize>,
    
    /// Skip vol.at scraper
    #[clap(long)]
    skip_vol: bool,
    
    /// Skip laendleimmo.at scraper
    #[clap(long)]
    skip_laendleimmo: bool,
}

fn scrape_new_properties_vol(existing_properties: &[Property], property_urls: Vec<String>, cookies: Option<&str>, max_items: Option<usize>) -> Result<Vec<Property>> {
    let mut new_properties = Vec::new();
    
    // Create a set of existing URLs for faster lookup
    let existing_urls: HashSet<String> = existing_properties
        .iter()
        .map(|p| p.url.clone())
        .collect();
    
    // Only scrape properties that aren't already in our database
    for url in property_urls {
        // Check if we've reached the maximum number of items
        if let Some(max) = max_items {
            if new_properties.len() >= max {
                println!("Reached maximum number of items ({}), stopping", max);
                break;
            }
        }
        
        if !existing_urls.contains(&url) {
            println!("Scraping new property: {}", url);
            match scraper::scrape_property_page(&url, cookies, ListingType::Sold) {
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

fn scrape_new_properties_laendleimmo(existing_properties: &[Property], property_urls: Vec<String>, max_items: Option<usize>) -> Result<Vec<Property>> {
    let mut new_properties = Vec::new();
    
    // Create a set of existing URLs for faster lookup
    let existing_urls: HashSet<String> = existing_properties
        .iter()
        .map(|p| p.url.clone())
        .collect();
    
    // Only scrape properties that aren't already in our database
    for url in property_urls {
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

fn run_vol_scraper(args: &Args, existing_properties: &[Property]) -> Result<Vec<Property>> {
    println!("Vol.at Property Scraper (Sold Properties)");
    println!("=========================================");
    
    // Get property URLs from index pages up to max_pages
    let property_urls = scraper::scrape_all_index_pages(args.max_pages)?;
    println!("Found {} property URLs", property_urls.len());
    
    if property_urls.is_empty() {
        println!("No vol.at properties found.");
        return Ok(Vec::new());
    }
    
    // Only scrape new properties
    let new_properties = scrape_new_properties_vol(
        existing_properties, 
        property_urls, 
        args.cookies.as_deref(),
        args.max_items
    )?;
    println!("Scraped {} new vol.at properties", new_properties.len());
    
    Ok(new_properties)
}

fn run_laendleimmo_scraper(args: &Args, existing_properties: &[Property]) -> Result<Vec<Property>> {
    println!("\nLaendleimmo.at Property Scraper (Available Properties)");
    println!("====================================================");
    
    // Get property URLs from listing pages up to max_pages
    let property_urls = laendleimmo_scraper::scrape_all_listing_pages(args.max_pages)?;
    println!("Found {} property URLs", property_urls.len());
    
    if property_urls.is_empty() {
        println!("No laendleimmo.at properties found.");
        return Ok(Vec::new());
    }
    
    // Only scrape new properties
    let new_properties = scrape_new_properties_laendleimmo(
        existing_properties, 
        property_urls, 
        args.max_items
    )?;
    println!("Scraped {} new laendleimmo.at properties", new_properties.len());
    
    Ok(new_properties)
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("Laendlefinder - Property Scraper for Vorarlberg");
    println!("===============================================");
    
    // Load existing properties first
    let mut all_properties = utils::load_properties_from_csv(&args.output)?;
    println!("Loaded {} existing properties", all_properties.len());
    
    let mut total_new_properties = 0;
    
    // Run vol.at scraper (sold properties)
    if !args.skip_vol {
        let new_vol_properties = run_vol_scraper(&args, &all_properties)?;
        total_new_properties += new_vol_properties.len();
        all_properties.extend(new_vol_properties);
    } else {
        println!("Skipping vol.at scraper");
    }
    
    // Run laendleimmo.at scraper (available properties)
    if !args.skip_laendleimmo {
        let new_laendleimmo_properties = run_laendleimmo_scraper(&args, &all_properties)?;
        total_new_properties += new_laendleimmo_properties.len();
        all_properties.extend(new_laendleimmo_properties);
    } else {
        println!("Skipping laendleimmo.at scraper");
    }
    
    if total_new_properties == 0 {
        println!("\nNo new properties to add.");
        return Ok(());
    }
    
    // Save all properties to CSV (with backup)
    utils::save_properties_to_csv(&all_properties, &args.output)?;
    
    println!("\n=== Summary ===");
    println!("Total new properties scraped: {}", total_new_properties);
    println!("Total properties in database: {}", all_properties.len());
    println!("Saved to: {}", args.output);
    
    Ok(())
}
