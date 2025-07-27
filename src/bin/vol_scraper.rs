use anyhow::Result;
use clap::Parser;
use laendlefinder::common_scraper::{ScrapingOptions, run_scraper_with_options};
use laendlefinder::scrapers::VolScraper;

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
    
    // Create scraping options
    let options = ScrapingOptions {
        output_file: args.output,
        max_pages: args.max_pages,
        max_items: args.max_items,
        refresh: args.refresh,
        cookies: args.cookies,
    };
    
    // Run vol.at scraper with new simplified API
    let vol_scraper = VolScraper;
    run_scraper_with_options(&vol_scraper, &options)?;
    
    Ok(())
}
