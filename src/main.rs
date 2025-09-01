use anyhow::Result;
use clap::Parser;
use laendlefinder::common_scraper::{ScrapingOptions, run_scraper_with_options, scrape_single_url};
use laendlefinder::scrapers::{VolScraper, LaendleimmoScraper};
use laendlefinder::debug;

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
    
    /// Scrape new URLs until no new ones found in 5 consecutive pages (default mode)
    #[clap(short, long, default_value = "true")]
    new: bool,
    
    /// Skip vol.at scraper
    #[clap(long)]
    skip_vol: bool,
    
    /// Skip laendleimmo.at scraper
    #[clap(long)]
    skip_laendleimmo: bool,
    
    /// Enable debug output
    #[clap(short, long)]
    debug: bool,
    
    /// Scrape a specific URL and update only that entry in the database
    #[clap(short = 'u', long)]
    url: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    
    // Set debug flag early
    debug::set_debug(args.debug);
    
    if !args.debug {
        println!("Laendlefinder - Property Scraper for Vorarlberg");
        println!("===============================================");
    }
    
    // Create scraping options
    let options = ScrapingOptions {
        output_file: args.output.clone(),
        max_pages: args.max_pages,
        max_items: args.max_items,
        refresh: args.refresh,
        new: args.new,
        cookies: args.cookies.clone(),
        debug: args.debug,
    };
    
    // If a specific URL is provided, scrape only that URL
    if let Some(url) = args.url {
        if !args.debug {
            println!("Scraping specific URL: {}", url);
        }
        
        // Determine which scraper to use based on the URL domain
        if url.contains("vol.at") {
            let vol_scraper = VolScraper;
            scrape_single_url(&vol_scraper, &url, &options)?;
        } else if url.contains("laendleimmo.at") {
            let laendleimmo_scraper = LaendleimmoScraper;
            scrape_single_url(&laendleimmo_scraper, &url, &options)?;
        } else {
            return Err(anyhow::anyhow!("Unsupported URL domain. Only vol.at and laendleimmo.at are supported."));
        }
        
        if !args.debug {
            println!("URL scraping completed. Results saved to: {}", args.output);
        }
        return Ok(());
    }
    
    // Run vol.at scraper (sold properties)
    if !args.skip_vol {
        if !args.debug {
            println!("\n--- Vol.at Scraper ---");
        }
        let vol_scraper = VolScraper;
        run_scraper_with_options(&vol_scraper, &options)?;
    } else if !args.debug {
        println!("Skipping vol.at scraper");
    }
    
    // Run laendleimmo.at scraper (available properties) 
    if !args.skip_laendleimmo {
        if !args.debug {
            println!("\n--- Laendleimmo.at Scraper ---");
        }
        let laendleimmo_scraper = LaendleimmoScraper;
        run_scraper_with_options(&laendleimmo_scraper, &options)?;
    } else if !args.debug {
        println!("Skipping laendleimmo.at scraper");
    }
    
    if !args.debug {
        println!("\n=== All scraping completed ===");
        println!("Results saved to: {}", args.output);
    }
    
    Ok(())
}
