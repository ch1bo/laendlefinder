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
    visible_lines: usize,
    visible_start: usize,
    visible_end: usize,
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
            visible_lines: 0,
            visible_start: 0,
            visible_end: 0,
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

        self.property_lines.push(property_state);
        let new_index = self.property_lines.len() - 1;
        
        // Update visible range if this is the first property or if we're still in the initial window
        if self.visible_end == 0 || new_index < 15 {
            self.visible_end = (new_index + 1).min(15);
        }
        
        // Only print if this property should be visible in our current window
        if new_index < self.visible_end {
            execute!(
                io::stdout(),
                SetForegroundColor(Color::DarkGrey),
                Print(format!("  ⏳ {}\n", Self::truncate_url(&url))),
                ResetColor
            )?;
            self.visible_lines += 1;
        }
        
        Ok(())
    }

    /// Print initial progress bar (call this after all properties are added)
    pub fn show_initial_progress_bar(&mut self) -> io::Result<()> {
        if !self.progress_bar_printed && !self.property_lines.is_empty() {
            // Set the initial visible window
            self.visible_start = 0;
            self.visible_end = 15.min(self.property_lines.len());
            
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
            
            // Only slide window if we're at the boundary (last 2 visible items)
            if index >= self.visible_end.saturating_sub(2) && self.visible_end < self.property_lines.len() {
                self.slide_window_forward()?;
            } else if index >= self.visible_start && index < self.visible_end {
                // Just update the line in place if it's already visible
                self.update_single_line(index)?;
            } else {
                // Property is outside visible range, need to slide to show it
                self.slide_window_to_show(index)?;
            }
        }
        Ok(())
    }

    /// Update the activity marker for the currently active property
    pub fn update_activity(&mut self) -> io::Result<()> {
        if let Some(index) = self.current_property_index {
            if self.property_lines[index].status == PropertyStatus::InProgress {
                if index >= self.visible_start && index < self.visible_end {
                    self.update_single_line(index)?;
                }
            }
        }
        Ok(())
    }

    /// Mark a property as completed (green)
    pub fn complete_property(&mut self, url: &str) -> io::Result<()> {
        if let Some(index) = self.find_property_index(url) {
            self.property_lines[index].status = PropertyStatus::Completed;
            if Some(index) == self.current_property_index {
                self.current_property_index = None;
            }
            
            // Just update the line in place if it's visible
            if index >= self.visible_start && index < self.visible_end {
                self.update_single_line(index)?;
            }
        }
        Ok(())
    }

    /// Mark a property as failed (red)
    pub fn fail_property(&mut self, url: &str) -> io::Result<()> {
        if let Some(index) = self.find_property_index(url) {
            self.property_lines[index].status = PropertyStatus::Failed;
            if Some(index) == self.current_property_index {
                self.current_property_index = None;
            }
            
            // Just update the line in place if it's visible
            if index >= self.visible_start && index < self.visible_end {
                self.update_single_line(index)?;
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

    /// Show failure report with URLs and reasons
    pub fn show_failure_report(&self, failed_urls: &[(String, String)]) -> io::Result<()> {
        if !failed_urls.is_empty() {
            execute!(
                io::stdout(),
                Print("\n"),
                SetForegroundColor(Color::Red),
                Print(format!("❌ Failure Report ({} failed URLs):\n", failed_urls.len())),
                ResetColor
            )?;

            for (url, reason) in failed_urls {
                execute!(
                    io::stdout(),
                    SetForegroundColor(Color::Red),
                    Print("  • "),
                    ResetColor,
                    SetForegroundColor(Color::White),
                    Print(format!("{}\n", url)),
                    ResetColor,
                    SetForegroundColor(Color::DarkGrey),
                    Print(format!("    Reason: {}\n", reason)),
                    ResetColor
                )?;
            }
        }
        Ok(())
    }

    fn find_property_index(&self, url: &str) -> Option<usize> {
        self.property_lines.iter().position(|p| p.url == url)
    }

    /// Update a single line in place without redrawing the entire window
    fn update_single_line(&mut self, _index: usize) -> io::Result<()> {
        // For simplicity, just redraw the entire window for now
        // This is still less janky than redrawing on every property change
        self.redraw_sliding_window()?;
        Ok(())
    }

    /// Slide the window forward by a few positions
    fn slide_window_forward(&mut self) -> io::Result<()> {
        // Calculate new window that shows last 3 completed + current + remaining pending (up to 15 total)
        if let Some(current_idx) = self.current_property_index {
            // Find the number of completed properties before current
            let completed_before = self.property_lines[..current_idx]
                .iter()
                .filter(|p| matches!(p.status, PropertyStatus::Completed | PropertyStatus::Failed))
                .count();
            
            // Start from 3 completed properties back, or beginning if less than 3
            let new_start = if completed_before >= 3 {
                // Find the index of the 3rd completed property before current
                let mut completed_count = 0;
                let mut start_idx = current_idx;
                for i in (0..current_idx).rev() {
                    if matches!(self.property_lines[i].status, PropertyStatus::Completed | PropertyStatus::Failed) {
                        completed_count += 1;
                        if completed_count == 3 {
                            start_idx = i;
                            break;
                        }
                    }
                }
                start_idx
            } else {
                0 // Show from beginning if we don't have 3 completed yet
            };
            
            let new_end = (new_start + 15).min(self.property_lines.len());
            
            if new_start != self.visible_start || new_end != self.visible_end {
                self.visible_start = new_start;
                self.visible_end = new_end;
                self.redraw_sliding_window()?;
            }
        }
        Ok(())
    }

    /// Slide the window to show a specific property
    fn slide_window_to_show(&mut self, index: usize) -> io::Result<()> {
        let new_start = index.saturating_sub(7); // Show more context with 15 total lines
        let new_end = (new_start + 15).min(self.property_lines.len());
        
        self.visible_start = new_start;
        self.visible_end = new_end;
        self.redraw_sliding_window()?;
        Ok(())
    }

    /// Clear visible property lines and redraw the sliding window
    fn redraw_sliding_window(&mut self) -> io::Result<()> {
        // Calculate how many lines to clear (visible property lines + progress bar if present)
        let lines_to_clear = self.visible_lines + if self.progress_bar_printed { 2 } else { 0 };
        
        if lines_to_clear > 0 {
            // Move back and clear all visible lines
            execute!(
                io::stdout(),
                MoveToPreviousLine(lines_to_clear as u16),
                Clear(ClearType::FromCursorDown),
            )?;
        }

        // Reset visible lines counter
        self.visible_lines = 0;

        // Redraw visible properties
        for i in self.visible_start..self.visible_end {
            self.draw_property_line(&self.property_lines[i])?;
            self.visible_lines += 1;
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
        if !self.property_lines.is_empty() && self.progress_bar_printed {
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