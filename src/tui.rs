use crossterm::{
    cursor::MoveToPreviousLine,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io;

pub struct ScraperTUI {
    summary_printed: bool,
    listing_line_printed: bool,
    gathering_line_printed: bool,
    property_lines: Vec<PropertyLineState>,
    current_property_index: Option<usize>,
}

#[derive(Clone)]
struct PropertyLineState {
    url: String,
    status: PropertyStatus,
}

#[derive(Clone, PartialEq)]
enum PropertyStatus {
    Pending,     // Grey
    InProgress,  // White with activity marker
    Completed,   // Green
    Failed,      // Red
}

impl ScraperTUI {
    pub fn new() -> Self {
        Self {
            summary_printed: false,
            listing_line_printed: false,
            gathering_line_printed: false,
            property_lines: Vec::new(),
            current_property_index: None,
        }
    }

    /// Show grey summary line with all loaded properties
    pub fn show_summary(&mut self, total_properties: usize) -> io::Result<()> {
        if !self.summary_printed {
            execute!(
                io::stdout(),
                SetForegroundColor(Color::DarkGrey),
                Print(format!("📁 Loaded {} existing properties\n", total_properties)),
                ResetColor
            )?;
            self.summary_printed = true;
        }
        Ok(())
    }

    /// Show initial gathering status
    pub fn start_gathering(&mut self, max_pages: usize) -> io::Result<()> {
        execute!(
            io::stdout(),
            SetForegroundColor(Color::White),
            Print(format!("⏳ Gathering URLs from listing pages (0/{})...\n", max_pages)),
            ResetColor
        )?;
        self.gathering_line_printed = true;
        Ok(())
    }

    /// Update gathering progress
    pub fn update_gathering_progress(&mut self, current_page: usize, max_pages: usize, urls_found: usize) -> io::Result<()> {
        if self.gathering_line_printed {
            // Move back to the gathering line and clear it
            execute!(
                io::stdout(),
                MoveToPreviousLine(1),
                Clear(ClearType::CurrentLine),
            )?;
        }

        let spinner = match current_page % 4 {
            0 => "⠋",
            1 => "⠙", 
            2 => "⠹",
            _ => "⠸",
        };

        execute!(
            io::stdout(),
            SetForegroundColor(Color::White),
            Print(format!(
                "{} Gathering URLs from listing pages ({}/{}) - {} URLs found\n",
                spinner, current_page, max_pages, urls_found
            )),
            ResetColor
        )?;

        self.gathering_line_printed = true;
        Ok(())
    }

    /// Finish gathering and show final count
    pub fn finish_gathering(&mut self, total_urls: usize) -> io::Result<()> {
        if self.gathering_line_printed {
            // Move back to the gathering line and clear it
            execute!(
                io::stdout(),
                MoveToPreviousLine(1),
                Clear(ClearType::CurrentLine),
            )?;
        }

        execute!(
            io::stdout(),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("✓ Gathered {} URLs from listing pages\n", total_urls)),
            ResetColor
        )?;

        self.gathering_line_printed = false; // Reset since we're done gathering
        Ok(())
    }

    /// Show live updated line about listing scraper
    pub fn update_listing_status(&mut self, new_count: usize, known_count: usize) -> io::Result<()> {
        if self.listing_line_printed {
            // Move back to the listing line and clear it
            execute!(
                io::stdout(),
                MoveToPreviousLine(self.property_lines.len() as u16 + 1),
                Clear(ClearType::CurrentLine),
            )?;
        }

        execute!(
            io::stdout(),
            SetForegroundColor(Color::White),
            Print(format!(
                "🔍 Found {} new, {} already known properties\n",
                new_count, known_count
            )),
            ResetColor
        )?;

        // Redraw all property lines if they exist
        for property_line in &self.property_lines {
            self.draw_property_line(property_line)?;
        }

        self.listing_line_printed = true;
        Ok(())
    }

    /// Show listing status for refresh mode
    pub fn update_listing_status_refresh(&mut self, new_count: usize, refresh_count: usize) -> io::Result<()> {
        if self.listing_line_printed {
            // Move back to the listing line and clear it
            execute!(
                io::stdout(),
                MoveToPreviousLine(self.property_lines.len() as u16 + 1),
                Clear(ClearType::CurrentLine),
            )?;
        }

        execute!(
            io::stdout(),
            SetForegroundColor(Color::White),
            Print(format!(
                "🔄 Found {} new, {} to be refreshed properties\n",
                new_count, refresh_count
            )),
            ResetColor
        )?;

        // Redraw all property lines if they exist
        for property_line in &self.property_lines {
            self.draw_property_line(property_line)?;
        }

        self.listing_line_printed = true;
        Ok(())
    }

    /// Add a new property to be scraped (initially greyed out)
    pub fn add_property(&mut self, url: String) -> io::Result<()> {
        let property_state = PropertyLineState {
            url: url.clone(),
            status: PropertyStatus::Pending,
        };

        execute!(
            io::stdout(),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("  ⏳ {}\n", Self::truncate_url(&url))),
            ResetColor
        )?;

        self.property_lines.push(property_state);
        Ok(())
    }

    /// Mark a property as currently being scraped (white with activity marker)
    pub fn start_scraping_property(&mut self, url: &str) -> io::Result<()> {
        if let Some(index) = self.find_property_index(url) {
            self.property_lines[index].status = PropertyStatus::InProgress;
            self.current_property_index = Some(index);
            self.update_property_line(index)?;
        }
        Ok(())
    }

    /// Update the activity marker for the currently active property
    pub fn update_activity(&mut self) -> io::Result<()> {
        if let Some(index) = self.current_property_index {
            if self.property_lines[index].status == PropertyStatus::InProgress {
                self.update_property_line(index)?;
            }
        }
        Ok(())
    }

    /// Mark a property as completed (green)
    pub fn complete_property(&mut self, url: &str) -> io::Result<()> {
        if let Some(index) = self.find_property_index(url) {
            self.property_lines[index].status = PropertyStatus::Completed;
            self.update_property_line(index)?;
            if Some(index) == self.current_property_index {
                self.current_property_index = None;
            }
        }
        Ok(())
    }

    /// Mark a property as failed (red)
    pub fn fail_property(&mut self, url: &str) -> io::Result<()> {
        if let Some(index) = self.find_property_index(url) {
            self.property_lines[index].status = PropertyStatus::Failed;
            self.update_property_line(index)?;
            if Some(index) == self.current_property_index {
                self.current_property_index = None;
            }
        }
        Ok(())
    }

    /// Show final summary
    pub fn show_final_summary(&mut self, _scraped_count: usize, total_count: usize) -> io::Result<()> {
        let completed = self.property_lines.iter().filter(|p| p.status == PropertyStatus::Completed).count();
        let failed = self.property_lines.iter().filter(|p| p.status == PropertyStatus::Failed).count();

        execute!(
            io::stdout(),
            SetForegroundColor(Color::White),
            Print(format!("\n✅ Scraping completed: {} successful, {} failed\n", completed, failed)),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("💾 Total properties in database: {}\n", total_count)),
            ResetColor
        )?;
        Ok(())
    }

    fn find_property_index(&self, url: &str) -> Option<usize> {
        self.property_lines.iter().position(|p| p.url == url)
    }

    fn update_property_line(&self, index: usize) -> io::Result<()> {
        // Calculate how many lines to move back
        let lines_back = self.property_lines.len() - index;
        
        execute!(
            io::stdout(),
            MoveToPreviousLine(lines_back as u16),
            Clear(ClearType::CurrentLine),
        )?;

        // Redraw this line
        self.draw_property_line(&self.property_lines[index])?;

        // Redraw all lines after this one
        for i in (index + 1)..self.property_lines.len() {
            self.draw_property_line(&self.property_lines[i])?;
        }

        Ok(())
    }

    fn draw_property_line(&self, property_line: &PropertyLineState) -> io::Result<()> {
        let (color, icon) = match property_line.status {
            PropertyStatus::Pending => (Color::DarkGrey, "⏳"),
            PropertyStatus::InProgress => (Color::White, "🔄"),
            PropertyStatus::Completed => (Color::Green, "✅"),
            PropertyStatus::Failed => (Color::Red, "❌"),
        };

        execute!(
            io::stdout(),
            SetForegroundColor(color),
            Print(format!("  {} {}\n", icon, Self::truncate_url(&property_line.url))),
            ResetColor
        )?;

        Ok(())
    }

    fn truncate_url(url: &str) -> String {
        if url.len() > 80 {
            format!("{}...", &url[..77])
        } else {
            url.to_string()
        }
    }
}

impl Default for ScraperTUI {
    fn default() -> Self {
        Self::new()
    }
}