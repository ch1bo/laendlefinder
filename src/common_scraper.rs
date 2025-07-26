use crate::models::Property;
use crate::utils;
use anyhow::Result;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ScrapingOptions {
    pub output_file: String,
    pub max_pages: usize,
    pub max_items: Option<usize>,
    pub refresh: bool,
    pub cookies: Option<String>,
}

impl Default for ScrapingOptions {
    fn default() -> Self {
        Self {
            output_file: "properties.csv".to_string(),
            max_pages: 1,
            max_items: None,
            refresh: false,
            cookies: None,
        }
    }
}

pub trait PlatformScraper {
    fn name(&self) -> &str;
    fn scrape_listings(&self, max_pages: usize) -> Result<Vec<String>>;
    fn scrape_property(&self, url: &str, cookies: Option<&str>) -> Result<Property>;
}

pub struct ScrapingResult {
    pub scraped_properties: Vec<Property>,
    pub scraped_urls: Vec<String>,
    pub is_refresh: bool,
}

pub fn run_scraper_with_options<T: PlatformScraper>(
    scraper: &T,
    options: &ScrapingOptions,
) -> Result<ScrapingResult> {
    println!("{} Property Scraper", scraper.name());
    println!("{}", "=".repeat(scraper.name().len() + 17));
    
    // Load existing properties first
    let existing_properties = utils::load_properties_from_csv(&options.output_file)?;
    println!("Loaded {} existing properties", existing_properties.len());
    
    // Get property URLs from listing pages up to max_pages
    let property_urls = scraper.scrape_listings(options.max_pages)?;
    println!("Found {} property URLs", property_urls.len());
    
    if property_urls.is_empty() {
        println!("No properties found.");
        return Ok(ScrapingResult {
            scraped_properties: Vec::new(),
            scraped_urls: Vec::new(),
            is_refresh: options.refresh,
        });
    }
    
    // Store the URLs we're attempting to scrape
    let scraped_urls = property_urls.clone();
    
    // Scrape properties based on refresh mode
    let scraped_properties = if options.refresh {
        scrape_all_properties(scraper, property_urls, options.cookies.as_deref(), options.max_items)?
    } else {
        scrape_new_properties_only(scraper, &existing_properties, property_urls, options.cookies.as_deref(), options.max_items)?
    };
    
    let action = if options.refresh { "scraped/updated" } else { "scraped" };
    println!("{} {} {} {} properties", action.char_indices().next().unwrap().1.to_uppercase().collect::<String>() + &action[1..], scraped_properties.len(), scraper.name().to_lowercase(), if options.refresh { "refreshed" } else { "new" });
    
    Ok(ScrapingResult {
        scraped_properties,
        scraped_urls,
        is_refresh: options.refresh,
    })
}

fn scrape_new_properties_only<T: PlatformScraper>(
    scraper: &T,
    existing_properties: &[Property],
    property_urls: Vec<String>,
    cookies: Option<&str>,
    max_items: Option<usize>,
) -> Result<Vec<Property>> {
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
            match scraper.scrape_property(&url, cookies) {
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

fn scrape_all_properties<T: PlatformScraper>(
    scraper: &T,
    property_urls: Vec<String>,
    cookies: Option<&str>,
    max_items: Option<usize>,
) -> Result<Vec<Property>> {
    let mut properties = Vec::new();
    
    for url in property_urls {
        // Check if we've reached the maximum number of items
        if let Some(max) = max_items {
            if properties.len() >= max {
                println!("Reached maximum number of items ({}), stopping", max);
                break;
            }
        }
        
        println!("Scraping property: {}", url);
        match scraper.scrape_property(&url, cookies) {
            Ok(property) => {
                properties.push(property);
            },
            Err(e) => {
                eprintln!("Error scraping property {}: {}", url, e);
            }
        }
        
        // Add a small delay to be respectful to the server
        std::thread::sleep(std::time::Duration::from_millis(500));
    }
    
    Ok(properties)
}

pub fn merge_properties_with_refresh(
    mut existing_properties: Vec<Property>,
    result: ScrapingResult,
    _platform_domain: &str,
) -> Vec<Property> {
    if result.is_refresh {
        // In refresh mode, only remove properties that were actually scraped
        // Keep all other properties (including other platforms and non-scraped URLs from this platform)
        existing_properties.retain(|p| !result.scraped_urls.contains(&p.url));
        existing_properties.extend(result.scraped_properties);
    } else {
        // In normal mode, just add new properties
        existing_properties.extend(result.scraped_properties);
    }
    existing_properties
}