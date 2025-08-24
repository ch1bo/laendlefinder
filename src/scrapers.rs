use crate::common_scraper::PlatformScraper;
use crate::models::{ListingType, Property};
use crate::tui::ScraperTUI;
use crate::{laendleimmo_scraper, scraper};
use anyhow::Result;

pub struct VolScraper;

impl PlatformScraper for VolScraper {
    fn base_url(&self) -> &str {
        "vol.at"
    }

    fn scrape_listings(
        &self,
        max_pages: usize,
        tui: Option<&mut ScraperTUI>,
        existing_urls: &std::collections::HashSet<String>,
    ) -> Result<Vec<String>> {
        scraper::scrape_all_index_pages(max_pages, tui, existing_urls)
    }

    fn scrape_property(&self, url: &str, cookies: Option<&str>) -> Result<Property> {
        check_url(self, url)?;
        scraper::scrape_property_page(url, cookies, ListingType::Sold)
    }
}

pub struct LaendleimmoScraper;

impl PlatformScraper for LaendleimmoScraper {
    fn base_url(&self) -> &str {
        "laendleimmo.at"
    }

    fn scrape_listings(
        &self,
        max_pages: usize,
        tui: Option<&mut ScraperTUI>,
        existing_urls: &std::collections::HashSet<String>,
    ) -> Result<Vec<String>> {
        laendleimmo_scraper::scrape_all_listing_pages(max_pages, tui, existing_urls)
    }

    fn scrape_property(&self, url: &str, _cookies: Option<&str>) -> Result<Property> {
        check_url(self, url)?;
        laendleimmo_scraper::scrape_property_page(url)
    }
}

fn check_url<S: PlatformScraper>(scraper: &S, url: &str) -> Result<()> {
    if !url.contains(scraper.base_url()) {
        return Err(anyhow::anyhow!(
            "URL does not match the base URL of the scraper: {}",
            scraper.base_url()
        ));
    }
    Ok(())
}
