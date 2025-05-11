use anyhow::{Result, anyhow};
use reqwest::blocking::Client;
use scraper::{Html, Selector};
use crate::models::Property;
use crate::parser;

const BASE_URL: &str = "https://www.vol.at/themen/grund-und-boden";

pub fn scrape_all_index_pages(max_pages: usize) -> Result<Vec<String>> {
    let mut all_property_urls = Vec::new();
    
    // Scrape the first page
    let first_page_urls = scrape_index_page_with_number(1)?;
    all_property_urls.extend(first_page_urls);
    println!("Page 1: Found {} property URLs", first_page_urls.len());
    
    // Scrape additional pages if max_pages > 1
    for page_num in 2..=max_pages {
        println!("Scraping page {} of {}", page_num, max_pages);
        match scrape_index_page_with_number(page_num) {
            Ok(urls) => {
                println!("Page {}: Found {} property URLs", page_num, urls.len());
                if urls.is_empty() {
                    println!("No more properties found on page {}, stopping pagination", page_num);
                    break;
                }
                all_property_urls.extend(urls);
            },
            Err(e) => {
                eprintln!("Error scraping page {}: {}", page_num, e);
                break;
            }
        }
    }
    
    println!("Total property URLs found: {}", all_property_urls.len());
    Ok(all_property_urls)
}

pub fn scrape_index_page() -> Result<Vec<String>> {
    scrape_index_page_with_number(1)
}

fn scrape_index_page_with_number(page_number: usize) -> Result<Vec<String>> {
    let client = Client::new();
    let url = if page_number == 1 {
        BASE_URL.to_string()
    } else {
        format!("{}/page/{}", BASE_URL, page_number)
    };
    
    println!("Fetching index page: {}", url);
    let response = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
        .send()?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch index page: HTTP {}", response.status()));
    }
    
    let html = response.text()?;
    let document = Html::parse_document(&html);
    
    // Extract property URLs from the page
    // This selector needs to be adjusted based on the actual HTML structure
    let article_selector = Selector::parse("article a.article-link").unwrap();
    
    let property_urls: Vec<String> = document
        .select(&article_selector)
        .filter_map(|element| {
            element.value().attr("href").map(|href| {
                if href.starts_with("http") {
                    href.to_string()
                } else {
                    format!("https://www.vol.at{}", href)
                }
            })
        })
        .collect();
    
    Ok(property_urls)
}

pub fn scrape_property_page(url: &str, cookies: Option<&str>) -> Result<Property> {
    println!("Scraping property page: {}", url);
    
    let client = Client::builder()
        .build()?;
    
    let mut request = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36");
    
    if let Some(cookie_str) = cookies {
        println!("Using cookies for authentication");
        request = request.header("Cookie", cookie_str);
    }
    
    let response = request.send()?;
    
    if !response.status().is_success() {
        return Err(anyhow!("Failed to fetch property page: HTTP {}", response.status()));
    }
    
    let html = response.text()?;
    println!("Successfully retrieved property page content");
    
    let document = Html::parse_document(&html);
    
    // Parse the HTML document to extract property details
    let property = parser::parse_property_page(&document, url)?;
    println!("Successfully extracted property data: {} at {}", property.property_type, property.location);
    
    Ok(property)
}
