# LeandleFinder Project Roadmap: Dirt Road → Gravel Road → Highway

## Project Overview
A hobby project to collect historical real estate sales data and visualize it on OpenStreetMap, with two main components:
1. **Data Scraper**: Collects and stores real estate sales data (Rust)
2. **Map Visualization**: Displays the data on OpenStreetMap (JavaScript)

## Dirt Road Version
*Goal: Create the simplest functional version to prove the concept works*

### Scraper (Rust)
- [ ] Set up basic Rust project with reqwest, scraper, and regex crates
- [ ] Create a simple scraper for the index page (https://www.vol.at/themen/grund-und-boden)
- [ ] Implement link extraction to find individual property listings
- [ ] Create parser for individual property pages using heuristic approach:
  - Extract price from headline using regex pattern (`um\s+([\d,.]+)\s+Euro`)
  - Extract location from headline using regex pattern (`in\s+([A-Za-zÄÖÜäöüß-]+)`)
  - Extract property type from headline using regex pattern (`eine\s+([A-Za-zÄÖÜäöüß-]+)`)
- [ ] Save data to a CSV file (no database yet)
- [ ] Handle basic error cases (page not found, network issues)
- [ ] Deploy visualization component to GitHub Pages

### Visualization
- [ ] Create a simple HTML page with basic OpenStreetMap integration
- [ ] Implement basic JavaScript to read data from the CSV
- [ ] Display simple markers for each property
- [ ] Show basic info (price, date) on click
- [ ] Manual process to update data (run scraper, copy CSV, reload page)

### Success Criteria
- Scraper can extract data from at least one page
- Map shows markers in correct locations
- Basic property information is viewable

## Gravel Road Version
*Goal: Build a more robust solution with proper data storage and improved visualization*

### Scraper Improvements
- [ ] Add SQLite database integration with rusqlite
- [ ] Design proper schema for property data
- [ ] Improve regex patterns to extract additional data fields from article content
- [ ] Handle edge cases in the content structure
- [ ] Extract historical data by navigating through pagination
- [ ] Add data normalization and cleaning
- [ ] Implement geocoding to get coordinates from addresses
- [ ] Add proper rate limiting and request handling
- [ ] Add command-line parameters for configuration
- [ ] Create basic logging
- [ ] Set up automated data updates (weekly/monthly)

### Visualization Improvements
- [ ] Create a proper frontend with filtering options:
  - Date range slider
  - Price range filter
  - Simple property attribute filters
- [ ] Implement property clusters for better map readability
- [ ] Add color coding for price ranges
- [ ] Create a simple backend API to query the SQLite database
- [ ] Add basic statistics (avg price, count by area)
- [ ] Implement responsive design for mobile viewing
- [ ] Add cadastral data overlay from Atlas Vorarlberg:
  - Research WMS endpoint availability from atlas.vorarlberg.at
  - Implement WMS layer integration using Leaflet.WMS plugin
  - Add toggle for cadastral plot display
  - Implement plot number visibility on zoom
  - Add layer opacity controls

### Success Criteria
- Scraper reliably collects data from multiple pages
- Database properly stores and indexes data
- Map visualization has useful filtering options
- Basic analytics provide insight into the data

## Highway Version
*Goal: Create a polished, production-quality solution with advanced features*

### Scraper Refinements
- [ ] Implement incremental updates (only scrape new listings)
- [ ] Add advanced error recovery and retry logic
- [ ] Create automated scheduling for regular updates
- [ ] Implement proxy rotation for avoiding blocking
- [ ] Add comprehensive test suite
- [ ] Optimize performance and memory usage
- [ ] Add configuration file support
- [ ] Implement data validation and quality checks
- [ ] Consider selective LLM integration only for edge cases:
  - Unusual property descriptions
  - Incomplete or ambiguous data
  - Address normalization

### Visualization Refinements
- [ ] Add heatmap visualization option
- [ ] Implement time-series visualization to show market trends
- [ ] Create advanced analytics:
  - Price per square foot comparisons
  - Neighborhood statistics
  - Year-over-year growth rates
- [ ] Add data export functionality (CSV, JSON)
- [ ] Implement user preferences/settings storage (localStorage)
- [ ] Add advanced filtering and search capabilities
- [ ] Optimize performance for large datasets
- [ ] Polish UI/UX with improved styling and interactions

### Deployment and Automation
- [ ] Set up automated scraper execution (cron job or service)
- [ ] Implement proper database backups
- [ ] Create documentation
- [ ] Set up proper hosting if desired
- [ ] Add monitoring and alerts for scraper failures

### Success Criteria
- System runs with minimal maintenance
- Advanced visualizations provide deep insights
- UI is polished and intuitive
- Data collection is reliable and automated

## Tech Stack
- **Scraper**: Rust with reqwest, scraper, regex, and rusqlite
- **Database**: SQLite
- **Visualization**: HTML/CSS/JavaScript with Leaflet.js for OpenStreetMap
- **Hosting**: GitHub Pages for the visualization component
- **Map Services**: OpenStreetMap base layer, WMS integration for Vorarlberg cadastral data

## Learning Resources
- [Rust Book](https://doc.rust-lang.org/book/)
- [Reqwest Documentation](https://docs.rs/reqwest)
- [Scraper Crate Documentation](https://docs.rs/scraper)
- [Rusqlite Documentation](https://docs.rs/rusqlite)
- [Leaflet.js Documentation](https://leafletjs.com/reference.html)
- [OpenStreetMap Wiki](https://wiki.openstreetmap.org/wiki/Main_Page)
- [Leaflet.WMS Plugin](https://github.com/heigeo/leaflet.wms)
- [WMS Service Tutorial](https://leafletjs.com/examples/wms/wms.html)
- [Atlas Vorarlberg Documentation](https://atlas.vorarlberg.at)

## Initial Setup Tasks
- [ ] Set up version control repository on GitHub (including GitHub Pages configuration)
- [ ] Install Rust toolchain
- [ ] Set up local development environment
- [ ] Create basic project structure:
  - Rust scraper component
  - Web visualization component
- [ ] Perform initial analysis of target website structure
- [ ] Create sample regex patterns for key data extraction
