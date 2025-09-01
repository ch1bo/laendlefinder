use anyhow::Result;
use clap::Parser;
use laendlefinder::common_scraper::{ScrapingOptions, run_scraper_with_options};
use laendlefinder::scrapers::LaendleimmoScraper;

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
    
    /// Scrape new URLs until no new ones found in 5 consecutive pages (default mode)
    #[clap(short, long, default_value = "true")]
    new: bool,
    
    /// Enable debug output
    #[clap(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Create scraping options
    let options = ScrapingOptions {
        output_file: args.output,
        max_pages: args.max_pages,
        max_items: args.max_items,
        refresh: args.refresh,
        new: args.new,
        cookies: None, // laendleimmo doesn't use cookies
        debug: args.debug,
    };
    
    // Run laendleimmo.at scraper with new simplified API
    let laendleimmo_scraper = LaendleimmoScraper;
    run_scraper_with_options(&laendleimmo_scraper, &options)?;
    
    Ok(())
}