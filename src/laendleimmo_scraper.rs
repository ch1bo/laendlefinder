use crate::models::{ListingType, Property};
use anyhow::{Context, Result};
use chrono::NaiveDate;
use regex::Regex;
use scraper::{Html, Selector};

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
            }
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

    // Try to extract from JSON-LD first (most reliable)
    if let Ok(mut json_data) = extract_from_json_ld(&body) {
        println!("Successfully extracted from JSON-LD");
        json_data.url = url.to_string(); // Set the URL
        return Ok(json_data);
    }

    // Fallback to HTML parsing
    println!("JSON-LD extraction failed, falling back to HTML parsing");
    let title = extract_title(&document)?;
    let price = extract_price(&document)?;
    let location = extract_location(&document, url)?;
    let property_type = extract_property_type(&document, url)?;
    let address = extract_address_from_location(&document);
    let size_living = extract_living_size(&document);
    let coordinates = extract_coordinates_from_map(&body);
    let date = extract_date_from_html(&body);

    println!(
        "Extracted data: price={}, location={}, type={}, title={}, date={:?}",
        price, location, property_type, title, date
    );

    Ok(Property {
        url: url.to_string(),
        price,
        location,
        property_type,
        listing_type: ListingType::Available,
        date,
        coordinates,
        address,
        size_living,
    })
}

fn extract_title(document: &Html) -> Result<String> {
    let title_selector = Selector::parse("h1, .property-title, .title")
        .map_err(|e| anyhow::anyhow!("Failed to parse title selector: {:?}", e))?;

    if let Some(element) = document.select(&title_selector).next() {
        return Ok(element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string());
    }
    Err(anyhow::anyhow!("Title not found"))
}

fn extract_price(document: &Html) -> Result<String> {
    // Look for various price selectors
    let price_selectors = [
        ".price",
        ".property-price",
        "[class*='price']",
        ".preis",
        ".kaufpreis",
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

    Err(anyhow::anyhow!("Price not found"))
}

fn extract_location(document: &Html, url: &str) -> Result<String> {
    if let Ok(loc) = extract_location_from_breadcrumbs(document) {
        return Ok(loc);
    }

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
        ".gemeinde",
    ];

    for selector_str in &location_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
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
    let type_selectors = [".property-type", ".objektart", "[class*='type']"];

    for selector_str in &type_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            if let Some(element) = document.select(&selector).next() {
                let text = element
                    .text()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .trim()
                    .to_string();
                if !text.is_empty() {
                    return Ok(text);
                }
            }
        }
    }

    Ok("Unknown".to_string())
}

fn extract_living_size(document: &Html) -> Option<String> {
    let size_selectors = [
        ".living-area",
        ".wohnflaeche",
        "[class*='area']",
        ".groesse",
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

fn extract_from_json_ld(body: &str) -> Result<Property> {
    // Look for JSON-LD script tag
    let json_start = body
        .find(r#"<script type="application/ld+json">"#)
        .context("JSON-LD script tag not found")?;
    let json_content_start = body[json_start..]
        .find('>')
        .context("JSON-LD script tag start not found")?
        + json_start
        + 1;
    let json_content_end = body[json_content_start..]
        .find("</script>")
        .context("JSON-LD script tag end not found")?
        + json_content_start;

    let json_str = &body[json_content_start..json_content_end];
    let json: serde_json::Value =
        serde_json::from_str(json_str).context("Failed to parse JSON-LD")?;

    // Extract data from JSON-LD structure
    let name = match json["name"].as_str() {
        Some(n) => n,
        None => return Err(anyhow::anyhow!("Name not found in JSON-LD")),
    };
    let price_val = json["offers"]["price"].as_f64().unwrap_or(0.0);
    let price = if price_val > 0.0 {
        price_val.to_string()
    } else {
        return Err(anyhow::anyhow!("Price not found in JSON-LD"));
    };

    // Extract location from address
    let location = json["location"]["address"]["addressLocality"]
        .as_str()
        .unwrap_or("Unknown")
        .to_string();

    // Extract property type from URL structure or name
    let property_type = if name.to_lowercase().contains("wohnung") {
        "wohnung".to_string()
    } else if name.to_lowercase().contains("haus") {
        "haus".to_string()
    } else {
        "immobilie".to_string()
    };

    // Extract address
    let street = json["location"]["address"]["streetAddress"]
        .as_str()
        .unwrap_or("");
    let locality = json["location"]["address"]["addressLocality"]
        .as_str()
        .unwrap_or("");
    let address = if !street.is_empty() && !locality.is_empty() {
        Some(format!("{}, {}", street, locality))
    } else if !street.is_empty() {
        Some(street.to_string())
    } else {
        None
    };

    // Extract coordinates if available in JSON-LD
    let mut coordinates = if let (Some(lat), Some(lng)) = (
        json["location"]["geo"]["latitude"].as_f64(),
        json["location"]["geo"]["longitude"].as_f64(),
    ) {
        Some((lat, lng))
    } else {
        None
    };

    // If coordinates not in JSON-LD, try map data as fallback
    if coordinates.is_none() {
        coordinates = extract_coordinates_from_map(body);
    }

    // Extract living size from description
    let description = json["description"].as_str().unwrap_or("");
    let size_living = extract_size_from_text(description);

    // Extract date from datePublished or dateCreated in JSON-LD
    let date = json["datePublished"]
        .as_str()
        .or_else(|| json["dateCreated"].as_str())
        .and_then(|d| parse_date_string(d))
        .or_else(|| extract_date_from_html(body)); // Fallback to HTML parsing

    println!(
        "JSON-LD extracted: price={}, location={}, type={}, name={}, date={:?}",
        price, location, property_type, name, date
    );

    Ok(Property {
        url: "".to_string(), // Will be set by caller
        price,
        location,
        property_type,
        listing_type: ListingType::Available,
        date,
        coordinates,
        address,
        size_living,
    })
}

fn extract_location_from_breadcrumbs(document: &Html) -> Result<String> {
    // Look for breadcrumb navigation
    let breadcrumb_selector =
        Selector::parse("a[href*='feldkirch'], a[href*='bregenz'], a[href*='dornbirn']")
            .map_err(|e| anyhow::anyhow!("Failed to parse breadcrumb selector: {:?}", e))?;

    if let Some(element) = document.select(&breadcrumb_selector).last() {
        let text = element
            .text()
            .collect::<Vec<_>>()
            .join(" ")
            .trim()
            .to_string();
        if !text.is_empty() {
            return Ok(text);
        }
    }

    Err(anyhow::anyhow!("Unknown"))
}

fn extract_address_from_location(document: &Html) -> Option<String> {
    // Look for address in the location section
    let location_selector = Selector::parse(".px-8.py-4.text-lg.uppercase").ok()?;

    if let Some(element) = document.select(&location_selector).next() {
        let text = element.text().collect::<Vec<_>>().join(" ");
        // Extract everything after the last comma (usually the street address)
        if let Some(last_comma) = text.rfind(',') {
            let address = text[last_comma + 1..].trim();
            if !address.is_empty() {
                return Some(address.to_string());
            }
        }
    }

    None
}

fn extract_coordinates_from_map(body: &str) -> Option<(f64, f64)> {
    // Look for coordinates in map data
    if let Some(start) = body
        .find("data-content-loader-url-value=\"/load-template/organisms/detail_page/map.html.twig")
    {
        if let Some(params_start) = body[start..].find("lat_long%5D=") {
            let coords_start = start + params_start + 12; // length of "lat_long%5D="
            if let Some(coords_end) = body[coords_start..].find('"') {
                let coords_str = &body[coords_start..coords_start + coords_end];
                let parts: Vec<&str> = coords_str.split(',').collect();
                if parts.len() == 2 {
                    if let (Ok(lat), Ok(lng)) = (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                        return Some((lat, lng));
                    }
                }
            }
        }
    }

    None
}

fn extract_size_from_text(text: &str) -> Option<String> {
    let size_regex = Regex::new(r"(\d+(?:,\d+)?)\s*m²").unwrap();
    if let Some(captures) = size_regex.captures(text) {
        if let Some(size) = captures.get(1) {
            return Some(size.as_str().replace(',', "."));
        }
    }
    None
}

fn extract_date_from_html(body: &str) -> Option<NaiveDate> {
    // Look for adReleaseDate in dataLayer script
    if let Some(start) = body.find("'adReleaseDate': `") {
        let date_start = start + 18; // length of "'adReleaseDate': `"
        if let Some(date_end) = body[date_start..].find('`') {
            let date_str = &body[date_start..date_start + date_end];
            return parse_date_string(date_str);
        }
    }

    // Look for other date patterns in dataLayer
    let date_patterns = [
        r#"'adReleaseDate':\s*`([^`]+)`"#,
        r#""adReleaseDate":\s*"([^"]+)""#,
        r#""release":\s*"([^"]+)""#,
        r#""datePublished":\s*"([^"]+)""#,
        r#""dateCreated":\s*"([^"]+)""#,
        r#"release[^:]*:\s*"([^"]+)""#,
        r#"published[^:]*:\s*"([^"]+)""#,
    ];

    for pattern in &date_patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if let Some(captures) = regex.captures(body) {
                if let Some(date) = captures.get(1) {
                    return parse_date_string(date.as_str());
                }
            }
        }
    }

    None
}

fn parse_date_string(date_str: &str) -> Option<NaiveDate> {
    // Try common date formats
    let formats = [
        "%Y-%m-%d", // 2025-07-25
        "%d.%m.%Y", // 25.07.2025
        "%d/%m/%Y", // 25/07/2025
        "%Y/%m/%d", // 2025/07/25
    ];

    for format in &formats {
        if let Ok(date) = NaiveDate::parse_from_str(date_str, format) {
            return Some(date);
        }
    }

    None
}
