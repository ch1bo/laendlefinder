# LeandleFinder

A hobby project to collect historical real estate sales data from Vorarlberg and visualize it on OpenStreetMap.

## Project Components

1. **Data Scraper**: Collects and stores real estate sales data (Rust)
2. **Map Visualization**: Displays the data on OpenStreetMap (JavaScript)

## Scraper Usage

``` shell
cargo run
```

### Using Authentication with Cookies

Some websites require authentication to access their content. You can provide cookies from your browser session:

1. Visit the target website (vol.at) in your browser and log in if required
2. Open browser developer tools (F12 or right-click → Inspect)
3. Go to the "Network" tab
4. Refresh the page
5. Click on any request to the website
6. In the request headers, find the "Cookie" header
7. Copy the entire cookie string
8. Save it to a file named `cookies.txt` in the project root directory
9. Run the scraper with: `cargo run cookies.txt` (or specify a different path)

## Project Status

This project is in the "Dirt Road" phase - the initial implementation to prove the concept works.

See the [roadmap.md](roadmap.md) file for the full development plan.
