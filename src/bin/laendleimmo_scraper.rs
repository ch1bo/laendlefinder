use anyhow::Result;
use clap::Parser;
use laendlefinder::common_scraper::{ScrapingOptions, run_scraper_with_options, merge_properties_with_refresh};
use laendlefinder::scrapers::LaendleimmoScraper;
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
    
    /// Re-scrape already known URLs to refresh data
    #[clap(short, long)]
    refresh: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Load existing properties first
    let mut all_properties = utils::load_properties_from_csv(&args.output)?;
    println!("Loaded {} existing properties", all_properties.len());
    
    // Create scraping options
    let options = ScrapingOptions {
        output_file: args.output.clone(),
        max_pages: args.max_pages,
        max_items: args.max_items,
        refresh: args.refresh,
        cookies: None, // laendleimmo doesn't use cookies
    };
    
    // Run laendleimmo.at scraper
    let laendleimmo_scraper = LaendleimmoScraper;
    let laendleimmo_result = run_scraper_with_options(&laendleimmo_scraper, &options)?;
    
    // Merge properties with proper refresh handling
    all_properties = merge_properties_with_refresh(all_properties, laendleimmo_result, "laendleimmo.at");
    
    // Save all properties to CSV (with backup)
    utils::save_properties_to_csv(&all_properties, &args.output)?;
    
    println!("Total properties in database: {}", all_properties.len());
    
    Ok(())
}