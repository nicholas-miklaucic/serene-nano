//! API wrapper for OpenMeteo geolocation.

use reqwest;
use serde::{Deserialize, Serialize};

/// Geolocation API result. (Does not include every field.)
#[derive(Clone, PartialEq, Debug, Deserialize, Serialize)]
pub(crate) struct Location {
    /// The location name.
    pub name: String,
    /// The latitude (WGS84).
    pub latitude: f64,
    /// The longitude (WGS84).
    pub longitude: f64,
    /// The first administrative level (e.g., state in US)
    pub admin1: String,
    /// The country code.
    pub country_code: String,
    /// The timezone.
    pub timezone: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct LocationsResponse {
    /// The results.
    results: Vec<Location>,
}

pub(crate) async fn find_location(name: &str) -> Option<Location> {
    let client = reqwest::Client::new();
    let r: reqwest::Response = client
        .get("https://geocoding-api.open-meteo.com/v1/search")
        .query(&[("name", name), ("count", "1")])
        .send()
        .await
        .ok()?;

    let locs: LocationsResponse = r.json().await.ok()?;
    locs.results.into_iter().next()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_geoloc() {
        let charlotte = find_location("Charlotte").await.unwrap();
        assert_eq!(charlotte.name, "Charlotte");
        assert_eq!(charlotte.admin1, "North Carolina");
        assert_eq!(charlotte.country_code, "US");
        assert_eq!(charlotte.timezone, "America/New_York");
    }
}
