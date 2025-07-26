use crate::models::{Property, ListingType};
use anyhow::{Result, Context};
use scraper::{Html, Selector};
use regex::Regex;

const BASE_URL: &str = "https://www.laendleimmo.at/kaufobjekt";

pub fn scrape_all_listing_pages(max_pages: usize) -> Result<Vec<String>> {
    let mut all_property_urls = Vec::new();
    
    for page in 1..=max_pages {
        let page_url = if page == 1 {
            BASE_URL.to_string()
        } else {
            format!("{}?page={}", BASE_URL, page)
        };
        
        println!("Scraping listing page: {}", page_url);
        
        match scrape_listing_page(&page_url) {
            Ok(urls) => {
                if urls.is_empty() {
                    println!("No more properties found on page {}, stopping", page);
                    break;
                }
                all_property_urls.extend(urls);
            },
            Err(e) => {
                eprintln!("Error scraping page {}: {}", page, e);
                break;
            }
        }
    }
    
    Ok(all_property_urls)
}

pub fn scrape_listing_page(url: &str) -> Result<Vec<String>> {
    println!("Fetching listing page: {}", url);
    
    let response = reqwest::blocking::Client::new()
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()
        .context("Failed to fetch listing page")?;
    
    let body = response.text().context("Failed to read response body")?;
    let document = Html::parse_document(&body);
    
    // Look for property links in the listing page
    // Based on the URL structure: /immobilien/{type}/{subtype}/vorarlberg/{district}/{id}
    let link_selector = Selector::parse("a[href*='/immobilien/']")
        .map_err(|e| anyhow::anyhow!("Failed to parse link selector: {:?}", e))?;
    
    let mut property_urls = Vec::new();
    
    for element in document.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            if href.contains("/immobilien/") && href.contains("/vorarlberg/") {
                let full_url = if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://www.laendleimmo.at{}", href)
                };
                
                // Avoid duplicates
                if !property_urls.contains(&full_url) {
                    property_urls.push(full_url);
                }
            }
        }
    }
    
    println!("Found {} property URLs on page", property_urls.len());
    Ok(property_urls)
}

pub fn scrape_property_page(url: &str) -> Result<Property> {
    println!("Scraping property page: {}", url);
    
    let response = reqwest::blocking::Client::new()
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()
        .context("Failed to fetch property page")?;
    
    let body = response.text().context("Failed to read response body")?;
    let document = Html::parse_document(&body);
    
    // Extract basic information
    let _title = extract_title(&document)?;
    let price = extract_price(&document)?;
    let location = extract_location(&document, url)?;
    let property_type = extract_property_type(&document, url)?;
    let address = extract_address(&document);
    let size_living = extract_living_size(&document);
    
    // Try to extract coordinates if available
    let coordinates = extract_coordinates(&document);
    
    println!("Extracted data: price={}, location={}, type={}", 
             price, location, property_type);
    
    Ok(Property {
        url: url.to_string(),
        price,
        location,
        property_type,
        listing_type: ListingType::Available,
        date: None, // Available properties don't have sale dates
        coordinates,
        address,
        size_living,
    })
}

fn extract_title(document: &Html) -> Result<String> {
    let title_selector = Selector::parse("h1, .property-title, .title")
        .map_err(|e| anyhow::anyhow!("Failed to parse title selector: {:?}", e))?;
    
    if let Some(element) = document.select(&title_selector).next() {
        Ok(element.text().collect::<Vec<_>>().join(" ").trim().to_string())
    } else {
        Ok("Unknown Property".to_string())
    }
}

fn extract_price(document: &Html) -> Result<String> {
    // Look for various price selectors
    let price_selectors = [
        ".price",
        ".property-price", 
        "[class*='price']",
        ".preis",
        ".kaufpreis"
    ];
    
    for selector_str in &price_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join(" ");
                if text.contains("€") || text.contains("EUR") {
                    // Clean up the price text
                    let price_regex = Regex::new(r"[\d,.]+").unwrap();
                    if let Some(price_match) = price_regex.find(&text) {
                        return Ok(price_match.as_str().replace(".", "").replace(",", ""));
                    }
                }
            }
        }
    }
    
    // Fallback: search in all text for price patterns
    let text = document.root_element().text().collect::<Vec<_>>().join(" ");
    let price_regex = Regex::new(r"(\d{1,3}(?:[.,]\d{3})*)\s*€").unwrap();
    if let Some(captures) = price_regex.captures(&text) {
        if let Some(price) = captures.get(1) {
            return Ok(price.as_str().replace(".", "").replace(",", ""));
        }
    }
    
    Ok("Unknown".to_string())
}

fn extract_location(document: &Html, url: &str) -> Result<String> {
    // Try to extract from URL first
    let url_regex = Regex::new(r"/vorarlberg/([^/]+)/").unwrap();
    if let Some(captures) = url_regex.captures(url) {
        if let Some(location) = captures.get(1) {
            return Ok(location.as_str().to_string());
        }
    }
    
    // Look for location in document
    let location_selectors = [
        ".location",
        ".property-location",
        "[class*='location']",
        ".ort",
        ".gemeinde"
    ];
    
    for selector_str in &location_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if !text.is_empty() && text != "Vorarlberg" {
                    return Ok(text);
                }
            }
        }
    }
    
    Ok("Unknown".to_string())
}

fn extract_property_type(document: &Html, url: &str) -> Result<String> {
    // Try to extract from URL first
    let url_regex = Regex::new(r"/immobilien/([^/]+)/").unwrap();
    if let Some(captures) = url_regex.captures(url) {
        if let Some(prop_type) = captures.get(1) {
            return Ok(prop_type.as_str().to_string());
        }
    }
    
    // Look for property type in document
    let type_selectors = [
        ".property-type",
        ".objektart",
        "[class*='type']"
    ];
    
    for selector_str in &type_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if !text.is_empty() {
                    return Ok(text);
                }
            }
        }
    }
    
    Ok("Unknown".to_string())
}

fn extract_address(document: &Html) -> Option<String> {
    let address_selectors = [
        ".address",
        ".property-address",
        "[class*='address']",
        ".adresse"
    ];
    
    for selector_str in &address_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = element.text().collect::<Vec<_>>().join(" ").trim().to_string();
                if !text.is_empty() {
                    return Some(text);
                }
            }
        }
    }
    
    None
}

fn extract_living_size(document: &Html) -> Option<String> {
    let size_selectors = [
        ".living-area",
        ".wohnflaeche",
        "[class*='area']",
        ".groesse"
    ];
    
    for selector_str in &size_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join(" ");
                if text.contains("m²") || text.contains("qm") {
                    let size_regex = Regex::new(r"(\d+(?:[.,]\d+)?)\s*(?:m²|qm)").unwrap();
                    if let Some(captures) = size_regex.captures(&text) {
                        if let Some(size) = captures.get(1) {
                            return Some(size.as_str().replace(",", "."));
                        }
                    }
                }
            }
        }
    }
    
    None
}


fn extract_coordinates(document: &Html) -> Option<(f64, f64)> {
    // Look for coordinates in script tags or data attributes
    let script_selector = Selector::parse("script").ok()?;
    
    for script in document.select(&script_selector) {
        let script_text = script.text().collect::<String>();
        
        // Look for common coordinate patterns
        let coord_patterns = [
            r"lat[^:]*:\s*([0-9.]+)[^,]*,\s*lng[^:]*:\s*([0-9.]+)",
            r"latitude[^:]*:\s*([0-9.]+)[^,]*,\s*longitude[^:]*:\s*([0-9.]+)",
            r"([0-9.]+),\s*([0-9.]+)" // Simple lat,lng pattern
        ];
        
        for pattern in &coord_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(captures) = regex.captures(&script_text) {
                    if let (Ok(lat), Ok(lng)) = (
                        captures.get(1)?.as_str().parse::<f64>(),
                        captures.get(2)?.as_str().parse::<f64>()
                    ) {
                        // Validate coordinates are in reasonable range for Austria
                        if lat >= 46.0 && lat <= 49.0 && lng >= 9.0 && lng <= 17.0 {
                            return Some((lat, lng));
                        }
                    }
                }
            }
        }
    }
    
    None
}