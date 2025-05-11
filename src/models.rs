#[derive(Debug, Clone)]
pub struct Property {
    pub url: String,
    pub price: f64,
    pub location: String,
    pub property_type: String,
    pub date: Option<String>,
    pub description: Option<String>,
}
