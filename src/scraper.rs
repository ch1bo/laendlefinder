use crate::models::Property;
use crate::parser;
use anyhow::{Result, Context};
use scraper::{Html, Selector};

const INDEX_URL: &str = "https://www.vol.at/themen/grund-und-boden";

pub fn scrape_index_page() -> Result<Vec<String>> {
    println!("Scraping index page: {}", INDEX_URL);
    
    // Fetch the index page
    let response = reqwest::blocking::get(INDEX_URL)
        .context("Failed to fetch index page")?;
    let html = response.text()
        .context("Failed to get response text")?;
    
    // Parse the HTML
    let document = Html::parse_document(&html);
    
    // Select article links - this selector will need to be adjusted based on actual HTML structure
    let article_selector = Selector::parse("article a.article-link").unwrap();
    
    // Extract links
    let mut links = Vec::new();
    for element in document.select(&article_selector) {
        if let Some(href) = element.value().attr("href") {
            // Ensure we have absolute URLs
            let absolute_url = if href.starts_with("http") {
                href.to_string()
            } else {
                format!("https://www.vol.at{}", href)
            };
            links.push(absolute_url);
        }
    }
    
    Ok(links)
}

pub fn scrape_property_page(url: &str) -> Result<Property> {
    println!("Scraping property page: {}", url);
    
    // Fetch the property page
    let response = reqwest::blocking::get(url)
        .context("Failed to fetch property page")?;
    let html = response.text()
        .context("Failed to get response text")?;
    
    // Parse the HTML
    let document = Html::parse_document(&html);
    
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
