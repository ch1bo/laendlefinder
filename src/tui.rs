use crossterm::{
    cursor::MoveToPreviousLine,
    execute,
    style::{Color, Print, ResetColor, SetForegroundColor},
    terminal::{Clear, ClearType},
};
use std::io;

pub struct ScraperTUI {
    initial_lines_printed: usize,
    property_lines: Vec<PropertyLineState>,
    current_property_index: Option<usize>,
    total_properties_in_db: usize,
    new_count: usize,
    known_count: usize,
    is_refresh_mode: bool,
    progress_bar_printed: bool,
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
            initial_lines_printed: 0,
            property_lines: Vec::new(),
            current_property_index: None,
            total_properties_in_db: 0,
            new_count: 0,
            known_count: 0,
            is_refresh_mode: false,
            progress_bar_printed: false,
        }
    }

    /// Show grey summary line with all loaded properties
    pub fn show_summary(&mut self, total_properties: usize) -> io::Result<()> {
        self.total_properties_in_db = total_properties;
        execute!(
            io::stdout(),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("📁 Loaded {} existing properties\n", total_properties)),
            ResetColor
        )?;
        self.initial_lines_printed += 1;
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
        self.initial_lines_printed += 1;
        Ok(())
    }

    /// Update gathering progress
    pub fn update_gathering_progress(&mut self, current_page: usize, max_pages: usize, urls_found: usize, new_urls: usize, known_urls: usize) -> io::Result<()> {
        // Move back to the gathering line and clear it
        execute!(
            io::stdout(),
            MoveToPreviousLine(1),
            Clear(ClearType::CurrentLine),
        )?;

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
                "{} Gathering URLs from listing pages ({}/{}) - {} URLs found ({} new, {} known)\n",
                spinner, current_page, max_pages, urls_found, new_urls, known_urls
            )),
            ResetColor
        )?;

        Ok(())
    }

    /// Finish gathering and show final count
    pub fn finish_gathering(&mut self, total_urls: usize) -> io::Result<()> {
        // Move back to the gathering line and clear it
        execute!(
            io::stdout(),
            MoveToPreviousLine(1),
            Clear(ClearType::CurrentLine),
        )?;

        execute!(
            io::stdout(),
            SetForegroundColor(Color::DarkGrey),
            Print(format!("✓ Gathered {} URLs from listing pages\n", total_urls)),
            ResetColor
        )?;

        Ok(())
    }

    /// Show live updated line about listing scraper
    pub fn update_listing_status(&mut self, new_count: usize, known_count: usize) -> io::Result<()> {
        self.new_count = new_count;
        self.known_count = known_count;
        self.is_refresh_mode = false;
        
        execute!(
            io::stdout(),
            SetForegroundColor(Color::White),
            Print(format!(
                "🔍 Found {} new, {} already known properties\n",
                new_count, known_count
            )),
            ResetColor
        )?;
        self.initial_lines_printed += 1;
        Ok(())
    }

    /// Show listing status for refresh mode
    pub fn update_listing_status_refresh(&mut self, new_count: usize, refresh_count: usize) -> io::Result<()> {
        self.new_count = new_count;
        self.known_count = refresh_count;
        self.is_refresh_mode = true;
        
        execute!(
            io::stdout(),
            SetForegroundColor(Color::White),
            Print(format!(
                "🔄 Found {} new, {} to be refreshed properties\n",
                new_count, refresh_count
            )),
            ResetColor
        )?;
        self.initial_lines_printed += 1;
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

    /// Print initial progress bar (call this after all properties are added)
    pub fn show_initial_progress_bar(&mut self) -> io::Result<()> {
        if !self.progress_bar_printed && !self.property_lines.is_empty() {
            self.print_progress_bar()?;
            self.progress_bar_printed = true;
        }
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
            if self.progress_bar_printed {
                self.update_progress_bar_in_place()?;
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
            if self.progress_bar_printed {
                self.update_progress_bar_in_place()?;
            }
        }
        Ok(())
    }

    /// Show final summary
    pub fn show_final_summary(&mut self, _scraped_count: usize, total_count: usize) -> io::Result<()> {
        self.total_properties_in_db = total_count;
        
        // Clear the current progress bar and show final result
        self.clear_progress_bar()?;
        
        let completed = self.property_lines.iter().filter(|p| p.status == PropertyStatus::Completed).count();
        let failed = self.property_lines.iter().filter(|p| p.status == PropertyStatus::Failed).count();

        execute!(
            io::stdout(),
            Print("─".repeat(80)),
            Print("\n"),
            SetForegroundColor(Color::Green),
            Print(format!("✅ Scraping completed: {} successful", completed)),
            ResetColor
        )?;
        
        if failed > 0 {
            execute!(
                io::stdout(),
                SetForegroundColor(Color::Red),
                Print(format!(", {} failed", failed)),
                ResetColor
            )?;
        }
        
        execute!(
            io::stdout(),
            SetForegroundColor(Color::DarkGrey),
            Print(format!(" | DB: {} total\n", total_count)),
            ResetColor
        )?;
        
        Ok(())
    }

    fn find_property_index(&self, url: &str) -> Option<usize> {
        self.property_lines.iter().position(|p| p.url == url)
    }

    fn update_property_line(&self, index: usize) -> io::Result<()> {
        // Calculate how many lines to move back to reach the specific property line
        let mut lines_back = self.property_lines.len() - index;
        
        // If progress bar is printed, account for it (2 extra lines: separator + progress bar)
        if self.progress_bar_printed {
            lines_back += 2;
        }
        
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

        // Redraw progress bar if it was there
        if self.progress_bar_printed {
            self.print_progress_bar()?;
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

    /// Print the progress bar for the first time
    fn print_progress_bar(&self) -> io::Result<()> {
        if self.property_lines.is_empty() {
            return Ok(());
        }

        let status_line = self.create_progress_bar_text();

        // Print separator and progress bar
        execute!(
            io::stdout(),
            Print("─".repeat(80)),
            Print("\n"),
            SetForegroundColor(Color::White),
            Print(status_line),
            Print("\n"),
            ResetColor
        )?;

        Ok(())
    }

    /// Update the progress bar in place (without moving property lines)
    fn update_progress_bar_in_place(&self) -> io::Result<()> {
        if self.property_lines.is_empty() || !self.progress_bar_printed {
            return Ok(());
        }

        let status_line = self.create_progress_bar_text();

        // Move back to progress bar line and update it
        execute!(
            io::stdout(),
            MoveToPreviousLine(1), // Move back to progress bar line
            Clear(ClearType::CurrentLine),
            SetForegroundColor(Color::White),
            Print(status_line),
            Print("\n"),
            ResetColor
        )?;

        Ok(())
    }

    /// Create the progress bar text
    fn create_progress_bar_text(&self) -> String {
        let completed = self.property_lines.iter().filter(|p| p.status == PropertyStatus::Completed).count();
        let failed = self.property_lines.iter().filter(|p| p.status == PropertyStatus::Failed).count();
        let total = self.property_lines.len();
        let percentage = if total > 0 { (completed * 100) / total } else { 0 };

        // Create progress bar (30 characters wide)
        let bar_width = 30;
        let filled = (completed * bar_width) / total.max(1);
        let progress_bar = format!(
            "[{}{}]",
            "█".repeat(filled),
            "░".repeat(bar_width - filled)
        );

        if failed > 0 {
            format!(
                "Progress: {} {}/{} ({}%) | {} failed | DB: {} total",
                progress_bar, completed, total, percentage, failed, self.total_properties_in_db
            )
        } else {
            format!(
                "Progress: {} {}/{} ({}%) | DB: {} total",
                progress_bar, completed, total, percentage, self.total_properties_in_db
            )
        }
    }

    /// Clear the progress bar (used before final summary)
    fn clear_progress_bar(&self) -> io::Result<()> {
        if !self.property_lines.is_empty() {
            // Move back 2 lines (separator + progress bar)
            execute!(
                io::stdout(),
                MoveToPreviousLine(2),
                Clear(ClearType::FromCursorDown),
            )?;
        }
        Ok(())
    }
}

impl Default for ScraperTUI {
    fn default() -> Self {
        Self::new()
    }
}