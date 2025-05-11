use crate::models::Property;
use crate::parser;
use anyhow::{Result, Context};
use scraper::{Html, Selector};
use serde_json::Value;
use chrono::NaiveDate;

const INDEX_URL: &str = "https://www.vol.at/themen/grund-und-boden";

pub fn scrape_index_page() -> Result<Vec<String>> {
    println!("Scraping index page: {}", INDEX_URL);
    
    // Fetch the index page
    let response = reqwest::blocking::get(INDEX_URL)
        .context("Failed to fetch index page")?;
    let html = response.text()
        .context("Failed to get response text")?;
    
    // Dump complete HTML to file for debugging
    std::fs::write("debug_index.html", &html)
        .context("Failed to write debug HTML file")?;
    println!("Dumped complete HTML to debug_index.html");
    
    // Parse the HTML
    let document = Html::parse_document(&html);
    
    // Find the script tag containing the JSON data
    let script_selector = Selector::parse("#topicDataNode").unwrap();
    let script = document.select(&script_selector)
        .next()
        .context("Topic data script not found")?;
    
    // Parse the JSON content
    let json_str = script.inner_html();
    let json: Value = serde_json::from_str(&json_str)
        .context("Failed to parse JSON data")?;
    
    // Extract just the first link from hits array
    let mut links = Vec::new();
    if let Some(hits) = json["prefetchedRawData"]["hits"].as_array() {
        if let Some(hit) = hits.first() {
            if let Some(link) = hit["link"].as_str() {
                links.push(link.replace(r"\/", "/").to_string());
            }
        }
    }
    
    println!("Taking first property link from JSON data");
    
    Ok(links)
}

pub fn scrape_property_page(url: &str, cookies: Option<&str>) -> Result<Property> {
    println!("Scraping property page: {}", url);
    
    // Build request with optional cookies
    let mut request = reqwest::blocking::Client::new()
        .get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36");
    
    if let Some(cookie_str) = cookies {
        println!("Using cookies: {}", cookie_str);
        // Try to add cookies, but continue even if it fails
        match reqwest::header::HeaderValue::from_str(cookie_str) {
            Ok(header_value) => {
                request = request.header("Cookie", header_value);
            },
            Err(e) => {
                println!("Warning: Could not use cookies due to invalid format: {}", e);
                println!("Continuing without cookies");
            }
        }
    } else {
        println!("No cookies provided");
    }

    // Fetch the property page
    let response = match request.send() {
        Ok(resp) => {
            println!("Response status: {}", resp.status());
            println!("Response headers: {:#?}", resp.headers());
            
            if !resp.status().is_success() {
                return Err(anyhow::anyhow!("HTTP error status: {}", resp.status()));
            }
            resp
        },
        Err(e) => {
            eprintln!("Network error for {}: {:?}", url, e);
            return Err(anyhow::anyhow!("Failed to fetch property page: {}", e));
        }
    };
    
    let html = match response.text() {
        Ok(text) => {
            println!("Received HTML content of length: {} bytes", text.len());
            if text.len() < 100 {
                println!("Suspiciously short HTML content: {}", text);
            }
            text
        },
        Err(e) => {
            eprintln!("Failed to get response text for {}: {:?}", url, e);
            return Err(anyhow::anyhow!("Failed to get response text: {}", e));
        }
    };
    
    // Save raw HTML for debugging
    std::fs::write("debug_property_raw.html", &html)
        .context("Failed to write raw debug property HTML file")?;
    println!("Dumped raw property page HTML to debug_property_raw.html");
    
    // Parse the HTML
    let document = Html::parse_document(&html);
    
    // Save cleaned property page HTML for debugging
    // Filter out script tags and other trackers
    let cleaned_html = {
        let document = Html::parse_document(&html);
        let _script_selector = Selector::parse("script").unwrap();
        let _iframe_selector = Selector::parse("iframe").unwrap();
        let _noscript_selector = Selector::parse("noscript").unwrap();
        
        // Get all text nodes except those inside script/iframe/noscript
        let mut output = String::new();
        document.root_element()
            .descendants()
            .filter(|n| n.value().is_text())
            .filter(|n| {
                let parent_is_blocked = n.parent()
                    .and_then(|p| p.value().as_element())
                    .map(|e| {
                        let name = e.name();
                        name != "script" && name != "iframe" && name != "noscript"
                    })
                    .unwrap_or(true);
                parent_is_blocked
            })
            .for_each(|n| {
                if let Some(text) = n.value().as_text() {
                    output.push_str(text);
                    output.push('\n');
                }
            });
        output
    };
    
    std::fs::write("debug_property.html", cleaned_html)
        .context("Failed to write debug property HTML file")?;
    println!("Dumped cleaned property page HTML to debug_property.html");
    
    // Try to extract data from embedded JavaScript
    let script_selector = Selector::parse("#externalPostDataNode").unwrap();
    if let Some(script) = document.select(&script_selector).next() {
        println!("Found externalPostDataNode script tag");
        let json_str = script.inner_html();
        
        // Save the JSON for debugging
        std::fs::write("debug_property_json.json", &json_str)
            .context("Failed to write debug JSON file")?;
        
        // Parse the JSON content
        let json: Value = serde_json::from_str(&json_str)
            .context("Failed to parse JSON data from externalPostDataNode")?;
        
        // Extract property data from the JSON
        return extract_property_from_json(json, url);
    }
    
    // Fallback to traditional HTML parsing if JavaScript data not found
    println!("JavaScript data not found, falling back to HTML parsing");
    
    // Try different headline selectors
    let headline_selectors = [
        "h1.article-headline",
        "h1",
        ".article-headline",
        ".headline",
        "header h1",
        "article h1"
    ];
    
    let mut headline = String::new();
    for selector_str in headline_selectors {
        println!("Trying headline selector: {}", selector_str);
        if let Ok(selector) = Selector::parse(selector_str) {
            let headlines: Vec<String> = document.select(&selector)
                .map(|el| {
                    let text = el.text().collect::<String>();
                    println!("Found with '{}': '{}'", selector_str, text);
                    text
                })
                .collect();
            
            if let Some(first_headline) = headlines.first() {
                headline = first_headline.to_string();
                println!("Selected headline: '{}'", headline);
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
    let property_type = parser::extract_property_type(&headline)?;
    
    println!("Extracted data: price={}, location={}, type={}", 
             price, location, property_type);
    
    // Create and return the Property
    Ok(Property {
        url: url.to_string(),
        price: price.to_string(),
        location,
        property_type,
        date: None,
        description: None,
    })
}

fn extract_property_from_json(json: Value, url: &str) -> Result<Property> {
    println!("Extracting property data from JSON");
    
    // Navigate to the content section where property details are stored
    let post = &json["content"]["data"]["post"];
    
    // Extract the title which contains price and location information
    let title = post["title"].as_str()
        .context("Title not found in JSON data")?;
    println!("Title from JSON: {}", title);
    
    // Extract price, location, and property type from the title
    let price = parser::extract_price(title)?;
    let location = parser::extract_location(title)?;
    let property_type = parser::extract_property_type(title)?;
    
    // Try to extract the transaction date from the structured data
    let mut date = None;
    
    // Look for the GrundUndBoden block which contains structured data
    if let Some(blocks) = post["blocks"].as_array() {
        for block in blocks {
            if block["ot"] == "russmedia/grund-und-boden" {
                if let Some(data_str) = block["a"].as_array()
                    .and_then(|attrs| attrs.iter().find(|attr| attr["key"] == "data"))
                    .and_then(|attr| attr["value"].as_str()) {
                    
                    // Parse the data attribute which is a JSON string
                    if let Ok(data_json) = serde_json::from_str::<Value>(data_str) {
                        if let Some(date_str) = data_json["transactionDate"].as_str() {
                            println!("Found transaction date: {}", date_str);
                            // Parse the date in format YYYY-MM-DD
                            if let Ok(parsed_date) = NaiveDate::parse_from_str(date_str, "%Y-%m-%d") {
                                date = Some(parsed_date);
                            }
                        }
                    }
                }
            }
        }
    }
    
    // Try to extract description from the content
    let mut description = None;
    if let Some(content) = post["content"].as_str() {
        // Extract text from HTML content
        let fragment = Html::parse_fragment(content);
        let text: String = fragment.root_element()
            .descendants()
            .filter_map(|n| {
                if n.value().is_text() {
                    n.value().as_text().map(|t| t.trim().to_string())
                } else {
                    None
                }
            })
            .filter(|t| !t.is_empty())
            .collect::<Vec<String>>()
            .join(" ");
        
        if !text.is_empty() {
            description = Some(text);
        }
    }
    
    println!("Extracted data from JSON: price={}, location={}, type={}, date={:?}", 
             price, location, property_type, date);
    
    // Create and return the Property
    Ok(Property {
        url: url.to_string(),
        price: price.to_string(),
        location,
        property_type,
        date,
        description,
    })
}
