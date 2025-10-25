use anyhow::Result;
use reqwest::blocking::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use crate::models::Property;
use crate::debug_println;
use crossterm::{
    cursor::MoveToPreviousLine,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io;

#[derive(Debug, Deserialize)]
struct NominatimResponse {
    lat: String,
    lon: String,
}

pub struct GeocodingTUI {
    total_properties: usize,
    geocoded_count: usize,
    failed_count: usize,
    current_index: usize,
}

impl GeocodingTUI {
    pub fn new(total_properties: usize) -> Self {
        Self {
            total_properties,
            geocoded_count: 0,
            failed_count: 0,
            current_index: 0,
        }
    }

    pub fn start_geocoding(&self) -> io::Result<()> {
        execute!(
            io::stdout(),
            SetForegroundColor(Color::White),
            Print(format!("🗺️  Found {} properties needing geocoding...\n", self.total_properties)),
            ResetColor
        )?;
        self.show_progress()?;
        Ok(())
    }

    pub fn update_progress(&mut self, geocoded: bool, property_name: &str, address: &str) -> io::Result<()> {
        self.current_index += 1;
        if geocoded {
            self.geocoded_count += 1;
        } else {
            self.failed_count += 1;
        }

        // Move back and update progress
        execute!(
            io::stdout(),
            MoveToPreviousLine(1),
            Clear(ClearType::CurrentLine),
        )?;

        self.show_progress()?;

        // Show current property being processed
        if geocoded {
            execute!(
                io::stdout(),
                SetForegroundColor(Color::Green),
                Print(format!("✓ Geocoded: {} ({})\n", property_name, address)),
                ResetColor
            )?;
        } else {
            execute!(
                io::stdout(),
                SetForegroundColor(Color::Yellow),
                Print(format!("⚠ Skipped: {} (no address or geocoding failed)\n", property_name)),
                ResetColor
            )?;
        }

        Ok(())
    }

    pub fn complete_geocoding(&self) -> io::Result<()> {
        // Move back and clear the last property line
        execute!(
            io::stdout(),
            MoveToPreviousLine(1),
            Clear(ClearType::CurrentLine),
        )?;

        // Show final summary
        execute!(
            io::stdout(),
            SetForegroundColor(Color::Green),
            Print(format!("✅ Geocoding completed: {} geocoded, {} skipped\n", 
                         self.geocoded_count, self.failed_count)),
            ResetColor
        )?;

        Ok(())
    }

    pub fn show_interruption_summary(&self) -> io::Result<()> {
        let remaining = self.total_properties - self.current_index;
        execute!(
            io::stdout(),
            SetForegroundColor(Color::Yellow),
            Print(format!("\n⚠ Geocoding interrupted: {} geocoded, {} skipped, {} remaining properties still need coordinates\n", 
                         self.geocoded_count, self.failed_count, remaining)),
            ResetColor
        )?;
        Ok(())
    }

    pub fn single_property_result(&self, geocoded: bool, property_name: &str) -> io::Result<()> {
        if geocoded {
            execute!(
                io::stdout(),
                SetForegroundColor(Color::Green),
                Print(format!("✅ Successfully geocoded: {}\n", property_name)),
                ResetColor
            )?;
        } else {
            execute!(
                io::stdout(),
                SetForegroundColor(Color::Yellow),
                Print(format!("⚠ Property already had coordinates or couldn't be geocoded: {}\n", property_name)),
                ResetColor
            )?;
        }
        Ok(())
    }

    fn show_progress(&self) -> io::Result<()> {
        let percentage = if self.total_properties > 0 {
            (self.current_index * 100) / self.total_properties
        } else {
            0
        };

        let remaining = self.total_properties - self.current_index;

        execute!(
            io::stdout(),
            SetForegroundColor(Color::Blue),
            Print(format!("📍 Progress: {}/{} ({}%) - {} geocoded, {} skipped{}\n", 
                         self.current_index, self.total_properties, percentage,
                         self.geocoded_count, self.failed_count,
                         if remaining > 0 { format!(", {} remaining", remaining) } else { String::new() })),
            ResetColor
        )?;

        Ok(())
    }
}

pub struct Geocoder {
    client: Client,
    cache: HashMap<String, Option<(f64, f64)>>,
    request_count: usize,
    rate_limit_delay_ms: u64,
}

impl Geocoder {
    pub fn new() -> Result<Self> {
        let client = Client::builder()
            .user_agent("LaendleFinder/1.0 (Real Estate Scraper)")
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Geocoder {
            client,
            cache: HashMap::new(),
            request_count: 0,
            rate_limit_delay_ms: 0, // No delay - test maximum speed
        })
    }

    fn rate_limit(&mut self) {
        if self.request_count > 0 && self.rate_limit_delay_ms > 0 {
            debug_println!("Rate limiting: sleeping for {}ms...", self.rate_limit_delay_ms);
            thread::sleep(Duration::from_millis(self.rate_limit_delay_ms));
        }
        self.request_count += 1;
    }

    pub fn geocode_address(&mut self, address: &str) -> Result<Option<(f64, f64)>> {
        if address.trim().is_empty() {
            return Ok(None);
        }

        let cache_key = address.to_lowercase().trim().to_string();
        
        // Check cache first
        if let Some(cached_result) = self.cache.get(&cache_key) {
            debug_println!("Cache hit for address: {}", address);
            return Ok(*cached_result);
        }

        // Rate limit before making request
        self.rate_limit();

        // Enhance address for Austrian context
        let enhanced_address = if address.contains("Austria") || address.contains("Österreich") {
            address.to_string()
        } else {
            format!("{}, Austria", address)
        };

        debug_println!("Geocoding address: {} -> {}", address, enhanced_address);

        let url = format!(
            "https://nominatim.openstreetmap.org/search?format=json&q={}&limit=1&countrycodes=at",
            urlencoding::encode(&enhanced_address)
        );

        let response = self.client.get(&url).send()?;
        
        if !response.status().is_success() {
            if response.status().as_u16() == 429 {
                println!("🚫 Rate limit hit (HTTP 429)! Adding {}ms delay for future requests.", self.rate_limit_delay_ms + 200);
                self.rate_limit_delay_ms = (self.rate_limit_delay_ms + 200).min(2000); // Cap at 2 seconds
                debug_println!("Rate limit hit for: {}", address);
                // Sleep longer on rate limit
                thread::sleep(Duration::from_secs(1));
            }
            debug_println!("HTTP error {}: {}", response.status(), url);
            self.cache.insert(cache_key, None);
            return Ok(None);
        }

        let responses: Vec<NominatimResponse> = response.json()?;
        
        let result = if let Some(geocode_result) = responses.first() {
            match (geocode_result.lat.parse::<f64>(), geocode_result.lon.parse::<f64>()) {
                (Ok(lat), Ok(lng)) => {
                    debug_println!("Successfully geocoded: {} -> ({}, {})", address, lat, lng);
                    Some((lat, lng))
                }
                _ => {
                    debug_println!("Failed to parse coordinates for: {}", address);
                    None
                }
            }
        } else {
            debug_println!("No results found for: {}", address);
            None
        };

        // Cache the result (even if None)
        self.cache.insert(cache_key, result);
        Ok(result)
    }

    pub fn geocode_property(&mut self, property: &mut Property) -> Result<bool> {
        // Skip if coordinates already exist
        if property.coordinates.is_some() {
            return Ok(false);
        }

        // Try address first, then location
        let address_to_geocode = if let Some(ref addr) = property.address {
            if !addr.trim().is_empty() {
                Some(addr.clone())
            } else {
                Some(property.location.clone())
            }
        } else {
            Some(property.location.clone())
        };

        if let Some(address) = address_to_geocode {
            if let Some((lat, lng)) = self.geocode_address(&address)? {
                property.coordinates = Some((lat, lng));
                debug_println!("Geocoded property: {} -> ({}, {})", 
                    property.name, lat, lng);
                return Ok(true);
            }
        }

        Ok(false)
    }
}

pub fn geocode_properties(properties: &mut Vec<Property>, output_file: &str) -> Result<usize> {
    let mut geocoder = Geocoder::new()?;

    // Collect indices of properties that need geocoding
    let indices_needing_geocode: Vec<usize> = properties.iter()
        .enumerate()
        .filter(|(_, p)| p.coordinates.is_none() && (p.address.is_some() || !p.location.trim().is_empty()))
        .map(|(i, _)| i)
        .collect();

    if indices_needing_geocode.is_empty() {
        debug_println!("No properties need geocoding");
        println!("📍 No properties need geocoding");
        return Ok(0);
    }

    let mut tui = GeocodingTUI::new(indices_needing_geocode.len());
    tui.start_geocoding()?;

    for index in indices_needing_geocode {
        let address_to_show = {
            let property = &properties[index];
            property.address.as_ref()
                .unwrap_or(&property.location)
                .clone()
        };
        
        let property_name = properties[index].name.clone();
        let geocoded = geocoder.geocode_property(&mut properties[index])?;
        
        // Save immediately after successful geocoding
        if geocoded {
            crate::utils::save_properties_to_csv(properties, output_file)?;
            debug_println!("Saved properties after geocoding: {}", property_name);
        }
        
        tui.update_progress(geocoded, &property_name, &address_to_show)?;
    }

    tui.complete_geocoding()?;
    Ok(tui.geocoded_count)
}

pub fn geocode_property_by_url(properties: &mut Vec<Property>, target_url: &str) -> Result<bool> {
    let mut geocoder = Geocoder::new()?;
    let tui = GeocodingTUI::new(1);
    
    // Find the property with the matching URL
    if let Some(property) = properties.iter_mut().find(|p| p.url == target_url) {
        if property.coordinates.is_some() {
            debug_println!("Property already has coordinates: {}", target_url);
            tui.single_property_result(false, &property.name)?;
            return Ok(false);
        }
        
        let geocoded = geocoder.geocode_property(property)?;
        tui.single_property_result(geocoded, &property.name)?;
        return Ok(geocoded);
    }
    
    debug_println!("Property not found in CSV: {}", target_url);
    Ok(false)
}