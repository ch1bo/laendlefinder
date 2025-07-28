use crate::models::{ListingType, Property, PropertyType};
use crate::tui::ScraperTUI;
use crate::{debug_println, debug_eprintln};
use anyhow::{Context, Result};
use chrono::NaiveDate;
use regex::Regex;
use scraper::{Html, Selector};

const BASE_URL: &str = "https://www.laendleimmo.at/kaufobjekt";

pub fn scrape_all_listing_pages(max_pages: usize, mut tui: Option<&mut ScraperTUI>) -> Result<Vec<String>> {
    let mut all_property_urls = Vec::new();

    if let Some(tui) = tui.as_mut() {
        tui.start_gathering(max_pages)?;
    }

    for page in 1..=max_pages {
        let page_url = if page == 1 {
            BASE_URL.to_string()
        } else {
            format!("{}?page={}", BASE_URL, page)
        };

        debug_println!("Scraping listing page: {}", page_url);

        match scrape_listing_page(&page_url) {
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

pub fn scrape_listing_page(url: &str) -> Result<Vec<String>> {
    debug_println!("Fetching listing page: {}", url);

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

    debug_println!("Found {} property URLs on page", property_urls.len());
    Ok(property_urls)
}

pub fn scrape_property_page(url: &str) -> Result<Property> {
    debug_println!("Scraping property page: {}", url);

    let response = reqwest::blocking::Client::new()
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()
        .context("Failed to fetch property page")?;

    let body = response.text().context("Failed to read response body")?;
    let document = Html::parse_document(&body);

    // Try to extract from JSON-LD first (most reliable)
    if let Ok(mut json_data) = extract_from_json_ld(&body, url) {
        debug_println!("Successfully extracted from JSON-LD");
        json_data.url = url.to_string(); // Set the URL
        return Ok(json_data);
    }

    // Fallback to HTML parsing
    debug_println!("JSON-LD extraction failed, falling back to HTML parsing");
    let title = extract_title(&document)?;
    let price = extract_price(&document)?;
    let location = extract_location(&document, url)?;
    let property_type = extract_property_type(&document, url);
    let address = extract_address_from_location(&document);
    let size_living = extract_living_size(&document);
    let size_ground = extract_ground_size(&document);
    debug_println!("HTML fallback extracted living size: {:?}", size_living);
    debug_println!("HTML fallback extracted ground size: {:?}", size_ground);
    let coordinates = extract_coordinates_from_map(&body);
    let date = extract_date_from_html(&body);

    debug_println!(
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
        size_ground,
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

    Ok("Unknown Property".to_string())
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

    Ok("Unknown".to_string())
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

fn extract_property_type(document: &Html, url: &str) -> PropertyType {
    // Try to extract from URL first - look at the full path for better classification
    if let Some(classified) = classify_property_type_from_url(url) {
        return classified;
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
                    let classified = PropertyType::from_string(&text);
                    if !matches!(classified, PropertyType::Unknown) {
                        return classified;
                    }
                }
            }
        }
    }

    PropertyType::Unknown
}

/// Enhanced URL-based property type classification for laendleimmo.at URLs
fn classify_property_type_from_url(url: &str) -> Option<PropertyType> {
    // Laendleimmo URLs follow pattern: /immobilien/{main_type}/{sub_type}/vorarlberg/{district}/{id}
    let url_regex = Regex::new(r"/immobilien/([^/]+)/([^/]+)/").unwrap();
    if let Some(captures) = url_regex.captures(url) {
        let main_type = captures.get(1)?.as_str().to_lowercase();
        let sub_type = captures.get(2)?.as_str().to_lowercase();
        
        // Combine main and sub type for better classification
        let combined = format!("{} {}", main_type, sub_type);
        
        // Direct mapping for known patterns
        match main_type.as_str() {
            "grundstuck" | "grundstueck" => return Some(PropertyType::Land),
            "wohnung" => return Some(PropertyType::Apartment),
            "haus" => return Some(PropertyType::House),
            _ => {}
        }
        
        // Check sub-type patterns for more specific classification
        if sub_type.contains("grundstuck") || sub_type.contains("grundstueck") || 
           sub_type.contains("baugrund") || sub_type.contains("bauplatz") {
            return Some(PropertyType::Land);
        }
        
        if sub_type.contains("wohnung") || sub_type.contains("apartment") {
            return Some(PropertyType::Apartment);
        }
        
        if sub_type.contains("haus") || sub_type.contains("villa") {
            return Some(PropertyType::House);
        }
        
        // Fall back to using the existing PropertyType::from_string logic on combined text
        let classified = PropertyType::from_string(&combined);
        if !matches!(classified, PropertyType::Unknown) {
            return Some(classified);
        }
    }
    
    None
}

fn extract_living_size(document: &Html) -> Option<String> {
    // First, try to extract from specific detailed sections
    let detail_selectors = [
        "#accordion-collapse",
        ".object-details",
        ".object-info", 
        ".property-details",
        "#sticky-subheader",
        ".details",
    ];
    
    for selector_str in &detail_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join(" ");
                if let Some(size) = extract_living_size_from_text(&text) {
                    debug_println!("Found living size in {}: {}", selector_str, size);
                    return Some(size);
                }
            }
        }
    }
    
    // Second, try to extract from the full HTML text
    let full_text = document.root_element().text().collect::<Vec<_>>().join(" ");
    if let Some(size) = extract_living_size_from_text(&full_text) {
        debug_println!("Found living size in full text: {}", size);
        return Some(size);
    }
    
    // Third, try more specific element searches
    let size_selectors = [
        "div", // General div elements that might contain property details
        "span", // General span elements
        "td", // Table cells often contain property details
        "li", // List items
        "p", // Paragraphs
    ];

    for selector_str in &size_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join(" ");
                if let Some(size) = extract_living_size_from_text(&text) {
                    debug_println!("Found living size in {} element: {}", selector_str, size);
                    return Some(size);
                }
            }
        }
    }

    debug_println!("No living size found in document");
    None
}

fn extract_ground_size(document: &Html) -> Option<String> {
    // First, try to extract from specific object-details structure
    let object_details_selectors = [
        ".object-details",
        ".object-info", 
        ".property-details",
        "#sticky-subheader",
        ".details",
    ];
    
    for selector_str in &object_details_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join(" ");
                if let Some(size) = extract_ground_size_from_text(&text) {
                    debug_println!("Found ground size in {}: {}", selector_str, size);
                    return Some(size);
                }
            }
        }
    }
    
    // Second, try to extract from the full HTML text for patterns like "Grundstücksgröße 700,00 m²"
    let full_text = document.root_element().text().collect::<Vec<_>>().join(" ");
    if let Some(size) = extract_ground_size_from_text(&full_text) {
        debug_println!("Found ground size in full text: {}", size);
        return Some(size);
    }
    
    // Third, try more specific element searches
    let size_selectors = [
        "div", // General div elements that might contain object details
        "span", // General span elements
        "td", // Table cells often contain property details
        "li", // List items
        "p", // Paragraphs
    ];

    for selector_str in &size_selectors {
        if let Ok(selector) = Selector::parse(selector_str) {
            for element in document.select(&selector) {
                let text = element.text().collect::<Vec<_>>().join(" ");
                // Use the improved text extraction function
                if let Some(size) = extract_ground_size_from_text(&text) {
                    debug_println!("Found ground size in {} element: {}", selector_str, size);
                    return Some(size);
                }
            }
        }
    }

    debug_println!("No ground size found in document");
    None
}

fn extract_from_json_ld(body: &str, url: &str) -> Result<Property> {
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

    // Extract property type from URL first, then fall back to name classification
    let property_type = classify_property_type_from_url(url)
        .unwrap_or_else(|| PropertyType::from_string(name));

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

    // Extract living size and ground size from description
    let description = json["description"].as_str().unwrap_or("");
    let mut size_living = extract_living_size_from_text(description);
    let mut size_ground = extract_ground_size_from_text(description);
    
    // If sizes not found in description, try extracting from full HTML body
    let document = Html::parse_document(body);
    
    if size_living.is_none() {
        size_living = extract_living_size(&document);
        debug_println!("Living size not in JSON-LD description, tried HTML extraction: {:?}", size_living);
    }
    
    if size_ground.is_none() {
        size_ground = extract_ground_size(&document);
        debug_println!("Ground size not in JSON-LD description, tried HTML extraction: {:?}", size_ground);
    }
    
    debug_println!("JSON-LD description for size extraction: {}", description);
    debug_println!("JSON-LD extracted living size: {:?}, ground size: {:?}", size_living, size_ground);

    // Extract date from datePublished or dateCreated in JSON-LD
    let date = json["datePublished"]
        .as_str()
        .or_else(|| json["dateCreated"].as_str())
        .and_then(|d| parse_date_string(d))
        .or_else(|| extract_date_from_html(body)); // Fallback to HTML parsing

    debug_println!(
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
        size_ground,
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

fn extract_living_size_from_text(text: &str) -> Option<String> {
    // Look for various German living area patterns
    let patterns = [
        // Wohnfläche 126,00 m²
        r"wohnfl[äa]che[:\s]*(\d+(?:[.,]\d+)?)\s*m²",
        // Nutzfläche 126,00 m²
        r"nutzfl[äa]che[:\s]*(\d+(?:[.,]\d+)?)\s*m²",
        // Living area: 126,00 m²
        r"living\s*area[:\s]*(\d+(?:[.,]\d+)?)\s*m²",
        // 126 m² Wohnfläche
        r"(\d+(?:[.,]\d+)?)\s*m²\s*wohnfl[äa]che",
        // 126 m² living
        r"(\d+(?:[.,]\d+)?)\s*m²\s*(?:living|wohn)",
    ];
    
    let lower_text = text.to_lowercase();
    
    for pattern in &patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if let Some(captures) = regex.captures(&lower_text) {
                if let Some(size) = captures.get(1) {
                    return Some(size.as_str().replace(',', "."));
                }
            }
        }
    }
    
    // Fallback: first size that's not explicitly ground size and not in ground context
    let size_regex = Regex::new(r"(\d+(?:[.,]\d+)?)\s*m²").unwrap();
    for captures in size_regex.captures_iter(&lower_text) {
        if let Some(size_match) = captures.get(0) {
            let before_match = &lower_text[..size_match.start()];
            let after_match = &lower_text[size_match.end()..];
            
            // Skip if this looks like ground size
            if before_match.contains("grundstück") || before_match.contains("grundstueck") ||
               before_match.contains("grund") || after_match.starts_with("grund") ||
               before_match.contains("parzel") || before_match.contains("bauland") {
                continue;
            }
            
            // Prefer if it's clearly about living/interior space
            if before_match.contains("wohn") || before_match.contains("nutz") ||
               before_match.contains("living") || after_match.starts_with("wohn") {
                if let Some(size) = captures.get(1) {
                    return Some(size.as_str().replace(',', "."));
                }
            }
        }
    }
    
    None
}

fn extract_ground_size_from_text(text: &str) -> Option<String> {
    // Look for various German ground size patterns, being specific to avoid living area
    let patterns = [
        // Grundstücksgröße 700,00 m²
        r"grundst[üu]cksgr[öo][sß]e[:\s]*(\d+(?:[.,]\d+)?)\s*m²",
        // Grundstücksfläche 700,00 m²
        r"grundst[üu]cksfl[äa]che[:\s]*(\d+(?:[.,]\d+)?)\s*m²",
        // Grundstück: 700,00 m²  
        r"grundst[üu]ck[:\s]*(\d+(?:[.,]\d+)?)\s*m²",
        // Mit 700 m² bietet es... (but only if not talking about living area)
        r"mit\s+(\d+(?:[.,]\d+)?)\s*m²(?!\s*wohnfl[äa]che)",
        // Plot size, parcel size
        r"parzellenfl[äa]che[:\s]*(\d+(?:[.,]\d+)?)\s*m²",
        r"baulandfl[äa]che[:\s]*(\d+(?:[.,]\d+)?)\s*m²",
    ];
    
    let lower_text = text.to_lowercase();
    
    // Skip if this text is clearly about living area, not ground area
    if lower_text.contains("wohnfläche") || lower_text.contains("wohnflaeche") ||
       lower_text.contains("nutzfläche") || lower_text.contains("nutzflaeche") {
        // Only look for ground-specific patterns in mixed content
        let ground_specific_patterns = [
            r"grundst[üu]cksgr[öo][sß]e[:\s]*(\d+(?:[.,]\d+)?)\s*m²",
            r"grundst[üu]cksfl[äa]che[:\s]*(\d+(?:[.,]\d+)?)\s*m²",
        ];
        
        for pattern in &ground_specific_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                if let Some(captures) = regex.captures(&lower_text) {
                    if let Some(size) = captures.get(1) {
                        return Some(size.as_str().replace(',', "."));
                    }
                }
            }
        }
        return None;
    }
    
    for pattern in &patterns {
        if let Ok(regex) = Regex::new(pattern) {
            if let Some(captures) = regex.captures(&lower_text) {
                if let Some(size) = captures.get(1) {
                    return Some(size.as_str().replace(',', "."));
                }
            }
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
