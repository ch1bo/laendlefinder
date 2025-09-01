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
    #[clap(short, long)]
    max_pages: Option<usize>,
    
    /// Maximum number of items to scrape (if not set, scrape all available items)
    #[clap(short = 'i', long)]
    max_items: Option<usize>,
    
    /// Re-scrape already known URLs to refresh data
    #[clap(short, long)]
    refresh: bool,
    
    /// Scrape new URLs until no new ones found in 5 consecutive pages (default mode unless max-items is specified)
    #[clap(short, long)]
    new: bool,
    
    /// Enable debug output
    #[clap(short, long)]
    debug: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Create scraping options
    // Use new mode by default, unless other flags are provided
    let use_new_mode = if args.max_items.is_some() || args.max_pages.is_some() || args.refresh {
        args.new // Use explicit --new flag when other options are specified
    } else {
        true // Default to new mode when no specific options provided
    };
    
    let options = ScrapingOptions {
        output_file: args.output,
        max_pages: args.max_pages,
        max_items: args.max_items,
        refresh: args.refresh,
        new: use_new_mode,
        cookies: args.cookies,
        debug: args.debug,
    };
    
    // Run vol.at scraper with new simplified API
    let vol_scraper = VolScraper;
    run_scraper_with_options(&vol_scraper, &options)?;
    
    Ok(())
}
