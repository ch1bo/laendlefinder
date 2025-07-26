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

pub fn run_scraper_with_options<T: PlatformScraper>(
    scraper: &T,
    options: &ScrapingOptions,
) -> Result<Vec<Property>> {
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
        return Ok(Vec::new());
    }
    
    // Scrape properties based on refresh mode
    let new_properties = if options.refresh {
        scrape_all_properties(scraper, property_urls, options.cookies.as_deref(), options.max_items)?
    } else {
        scrape_new_properties_only(scraper, &existing_properties, property_urls, options.cookies.as_deref(), options.max_items)?
    };
    
    let action = if options.refresh { "scraped/updated" } else { "scraped" };
    println!("{} {} new {} properties", action.char_indices().next().unwrap().1.to_uppercase().collect::<String>() + &action[1..], new_properties.len(), scraper.name().to_lowercase());
    
    Ok(new_properties)
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