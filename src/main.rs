use anyhow::Result;
use clap::Parser;
use laendlefinder::common_scraper::{ScrapingOptions, run_scraper_with_options};
use laendlefinder::scrapers::{VolScraper, LaendleimmoScraper};

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
        println!("\n--- Vol.at Scraper ---");
        let vol_scraper = VolScraper;
        run_scraper_with_options(&vol_scraper, &options)?;
    } else {
        println!("Skipping vol.at scraper");
    }
    
    // Run laendleimmo.at scraper (available properties) 
    if !args.skip_laendleimmo {
        println!("\n--- Laendleimmo.at Scraper ---");
        let laendleimmo_scraper = LaendleimmoScraper;
        run_scraper_with_options(&laendleimmo_scraper, &options)?;
    } else {
        println!("Skipping laendleimmo.at scraper");
    }
    
    println!("\n=== All scraping completed ===");
    println!("Results saved to: {}", args.output);
    
    Ok(())
}
