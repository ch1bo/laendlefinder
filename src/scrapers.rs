use crate::common_scraper::PlatformScraper;
use crate::models::{Property, ListingType};
use crate::{scraper, laendleimmo_scraper};
use anyhow::Result;

pub struct VolScraper;

impl PlatformScraper for VolScraper {
    fn name(&self) -> &str {
        "Vol.at"
    }
    
    fn scrape_listings(&self, max_pages: usize) -> Result<Vec<String>> {
        scraper::scrape_all_index_pages(max_pages)
    }
    
    fn scrape_property(&self, url: &str, cookies: Option<&str>) -> Result<Property> {
        scraper::scrape_property_page(url, cookies, ListingType::Sold)
    }
}

pub struct LaendleimmoScraper;

impl PlatformScraper for LaendleimmoScraper {
    fn name(&self) -> &str {
        "Laendleimmo.at"
    }
    
    fn scrape_listings(&self, max_pages: usize) -> Result<Vec<String>> {
        laendleimmo_scraper::scrape_all_listing_pages(max_pages)
    }
    
    fn scrape_property(&self, url: &str, _cookies: Option<&str>) -> Result<Property> {
        laendleimmo_scraper::scrape_property_page(url)
    }
}