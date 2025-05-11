use chrono::NaiveDate;

#[derive(Debug, Clone)]
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

impl Property {
    pub fn to_csv_record(&self) -> Vec<String> {
        vec![
            self.url.clone(),
            self.price.clone(),
            self.location.clone(),
            self.property_type.clone(),
            self.date.map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default(),
            self.coordinates.map(|(lat, lng)| format!("{},{}", lat, lng)).unwrap_or_default(),
            self.address.clone().unwrap_or_default(),
            self.size_living.clone().unwrap_or_default(),
            self.description.clone().unwrap_or_default(),
        ]
    }
}
