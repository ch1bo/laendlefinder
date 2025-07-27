use crate::models::{ListingType, Property, PropertyType};
use crate::parser;
use crate::tui::ScraperTUI;
use crate::{debug_eprintln, debug_println};
use anyhow::{Context, Result};
use chrono::NaiveDate;
use scraper::{Html, Selector};
use serde_json::Value;

const INDEX_URL: &str = "https://www.vol.at/themen/grund-und-boden";

pub fn scrape_all_index_pages(max_pages: usize, mut tui: Option<&mut ScraperTUI>) -> Result<Vec<String>> {
    let mut all_property_urls = Vec::new();
    let base_url = "https://www.vol.at/themen/grund-und-boden";

    if let Some(tui) = tui.as_mut() {
        tui.start_gathering(max_pages)?;
    }

    debug_println!("Scraping index page: {}", base_url);

    // Scrape the first page
    let property_urls = scrape_index_page()?;
    all_property_urls.extend(property_urls);

    if let Some(tui) = tui.as_mut() {
        tui.update_gathering_progress(1, max_pages, all_property_urls.len())?;
    }

    // If max_pages is 1, we're done
    if max_pages <= 1 {
        if let Some(tui) = tui.as_mut() {
            tui.finish_gathering(all_property_urls.len())?;
        }
        return Ok(all_property_urls);
    }

    // Otherwise, scrape additional pages up to max_pages
    for page in 2..=max_pages {
        let page_url = format!("{}?page={}", base_url, page);
        debug_println!("Scraping index page: {}", page_url);

        match scrape_index_page_with_url(&page_url) {
            Ok(urls) => {
                if urls.is_empty() {
                    debug_println!("No more properties found on page {}, stopping", page);
                    break;
                }
                all_property_urls.extend(urls);
                
                if let Some(tui) = tui.as_mut() {
                    tui.update_gathering_progress(page, max_pages, all_property_urls.len())?;
                }
            }
            Err(e) => {
                debug_eprintln!("Error scraping page {}: {}", page, e);
                break;
            }
        }
    }

    if let Some(tui) = tui.as_mut() {
        tui.finish_gathering(all_property_urls.len())?;
    }

    Ok(all_property_urls)
}

pub fn scrape_index_page() -> Result<Vec<String>> {
    scrape_index_page_with_url(INDEX_URL)
}

fn scrape_index_page_with_url(url: &str) -> Result<Vec<String>> {
    debug_println!("Scraping index page: {}", url);

    // Fetch the index page
    let response = reqwest::blocking::get(url).context("Failed to fetch index page")?;
    let html = response.text().context("Failed to get response text")?;

    // Parse the HTML
    let document = Html::parse_document(&html);

    // Find the script tag containing the JSON data
    let script_selector = Selector::parse("#topicDataNode").unwrap();
    let script = document
        .select(&script_selector)
        .next()
        .context("Topic data script not found")?;

    // Parse the JSON content
    let json_str = script.inner_html();
    let json: Value = serde_json::from_str(&json_str).context("Failed to parse JSON data")?;

    // Extract all links from hits array
    let mut links = Vec::new();
    if let Some(hits) = json["prefetchedRawData"]["hits"].as_array() {
        for hit in hits {
            if let Some(link) = hit["link"].as_str() {
                links.push(link.replace(r"\/", "/").to_string());
            }
        }
    }

    debug_println!("Found {} property links on page", links.len());

    Ok(links)
}

pub fn scrape_property_page(
    url: &str,
    cookies: Option<&str>,
    listing_type: ListingType,
) -> Result<Property> {
    debug_println!("Scraping property page: {}", url);

    // Build request with optional cookies
    let mut request = reqwest::blocking::Client::new()
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36");

    if let Some(cookie_str) = cookies {
        debug_println!("Using cookies: {}", cookie_str);
        // Try to add cookies, but continue even if it fails
        match reqwest::header::HeaderValue::from_str(cookie_str) {
            Ok(header_value) => {
                request = request.header("Cookie", header_value);
            }
            Err(e) => {
                debug_println!(
                    "Warning: Could not use cookies due to invalid format: {}",
                    e
                );
                debug_println!("Continuing without cookies");
            }
        }
    } else {
        debug_println!("No cookies provided");
    }

    // Fetch the property page
    let response = match request.send() {
        Ok(resp) => {
            debug_println!("Response status: {}", resp.status());

            if !resp.status().is_success() {
                return Err(anyhow::anyhow!("HTTP error status: {}", resp.status()));
            }
            resp
        }
        Err(e) => {
            debug_eprintln!("Network error for {}: {:?}", url, e);
            return Err(anyhow::anyhow!("Failed to fetch property page: {}", e));
        }
    };

    let html = match response.text() {
        Ok(text) => {
            debug_println!("Received HTML content of length: {} bytes", text.len());
            if text.len() < 100 {
                debug_println!("Suspiciously short HTML content: {}", text);
            }
            text
        }
        Err(e) => {
            debug_eprintln!("Failed to get response text for {}: {:?}", url, e);
            return Err(anyhow::anyhow!("Failed to get response text: {}", e));
        }
    };

    // Parse the HTML
    let document = Html::parse_document(&html);

    // Try to extract data from embedded JavaScript
    let script_selector = Selector::parse("#externalPostDataNode").unwrap();
    if let Some(script) = document.select(&script_selector).next() {
        debug_println!("Found externalPostDataNode script tag");
        let json_str = script.inner_html();

        // Parse the JSON content
        let json: Value = serde_json::from_str(&json_str)
            .context("Failed to parse JSON data from externalPostDataNode")?;

        // Extract property data from the JSON
        return extract_property_from_json(json, url, &listing_type);
    }

    // Fallback to traditional HTML parsing if JavaScript data not found
    debug_println!("JavaScript data not found, falling back to HTML parsing");

    // Try different headline selectors
    let headline_selectors = [
        "h1.article-headline",
        "h1",
        ".article-headline",
        ".headline",
        "header h1",
        "article h1",
    ];

    let mut headline = String::new();
    for selector_str in headline_selectors {
        debug_println!("Trying headline selector: {}", selector_str);
        if let Ok(selector) = Selector::parse(selector_str) {
            let headlines: Vec<String> = document
                .select(&selector)
                .map(|el| {
                    let text = el.text().collect::<String>();
                    debug_println!("Found with '{}': '{}'", selector_str, text);
                    text
                })
                .collect();

            if let Some(first_headline) = headlines.first() {
                headline = first_headline.to_string();
                debug_println!("Selected headline: '{}'", headline);
                break;
            }
        }
    }

    if headline.is_empty() {
        return Err(anyhow::anyhow!("Headline not found with any selector"));
    }

    // Parse the headline using regex patterns
    let price = parser::extract_price(&headline)?;
    let location = parser::extract_location(&headline)?;

    // Extract property type using classification
    let property_type = PropertyType::from_string(&headline);

    debug_println!(
        "Extracted data: price={}, location={}, type={}",
        price,
        location,
        property_type
    );

    // Create and return the Property
    Ok(Property {
        url: url.to_string(),
        price: price.to_string(),
        location,
        property_type,
        listing_type: listing_type.clone(),
        date: None,
        coordinates: None,
        address: None,
        size_living: None,
        size_ground: None,
    })
}

fn extract_property_from_json(
    json: Value,
    url: &str,
    listing_type: &ListingType,
) -> Result<Property> {
    debug_println!("Extracting property data from JSON");

    // Navigate to the content section where property details are stored
    let post = &json["content"]["data"]["post"];

    // Extract the title which contains price and location information
    let title = post["title"]
        .as_str()
        .context("Title not found in JSON data")?;
    debug_println!("Title from JSON: {}", title);

    let location = parser::extract_location(title)?;

    // Extract property type using classification
    let property_type = PropertyType::from_string(title);

    // Try to extract the transaction date and coordinates from the structured data
    let mut price = None;
    let mut date = None;
    let mut coordinates = None;
    let mut address = None;
    let mut size_living = None;
    let mut size_ground = None;

    // Look for the GrundUndBoden block which contains structured data
    if let Some(blocks) = post["blocks"].as_array() {
        for block in blocks {
            if block["ot"] == "russmedia/grund-und-boden" {
                if let Some(data_str) = block["a"]
                    .as_array()
                    .and_then(|attrs| attrs.iter().find(|attr| attr["key"] == "data"))
                    .and_then(|attr| attr["value"].as_str())
                {
                    // Parse the data attribute which is a JSON string
                    if let Ok(data_json) = serde_json::from_str::<Value>(data_str) {
                        debug_println!("Found grund-und-boden data: {}", data_str);

                        // Extract price
                        match &data_json["price"] {
                            Value::Number(p) => {
                                price = Some(p.to_string());
                                debug_println!("Found price: {}", p);
                            }
                            _ => {
                                debug_println!("Price not found in JSON data");
                            }
                        }

                        // Extract transaction date
                        if let Some(date_str) = data_json["transactionDate"].as_str() {
                            debug_println!("Found transaction date: {}", date_str);
                            // Parse the date in format YYYY-MM-DD
                            if let Ok(parsed_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d")
                            {
                                date = Some(parsed_date);
                            }
                        }

                        // Extract coordinates
                        if let Some(coords) = data_json["coords"].as_object() {
                            if let (Some(lat), Some(lng)) =
                                (coords["lat"].as_f64(), coords["lng"].as_f64())
                            {
                                coordinates = Some((lat, lng));
                                debug_println!("Found coordinates: lat={}, lng={}", lat, lng);
                            }
                        }

                        // Extract address
                        if let Some(addr) = data_json["address"].as_str() {
                            address = Some(addr.to_string());
                            debug_println!("Found address: {}", addr);
                        }

                        // Extract living size
                        if let Some(size) = data_json["sizeLiving"].as_str() {
                            size_living = Some(size.to_string());
                            debug_println!("Found living size: {}", size);
                        }

                        // Extract ground size
                        if let Some(size) = data_json["sizeGround"].as_str() {
                            size_ground = Some(size.to_string());
                            debug_println!("Found ground size: {}", size);
                        }
                    }
                }
            }
        }
    }

    debug_println!(
        "Extracted data from JSON: price={:?}, location={}, type={}, date={:?}",
        price,
        location,
        property_type,
        date
    );

    // Create and return the Property
    Ok(Property {
        url: url.to_string(),
        price: price.unwrap_or("Unknown".to_string()),
        location,
        property_type,
        listing_type: listing_type.clone(),
        date,
        coordinates,
        address,
        size_living,
        size_ground,
    })
}
