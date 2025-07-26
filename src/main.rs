use anyhow::Result;
use clap::Parser;
use laendlefinder::common_scraper::{ScrapingOptions, run_scraper_with_options, merge_properties_with_refresh};
use laendlefinder::scrapers::{VolScraper, LaendleimmoScraper};
use laendlefinder::utils;

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
    
    /// Re-scrape already known URLs to refresh data
    #[clap(short, long)]
    refresh: bool,
    
    /// Skip vol.at scraper
    #[clap(long)]
    skip_vol: bool,
    
    /// Skip laendleimmo.at scraper
    #[clap(long)]
    skip_laendleimmo: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    println!("Laendlefinder - Property Scraper for Vorarlberg");
    println!("===============================================");
    
    // Load existing properties first
    let mut all_properties = utils::load_properties_from_csv(&args.output)?;
    println!("Loaded {} existing properties", all_properties.len());
    
    let mut total_new_properties = 0;
    
    // Create scraping options
    let options = ScrapingOptions {
        output_file: args.output.clone(),
        max_pages: args.max_pages,
        max_items: args.max_items,
        refresh: args.refresh,
        cookies: args.cookies.clone(),
    };
    
    // Run vol.at scraper (sold properties)
    if !args.skip_vol {
        let vol_scraper = VolScraper;
        let vol_result = run_scraper_with_options(&vol_scraper, &options)?;
        total_new_properties += vol_result.scraped_properties.len();
        
        all_properties = merge_properties_with_refresh(all_properties, vol_result, "vol.at");
    } else {
        println!("Skipping vol.at scraper");
    }
    
    // Run laendleimmo.at scraper (available properties)
    if !args.skip_laendleimmo {
        let laendleimmo_scraper = LaendleimmoScraper;
        let laendleimmo_result = run_scraper_with_options(&laendleimmo_scraper, &options)?;
        total_new_properties += laendleimmo_result.scraped_properties.len();
        
        all_properties = merge_properties_with_refresh(all_properties, laendleimmo_result, "laendleimmo.at");
    } else {
        println!("Skipping laendleimmo.at scraper");
    }
    
    if total_new_properties == 0 && !args.refresh {
        println!("\nNo new properties to add.");
        return Ok(());
    }
    
    // Save all properties to CSV (with backup)
    utils::save_properties_to_csv(&all_properties, &args.output)?;
    
    println!("\n=== Summary ===");
    if args.refresh {
        println!("Total properties scraped/refreshed: {}", total_new_properties);
    } else {
        println!("Total new properties scraped: {}", total_new_properties);
    }
    println!("Total properties in database: {}", all_properties.len());
    println!("Saved to: {}", args.output);
    
    Ok(())
}
