use crate::models::Property;
use crate::tui::ScraperTUI;
use crate::utils;
use crate::{debug, debug_println};
use anyhow::Result;
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

    // 2. Remove existing entry with the same URL if it exists
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
    
    match scraper.scrape_property(url, options.cookies.as_deref()) {
        Ok(property) => {
            all_properties.push(property);
            tui.complete_property(url)?;
            debug_println!("Successfully scraped and updated: {}", url);
            
            // Save immediately after successful scrape
            utils::save_properties_to_csv(&all_properties, &options.output_file)?;
        }
        Err(e) => {
            failed_urls.push((url.to_string(), e.to_string()));
            tui.fail_property(url)?;
            // Continue to show summary instead of returning early
        }
    }

    // Show final summary
    tui.show_final_summary(1, all_properties.len())?;

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
    let all_properties = utils::load_properties_from_csv(&options.output_file)?;
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

/// Deduplicate properties by URL, keeping the last occurrence (most recent data)
pub fn deduplicate_properties_by_url(properties: Vec<Property>) -> Vec<Property> {
    let mut seen_urls = HashSet::new();
    let mut deduplicated = Vec::new();

    // Process in reverse order so we keep the last (most recent) occurrence of each URL
    for property in properties.into_iter().rev() {
        if seen_urls.insert(property.url.clone()) {
            deduplicated.push(property);
        }
    }

    // Reverse back to maintain original order
    deduplicated.reverse();
    deduplicated
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

