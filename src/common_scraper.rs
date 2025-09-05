use crate::models::{Property, PropertyType, ListingType};
use crate::tui::ScraperTUI;
use crate::utils;
use crate::{debug, debug_println};
use anyhow::Result;
use chrono;
use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct ScrapingOptions {
    pub output_file: String,
    pub max_pages: Option<usize>,
    pub max_items: Option<usize>,
    pub refresh: bool,
    pub new: bool,
    pub cookies: Option<String>,
    pub debug: bool,
}

impl Default for ScrapingOptions {
    fn default() -> Self {
        Self {
            output_file: "properties.csv".to_string(),
            max_pages: None,
            max_items: None,
            refresh: false,
            new: true,
            cookies: None,
            debug: false,
        }
    }
}

pub trait PlatformScraper {
    fn base_url(&self) -> &str;
    fn scrape_listings(
        &self,
        max_pages: Option<usize>,
        tui: Option<&mut ScraperTUI>,
        existing_urls: &HashSet<String>,
    ) -> Result<Vec<String>>;
    fn scrape_new_urls(
        &self,
        tui: Option<&mut ScraperTUI>,
        existing_urls: &HashSet<String>,
    ) -> Result<Vec<String>>;
    fn scrape_property(&self, url: &str, cookies: Option<&str>) -> Result<Property>;
}

pub fn scrape_single_url<T: PlatformScraper>(
    scraper: &T,
    url: &str,
    options: &ScrapingOptions,
) -> Result<()> {
    // Set global debug flag
    debug::set_debug(options.debug);

    let mut tui = ScraperTUI::new();

    // 1. Load all existing properties
    let mut all_properties = utils::load_properties_from_csv(&options.output_file)?;
    tui.show_summary(all_properties.len())?;

    // 2. Find and temporarily store existing entry with the same URL if it exists
    let existing_property = all_properties.iter().find(|p| p.url == url).cloned();
    let existing_count = all_properties.len();
    all_properties.retain(|p| p.url != url);
    let removed_count = existing_count - all_properties.len();

    if removed_count > 0 {
        debug_println!("Removed {} existing entry for URL: {}", removed_count, url);
    }

    // 3. Scrape the specific URL
    tui.add_property(url.to_string())?;
    tui.start_scraping_property(url)?;

    let mut failed_urls = Vec::new();
    let mut final_properties = all_properties.clone();
    
    match scraper.scrape_property(url, options.cookies.as_deref()) {
        Ok(property) => {
            // Re-add existing property if it was found, so deduplication can merge properly
            if let Some(existing) = existing_property {
                final_properties.push(existing);
            }
            final_properties.push(property);
            
            // Use deduplication logic to properly handle unavailable transitions
            let deduplicated = deduplicate_properties_by_url(final_properties);
            
            tui.complete_property(url)?;
            debug_println!("Successfully scraped and updated: {}", url);
            
            // Save immediately after successful scrape
            utils::save_properties_to_csv(&deduplicated, &options.output_file)?;
            
            // Show final summary
            tui.show_final_summary(1, deduplicated.len())?;
        }
        Err(e) => {
            failed_urls.push((url.to_string(), e.to_string()));
            tui.fail_property(url)?;
            
            // Show final summary even on failure
            tui.show_final_summary(0, all_properties.len())?;
        }
    }

    // Show failure report if there were any failures
    tui.show_failure_report(&failed_urls)?;
    
    // Return error if scraping failed
    if !failed_urls.is_empty() {
        return Err(anyhow::anyhow!("Failed to scrape URL {}: {}", url, failed_urls[0].1));
    }

    Ok(())
}

pub fn run_scraper_with_options<T: PlatformScraper>(
    scraper: &T,
    options: &ScrapingOptions,
) -> Result<()> {
    // Set global debug flag
    debug::set_debug(options.debug);

    let mut tui = ScraperTUI::new();

    // 1. Load all existing properties
    let mut all_properties = utils::load_properties_from_csv(&options.output_file)?;
    tui.show_summary(all_properties.len())?;

    let relevant_urls: Vec<String> = all_properties
        .iter()
        .filter_map(|x| {
            x.url
                .contains(scraper.base_url())
                .then_some(x.url.to_string())
        })
        .collect();

    let urls_to_scrape = if options.refresh {
        // In refresh mode, use existing URLs instead of gathering new ones
        let existing_urls: Vec<String> = relevant_urls;

        if existing_urls.is_empty() {
            tui.update_listing_status(0, 0)?;
            return Ok(());
        }

        tui.update_listing_status_refresh(0, existing_urls.len())?;
        existing_urls
    } else if options.new {
        // New mode: gather new links until no new ones found in 5 consecutive pages
        // Create a set of existing URLs for fast lookup
        let existing_urls: HashSet<String> = relevant_urls.into_iter().collect();

        let found_urls = scraper.scrape_new_urls(Some(&mut tui), &existing_urls)?;

        if found_urls.is_empty() {
            tui.update_listing_status(0, 0)?;
            return Ok(());
        }

        // Update last_seen for existing properties that were found in listings
        let now = chrono::Utc::now().naive_utc().date();
        let mut updated_count = 0;
        
        for property in &mut all_properties {
            if found_urls.contains(&property.url) && existing_urls.contains(&property.url) {
                property.last_seen = Some(now);
                updated_count += 1;
            }
        }
        
        // Save updated properties if any last_seen dates were updated
        if updated_count > 0 {
            let deduplicated = deduplicate_properties_by_url(all_properties.clone());
            utils::save_properties_to_csv(&deduplicated, &options.output_file)?;
        }

        // Filter out existing URLs in normal mode
        let mut new_urls = Vec::new();
        let mut known_count = 0;

        for url in &found_urls {
            if existing_urls.contains(url) {
                known_count += 1;
            } else {
                new_urls.push(url.clone());
            }
        }

        tui.update_listing_status(new_urls.len(), known_count)?;

        if new_urls.is_empty() {
            return Ok(());
        }

        new_urls
    } else {
        // Legacy mode: gather new links from listings with max_pages limit
        // Create a set of existing URLs for fast lookup
        let existing_urls: HashSet<String> = relevant_urls.into_iter().collect();

        let found_urls = scraper.scrape_listings(options.max_pages, Some(&mut tui), &existing_urls)?;

        if found_urls.is_empty() {
            tui.update_listing_status(0, 0)?;
            return Ok(());
        }

        // Update last_seen for existing properties that were found in listings
        let now = chrono::Utc::now().naive_utc().date();
        let mut updated_count = 0;
        
        for property in &mut all_properties {
            if found_urls.contains(&property.url) && existing_urls.contains(&property.url) {
                property.last_seen = Some(now);
                updated_count += 1;
            }
        }
        
        // Save updated properties if any last_seen dates were updated
        if updated_count > 0 {
            let deduplicated = deduplicate_properties_by_url(all_properties.clone());
            utils::save_properties_to_csv(&deduplicated, &options.output_file)?;
        }

        // Filter out existing URLs in normal mode
        let mut new_urls = Vec::new();
        let mut known_count = 0;

        for url in &found_urls {
            if existing_urls.contains(url) {
                known_count += 1;
            } else {
                new_urls.push(url.clone());
            }
        }

        tui.update_listing_status(new_urls.len(), known_count)?;

        if new_urls.is_empty() {
            return Ok(());
        }

        new_urls
    };

    // Apply max_items limit if specified
    let urls_to_scrape = if let Some(max_items) = options.max_items {
        urls_to_scrape
            .into_iter()
            .take(max_items)
            .collect::<Vec<_>>()
    } else {
        urls_to_scrape
    };

    // Add all properties to TUI as pending
    for url in &urls_to_scrape {
        tui.add_property(url.clone())?;
    }

    // Show initial progress bar after all properties are added
    tui.show_initial_progress_bar()?;

    // Scrape the selected URLs
    let mut newly_scraped = Vec::new();
    let mut failed_urls = Vec::new();
    
    for url in urls_to_scrape.iter() {
        tui.start_scraping_property(url)?;

        match scraper.scrape_property(url, options.cookies.as_deref()) {
            Ok(property) => {
                newly_scraped.push(property.clone());
                tui.complete_property(url)?;
                
                // Save progress after each successful scrape
                let mut current_properties = all_properties.clone();
                current_properties.extend(newly_scraped.clone());
                
                // If in refresh mode, remove old versions of successfully scraped URLs
                if options.refresh {
                    let scraped_urls: HashSet<String> = newly_scraped.iter().map(|p| p.url.clone()).collect();
                    current_properties.retain(|p| !scraped_urls.contains(&p.url) || newly_scraped.iter().any(|np| np.url == p.url));
                }
                
                let deduplicated = deduplicate_properties_by_url(current_properties);
                utils::save_properties_to_csv(&deduplicated, &options.output_file)?;
            }
            Err(e) => {
                failed_urls.push((url.clone(), e.to_string()));
                tui.fail_property(url)?;
            }
        }

        // Add a small delay to be respectful to the server
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    // Final cleanup and summary (properties already saved after each scrape)
    let scraped_count = newly_scraped.len();
    
    // Calculate final totals for summary
    let mut final_properties = all_properties.clone();
    final_properties.extend(newly_scraped.clone());
    
    if options.refresh {
        let scraped_urls: HashSet<String> = urls_to_scrape.iter().cloned().collect();
        final_properties.retain(|p| !scraped_urls.contains(&p.url));
        final_properties.extend(newly_scraped);
    }
    
    let deduplicated_properties = deduplicate_properties_by_url(final_properties);

    // Show final summary
    tui.show_final_summary(scraped_count, deduplicated_properties.len())?;

    // Show failure report if there were any failures
    tui.show_failure_report(&failed_urls)?;

    Ok(())
}

/// Deduplicate properties by URL, merging first_seen/last_seen dates properly
pub fn deduplicate_properties_by_url(properties: Vec<Property>) -> Vec<Property> {
    use std::collections::HashMap;
    
    let mut property_map: HashMap<String, Property> = HashMap::new();
    
    for property in properties {
        match property_map.get(&property.url) {
            Some(existing) => {
                // Property already exists, merge the data intelligently
                let merged_property = if property.listing_type == ListingType::Unavailable && existing.listing_type != ListingType::Unavailable {
                    // Property became unavailable - preserve all existing data except status and dates
                    debug_println!("Property became unavailable, preserving existing data: {}", property.url);
                    Property {
                        url: property.url.clone(),
                        name: if existing.name != "Unknown Property" && existing.name != "Unavailable Property" { existing.name.clone() } else { property.name },
                        price: if existing.price != "Unknown" && existing.price != "Unavailable" { existing.price.clone() } else { property.price },
                        location: if existing.location != "Unknown" { existing.location.clone() } else { property.location },
                        property_type: if existing.property_type != PropertyType::Unknown { existing.property_type.clone() } else { property.property_type },
                        listing_type: property.listing_type, // Update to unavailable
                        date: existing.date.or(property.date), // Preserve original listing date
                        coordinates: existing.coordinates.or(property.coordinates),
                        address: existing.address.clone().or(property.address),
                        size_living: existing.size_living.clone().or(property.size_living),
                        size_ground: existing.size_ground.clone().or(property.size_ground),
                        // Keep the earliest first_seen date
                        first_seen: existing.first_seen.or(property.first_seen),
                        // Use the latest last_seen date
                        last_seen: property.last_seen.or(existing.last_seen),
                    }
                } else {
                    // Normal property update - use new data but preserve tracking dates
                    Property {
                        url: property.url.clone(),
                        name: property.name,
                        price: property.price,
                        location: property.location,
                        property_type: property.property_type,
                        listing_type: property.listing_type,
                        date: property.date,
                        coordinates: property.coordinates,
                        address: property.address,
                        size_living: property.size_living,
                        size_ground: property.size_ground,
                        // Keep the earliest first_seen date
                        first_seen: existing.first_seen.or(property.first_seen),
                        // Use the latest last_seen date
                        last_seen: property.last_seen.or(existing.last_seen),
                    }
                };
                property_map.insert(property.url.clone(), merged_property);
            }
            None => {
                // New property, just insert it
                property_map.insert(property.url.clone(), property);
            }
        }
    }
    
    // Convert back to Vec, maintaining insertion order is less important here
    // since we're dealing with different URLs
    property_map.into_values().collect()
}

// Legacy functions for backwards compatibility
pub struct ScrapingResult {
    pub scraped_properties: Vec<Property>,
    pub scraped_urls: Vec<String>,
    pub is_refresh: bool,
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
    }

    // Add the newly scraped properties
    existing_properties.extend(result.scraped_properties);

    // Deduplicate by URL (keep the last occurrence to preserve refreshed data)
    deduplicate_properties_by_url(existing_properties)
}

