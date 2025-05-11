use crate::models::Property;
use crate::parser;
use anyhow::{Result, Context};
use scraper::{Html, Selector};
use serde_json::Value;

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
        .get(url);
    
    if let Some(cookie_str) = cookies {
        request = request.header("Cookie", cookie_str);
    }

    // Fetch the property page
    let response = request.send()
        .context("Failed to fetch property page")?;
    let html = response.text()
        .context("Failed to get response text")?;
    
    // Parse the HTML
    let document = Html::parse_document(&html);
    
    // Save cleaned property page HTML for debugging
    // Filter out script tags and other trackers
    let cleaned_html = {
        let document = Html::parse_document(&html);
        let script_selector = Selector::parse("script").unwrap();
        let iframe_selector = Selector::parse("iframe").unwrap();
        let noscript_selector = Selector::parse("noscript").unwrap();
        
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
    
    // Select the headline - adjust selector based on actual HTML structure
    let headline_selector = Selector::parse("h1.article-headline").unwrap();
    // Print all headlines found for debugging
    let headlines: Vec<String> = document.select(&headline_selector)
        .map(|el| el.text().collect::<String>())
        .collect();
    
    println!("Found headlines: {:?}", headlines);
    
    let headline = headlines.first()
        .context("Headline not found")?
        .to_string();
    
    // Parse the headline using regex patterns
    let price = parser::extract_price(&headline)?;
    let location = parser::extract_location(&headline)?;
    let property_type = parser::extract_property_type(&headline)?;
    
    // Create and return the Property
    Ok(Property {
        url: url.to_string(),
        price,
        location,
        property_type,
        date: None,
        description: None,
    })
}
