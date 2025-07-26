use chrono::NaiveDate;
use serde::{Serialize, Deserialize, Serializer, Deserializer};
use serde::ser::SerializeStruct;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ListingType {
    Available,
    Sold,
}

impl fmt::Display for ListingType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ListingType::Available => write!(f, "available"),
            ListingType::Sold => write!(f, "sold"),
        }
    }
}

impl Serialize for ListingType {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ListingType {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        match s.as_str() {
            "available" => Ok(ListingType::Available),
            "sold" => Ok(ListingType::Sold),
            _ => Err(serde::de::Error::custom(format!("Invalid listing type: {}", s))),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Property {
    pub url: String,
    pub price: String,
    pub location: String,
    pub property_type: String,
    pub listing_type: ListingType,
    pub date: Option<NaiveDate>,
    pub coordinates: Option<(f64, f64)>,
    pub address: Option<String>,
    pub size_living: Option<String>,
    pub description: Option<String>,
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
        state.serialize_field("listing_type", &self.listing_type)?;
        state.serialize_field("date", &self.date)?;
        
        // Serialize coordinates as a single string field
        let coords_str = match &self.coordinates {
            Some((lat, lng)) => format!("{},{}", lat, lng),
            None => String::new(),
        };
        state.serialize_field("coordinates", &coords_str)?;
        
        state.serialize_field("address", &self.address)?;
        state.serialize_field("size_living", &self.size_living)?;
        
        // Description moved to the last field
        state.serialize_field("description", &self.description)?;
        
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
            listing_type: ListingType,
            date: Option<NaiveDate>,
            coordinates: String,
            address: Option<String>,
            size_living: Option<String>,
            description: Option<String>,
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
            listing_type: helper.listing_type,
            date: helper.date,
            coordinates,
            address: helper.address,
            size_living: helper.size_living,
            description: helper.description,
        })
    }
}
