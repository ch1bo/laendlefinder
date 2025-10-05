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
    pub refresh_days: Option<u32>,
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
            refresh_days: None,
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

    // 2. Find existing entry position for in-place update
    let existing_position = all_properties.iter().position(|p| p.url == url);
    
    if let Some(pos) = existing_position {
        debug_println!("Found existing entry at position {} for URL: {}", pos, url);
    }

    // 3. Scrape the specific URL
    tui.add_property(url.to_string())?;
    tui.start_scraping_property(url)?;

    let mut failed_urls = Vec::new();
    
    match scraper.scrape_property(url, options.cookies.as_deref()) {
        Ok(property) => {
            // Handle single property update in-place to preserve order
            let merged_property = if let Some(pos) = existing_position {
                let existing = &all_properties[pos];
                // Property already exists, merge the data intelligently
                if property.listing_type == ListingType::Unavailable && existing.listing_type != ListingType::Unavailable {
                    // Property became unavailable - preserve all existing data except status and dates
                    debug_println!("Property became unavailable, preserving existing data: {}", property.url);
                    Property {
                        url: property.url.clone(),
                        name: if existing.name != "Unknown Property" && existing.name != "Unavailable Property" { existing.name.clone() } else { property.name },
                        price: existing.price.clone(), // Always preserve existing price when becoming unavailable
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
                        // Preserve existing last_seen since property became unavailable
                        last_seen: existing.last_seen.or(property.last_seen),
                    }
                } else {
                    // Normal property update - use new data but preserve existing data when scraper fails
                    Property {
                        url: property.url.clone(),
                        name: if property.name.is_empty() || property.name == "Unknown Property" || property.name == "Unavailable Property" { existing.name.clone() } else { property.name },
                        price: if property.price.is_empty() || property.price == "Unknown" || property.price == "Unavailable" { existing.price.clone() } else { property.price },
                        location: if property.location.is_empty() || property.location == "Unknown" { existing.location.clone() } else { property.location },
                        property_type: if property.property_type == PropertyType::Unknown { existing.property_type.clone() } else { property.property_type },
                        listing_type: property.listing_type, // Always update listing status
                        date: property.date.or(existing.date),
                        coordinates: property.coordinates.or(existing.coordinates),
                        address: property.address.or(existing.address.clone()),
                        size_living: property.size_living.or(existing.size_living.clone()),
                        size_ground: property.size_ground.or(existing.size_ground.clone()),
                        // Keep the earliest first_seen date
                        first_seen: existing.first_seen.or(property.first_seen),
                        // Use the latest last_seen date
                        last_seen: property.last_seen.or(existing.last_seen),
                    }
                }
            } else {
                // New property, just use it as-is
                property
            };
            
            // Update in-place or add at end for new properties
            if let Some(pos) = existing_position {
                all_properties[pos] = merged_property;
            } else {
                all_properties.push(merged_property);
            }
            
            tui.complete_property(url)?;
            debug_println!("Successfully scraped and updated: {}", url);
            
            // Save immediately after successful scrape
            utils::save_properties_to_csv(&all_properties, &options.output_file)?;
            
            // Show final summary
            tui.show_final_summary(1, all_properties.len())?;
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

    let urls_to_scrape = if let Some(refresh_days) = options.refresh_days {
        // In refresh mode, filter and prioritize properties older than N days
        let refresh_days = refresh_days.max(1); // Default to 1 day minimum
        let cutoff_date = chrono::Utc::now().naive_utc().date() - chrono::Duration::days(refresh_days as i64);
        
        let mut relevant_properties: Vec<&Property> = all_properties
            .iter()
            .filter(|x| {
                // Filter by platform URL
                if !x.url.contains(scraper.base_url()) {
                    return false;
                }
                // Only refresh available properties - no point in refreshing unavailable or sold properties
                if x.listing_type != ListingType::Available {
                    return false;
                }
                // Filter by age - include properties without last_seen or with old last_seen
                match x.last_seen {
                    None => true, // Properties without last_seen should be refreshed
                    Some(last_seen) => last_seen <= cutoff_date, // Properties older than cutoff
                }
            })
            .collect();
            
        if relevant_properties.is_empty() {
            debug_println!("Refresh mode: no properties older than {} days found", refresh_days);
            tui.update_listing_status(0, 0)?;
            return Ok(());
        }
        
        // Sort by main property date (oldest first), then by first_seen for properties without date
        relevant_properties.sort_by(|a, b| {
            match (a.date, b.date) {
                (Some(a_date), Some(b_date)) => a_date.cmp(&b_date), // oldest first
                (None, Some(_)) => std::cmp::Ordering::Less, // properties without date come first
                (Some(_), None) => std::cmp::Ordering::Greater,
                (None, None) => a.first_seen.cmp(&b.first_seen), // fallback to first_seen
            }
        });
        
        let prioritized_urls: Vec<String> = relevant_properties
            .into_iter()
            .map(|p| p.url.clone())
            .collect();
            
        debug_println!("Refresh mode: found {} properties older than {} days (cutoff: {})", 
                      prioritized_urls.len(), refresh_days, cutoff_date);
        tui.update_listing_status_refresh(0, prioritized_urls.len())?;
        prioritized_urls
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
                
                // Use deduplication logic to properly handle unavailable transitions
                let deduplicated = deduplicate_properties_by_url(current_properties);
                utils::save_properties_to_csv(&deduplicated, &options.output_file)?;
            }
            Err(e) => {
                failed_urls.push((url.clone(), e.to_string()));
                tui.fail_property(url)?;
            }
        }

        // Add a delay to be respectful to the server and avoid rate limiting
        std::thread::sleep(std::time::Duration::from_millis(2000));
    }

    // Final cleanup and summary (properties already saved after each scrape)
    let scraped_count = newly_scraped.len();
    
    // Calculate final totals for summary
    let mut final_properties = all_properties.clone();
    final_properties.extend(newly_scraped.clone());
    
    let deduplicated_properties = deduplicate_properties_by_url(final_properties);

    // Show final summary
    tui.show_final_summary(scraped_count, deduplicated_properties.len())?;

    // Show failure report if there were any failures
    tui.show_failure_report(&failed_urls)?;

    Ok(())
}

/// Deduplicate properties by URL, merging first_seen/last_seen dates properly
/// PRESERVES ORDER: Updates existing properties in-place, appends new ones at the end
pub fn deduplicate_properties_by_url(properties: Vec<Property>) -> Vec<Property> {
    let mut result = Vec::new();
    let mut seen_urls = std::collections::HashSet::new();
    
    // First pass: collect all unique properties in original order
    for property in properties {
        if !seen_urls.contains(&property.url) {
            seen_urls.insert(property.url.clone());
            result.push(property);
        } else {
            // Find existing property and merge
            if let Some(existing_pos) = result.iter().position(|p| p.url == property.url) {
                let existing = &result[existing_pos];
                let merged_property = if property.listing_type == ListingType::Unavailable && existing.listing_type != ListingType::Unavailable {
                    // Property became unavailable - preserve all existing data except status and dates
                    debug_println!("Property became unavailable, preserving existing data: {}", property.url);
                    Property {
                        url: property.url.clone(),
                        name: if existing.name != "Unknown Property" && existing.name != "Unavailable Property" { existing.name.clone() } else { property.name },
                        price: existing.price.clone(), // Always preserve existing price when becoming unavailable
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
                        // Preserve existing last_seen since property became unavailable
                        last_seen: existing.last_seen.or(property.last_seen),
                    }
                } else {
                    // Normal property update - use new data but preserve existing data when scraper fails
                    Property {
                        url: property.url.clone(),
                        name: if property.name.is_empty() || property.name == "Unknown Property" || property.name == "Unavailable Property" { existing.name.clone() } else { property.name },
                        price: if property.price.is_empty() || property.price == "Unknown" || property.price == "Unavailable" { existing.price.clone() } else { property.price },
                        location: if property.location.is_empty() || property.location == "Unknown" { existing.location.clone() } else { property.location },
                        property_type: if property.property_type == PropertyType::Unknown { existing.property_type.clone() } else { property.property_type },
                        listing_type: property.listing_type, // Always update listing status
                        date: property.date.or(existing.date),
                        coordinates: property.coordinates.or(existing.coordinates),
                        address: property.address.or(existing.address.clone()),
                        size_living: property.size_living.or(existing.size_living.clone()),
                        size_ground: property.size_ground.or(existing.size_ground.clone()),
                        // Keep the earliest first_seen date
                        first_seen: existing.first_seen.or(property.first_seen),
                        // Use the latest last_seen date
                        last_seen: property.last_seen.or(existing.last_seen),
                    }
                };
                // Update in-place to preserve order
                result[existing_pos] = merged_property;
            }
        }
    }
    
    result
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

