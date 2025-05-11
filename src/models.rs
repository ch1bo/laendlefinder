use chrono::NaiveDate;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::ser::SerializeStruct;

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

// Custom serialization for Property to handle the coordinates tuple
impl Serialize for Property {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("Property", 10)?;
        state.serialize_field("url", &self.url)?;
        state.serialize_field("price", &self.price)?;
        state.serialize_field("location", &self.location)?;
        state.serialize_field("property_type", &self.property_type)?;
        state.serialize_field("date", &self.date)?;
        state.serialize_field("description", &self.description)?;
        
        // Serialize coordinates as a single string field
        let coords_str = match &self.coordinates {
            Some((lat, lng)) => format!("{},{}", lat, lng),
            None => String::new(),
        };
        state.serialize_field("coordinates", &coords_str)?;
        
        state.serialize_field("address", &self.address)?;
        state.serialize_field("size_living", &self.size_living)?;
        state.end()
    }
}

// Custom deserialization for Property to handle the coordinates string
impl<'de> Deserialize<'de> for Property {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct PropertyHelper {
            url: String,
            price: String,
            location: String,
            property_type: String,
            date: Option<NaiveDate>,
            description: Option<String>,
            coordinates: String,
            address: Option<String>,
            size_living: Option<String>,
        }

        let helper = PropertyHelper::deserialize(deserializer)?;
        
        // Parse coordinates from string
        let coordinates = if helper.coordinates.is_empty() {
            None
        } else {
            let parts: Vec<&str> = helper.coordinates.split(',').collect();
            if parts.len() == 2 {
                match (parts[0].parse::<f64>(), parts[1].parse::<f64>()) {
                    (Ok(lat), Ok(lng)) => Some((lat, lng)),
                    _ => None
                }
            } else {
                None
            }
        };

        Ok(Property {
            url: helper.url,
            price: helper.price,
            location: helper.location,
            property_type: helper.property_type,
            date: helper.date,
            description: helper.description,
            coordinates,
            address: helper.address,
            size_living: helper.size_living,
        })
    }
}
