use chrono::NaiveDate;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Property {
    pub url: String,
    pub price: String,
    pub location: String,
    pub property_type: String,
    pub date: Option<NaiveDate>,
    pub description: Option<String>,
    pub coordinates: Option<(f64, f64)>,
    pub address: Option<String>,
    pub size_living: Option<String>,
}
