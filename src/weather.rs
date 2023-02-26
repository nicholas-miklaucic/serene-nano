//! Command to get data from OpenMeteo.
use std::fmt::Display;

use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use serenity::{
    builder::{CreateEmbed, CreateInteractionResponseData},
    client::Context,
};

use crate::geolocation::{find_location, Location};

/// Groups of units for the weather.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
pub(crate) enum UnitSystem {
    /// The US imperial system.
    Imperial,
    /// The metric system.
    Metric,
}

impl UnitSystem {
    /// Adds the necessary query arguments to make OpenMeteo use the system.
    pub fn query_args(&self, req: RequestBuilder) -> RequestBuilder {
        match &self {
            Self::Imperial => req.query(&[
                ("temperature_unit", "fahrenheit"),
                ("windspeed_unit", "mph"),
                ("precipitation_unit", "inch"),
            ]),
            Self::Metric => req.query(&[
                ("temperature_unit", "celsius"),
                ("windspeed_unit", "kmh"),
                ("precipitation_unit", "mm"),
            ]),
        }
    }
}

/// Temperature units.
#[derive(Debug, Copy, Clone, Hash, Eq, PartialEq, Deserialize, Serialize)]
enum TempUnit {
    /// Celsius
    #[serde(rename = "°C")]
    Celsius,

    /// Fahrenheit.
    #[serde(rename = "°F")]
    Fahrenheit,
}

impl Display for TempUnit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match &self {
                Self::Celsius => "°C",
                Self::Fahrenheit => "°F",
            }
        )
    }
}

/// The units that the results are in. (We just care about the ones that change.)
#[derive(Debug, Clone, Deserialize, Serialize)]
struct WeatherUnits {
    pub apparent_temperature_max: TempUnit,
    pub apparent_temperature_min: TempUnit,
}

/// Daily weather data.
#[derive(Debug, Clone, Deserialize, Serialize)]
struct DailyWeatherData {
    /// The times of each forecast, in ISO string format.
    pub time: Vec<String>,
    /// The weather codes, in WMO format.
    pub weathercode: Vec<usize>,
    /// The maximum apparent temperature.
    pub apparent_temperature_max: Vec<f64>,
    /// The minimum apparent temperature.
    pub apparent_temperature_min: Vec<f64>,
    /// The maximum precipitation probability, as a percent.
    pub precipitation_probability_max: Vec<f64>,
}

/// Weather response data for a daily weather data request.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct WeatherResponse {
    /// The latitude.
    pub latitude: f64,
    /// The longitude.
    pub longitude: f64,
    /// The units used.
    pub daily_units: WeatherUnits,
    /// The daily weather data itself.
    pub daily: DailyWeatherData,
}

pub(crate) async fn get_weather_forecast_from_loc(
    loc: &Location,
    units: &UnitSystem,
) -> Option<WeatherResponse> {
    let client = reqwest::Client::new();
    let daily_info = vec![
        "weathercode",
        "apparent_temperature_max",
        "apparent_temperature_min",
        "precipitation_probability_max",
    ];
    // comma-separated lists don't work in reqwests using query()
    let daily_info_str = format!("daily={}", daily_info.join(","));
    let r: reqwest::Response = units
        .query_args(
            client
                .get(format!(
                    "https://api.open-meteo.com/v1/forecast?{}",
                    daily_info_str
                ))
                .query(&[("latitude", loc.latitude), ("longitude", loc.longitude)])
                .query(&[("timezone", &loc.timezone)]),
        )
        .send()
        .await
        .ok()?;

    let locs: Option<WeatherResponse> = dbg!(r).json().await.ok();
    locs
}

/// Gets the appropriate weather icon URL for a given WMO weather code.
fn get_weather_icon_url(wmo_code: usize) -> String {
    let icon_name = match wmo_code {
        0 => "clear",                                // Clear sky
        1 => "mostlysunny",                          // Mainly clear
        2 => "partlycloudy",                         // Partly cloudy
        3 => "cloudy",                               // Overcast
        51 | 53 | 55 | 80 | 81 | 82 => "chancerain", // Drizzles and rain showers
        56 | 57 => "chancesleet",                    // Freezing drizzles
        61 | 63 | 65 => "rain",                      // Rain and rain showers
        71 | 73 | 75 | 77 => "snow",                 // Snow and "snow grains"
        85 | 86 => "chancesnow",                     // Snow showers
        95 | 96 | 99 => "tstorms",                   // Thunderstorms
        _ => "unknown",                              // Unknown
    };

    format!("https://cdn.jsdelivr.net/gh/manifestinteractive/weather-underground-icons/dist/icons/white/png/128x128/{}.png", icon_name)
}

/// Gets the weather forecast given a name and units.
pub(crate) async fn get_weather_forecast_from_name(
    name: &str,
    units: &UnitSystem,
) -> Option<WeatherResponse> {
    let loc: Location = find_location(name).await?;
    get_weather_forecast_from_loc(&loc, units).await
}

/// Reports the weather forecast at the given name in the given units.
pub(crate) fn weather_forecast_msg<'a, 'b>(
    loc: Location,
    forecast: WeatherResponse,
    msg: &'a mut CreateInteractionResponseData<'b>,
) -> &'a mut CreateInteractionResponseData<'b> {
    let temp_code = forecast.daily_units.apparent_temperature_max;
    let daily = forecast.daily;
    msg.embed(|e| {
        e.image(get_weather_icon_url(daily.weathercode[0]))
            .color((229, 100, 255))
            .field(
                "Low",
                format!("{} {}", daily.apparent_temperature_min[0], temp_code),
                true,
            )
            .field(
                "High",
                format!("{} {}", daily.apparent_temperature_max[0], temp_code),
                true,
            )
            .field(
                "Precipitation Chance",
                format!("{}%", daily.precipitation_probability_max[0]),
                true,
            )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_weather_loc() {
        let boston = Location {
            name: "Boston".to_string(),
            latitude: 42.35843,
            longitude: -71.05977,
            admin1: "Massachusetts".to_string(),
            country_code: "US".to_string(),
            timezone: "America/New_York".to_string(),
        };

        let forecast = get_weather_forecast_from_loc(&boston, &UnitSystem::Metric)
            .await
            .unwrap();
        dbg!(&forecast);
    }

    #[tokio::test]
    async fn test_weather_name() {
        let forecast = get_weather_forecast_from_name("Georgetown", &UnitSystem::Metric)
            .await
            .unwrap();
        dbg!(&forecast);
    }
}
