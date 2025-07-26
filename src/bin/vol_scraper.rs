use anyhow::Result;
use clap::Parser;
use laendlefinder::common_scraper::{ScrapingOptions, run_scraper_with_options};
use laendlefinder::scrapers::VolScraper;
use laendlefinder::utils;

#[derive(Parser, Debug)]
#[clap(author, version, about)]
struct Args {
    /// Path to output CSV file
    #[clap(short, long, default_value = "properties.csv")]
    output: String,
    
    /// Optional cookies for authenticated requests
    #[clap(short, long, default_value = "cookies.txt")]
    cookies: Option<String>,
    
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
        cookies: args.cookies.clone(),
    };
    
    // Run vol.at scraper
    let vol_scraper = VolScraper;
    let new_properties = run_scraper_with_options(&vol_scraper, &options)?;
    
    // Handle refresh mode
    if args.refresh {
        // Remove existing vol.at properties and add refreshed ones
        all_properties.retain(|p| !p.url.contains("vol.at"));
        all_properties.extend(new_properties);
    } else {
        all_properties.extend(new_properties);
    }
    
    // Save all properties to CSV (with backup)
    utils::save_properties_to_csv(&all_properties, &args.output)?;
    
    println!("Total properties in database: {}", all_properties.len());
    
    Ok(())
}
