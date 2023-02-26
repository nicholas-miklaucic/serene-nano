//! Command to get data from OpenMeteo.
use std::{
    convert::TryInto,
    fmt::Display,
    time::{Duration, SystemTime},
};

use reqwest::RequestBuilder;
use serde::{Deserialize, Serialize};
use serenity::{
    builder::{CreateEmbed, CreateInteractionResponseData, CreateMessage},
    client::Context,
    model::interactions::application_command::ApplicationCommandInteraction,
};
use serenity_additions::menu::{MenuBuilder, Page};

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
pub(crate) enum TempUnit {
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
pub(crate) struct WeatherUnits {
    pub apparent_temperature: TempUnit,
}

/// Hourly weather data.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct HourlyWeatherData {
    /// The times of each forecast, in UNIX time.
    pub time: Vec<u64>,
    /// The weather codes, in WMO format.
    pub weathercode: Vec<usize>,
    /// The apparent temperature.
    pub apparent_temperature: Vec<f64>,
    /// The maximum precipitation probability, as a percent.
    pub precipitation_probability: Vec<f64>,
}

impl HourlyWeatherData {
    /// Gets the length of the data.
    pub fn len(&self) -> usize {
        self.time.len()
    }
}

/// Weather response data for a hourly weather data request.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub(crate) struct WeatherResponse {
    /// The latitude.
    pub latitude: f64,
    /// The longitude.
    pub longitude: f64,
    /// The offset in seconds.
    pub utc_offset_seconds: i64,
    /// The units used.
    pub hourly_units: WeatherUnits,
    /// The hourly weather data itself.
    pub hourly: HourlyWeatherData,
}

pub(crate) async fn get_weather_forecast_from_loc(
    loc: &Location,
    units: &UnitSystem,
) -> Option<WeatherResponse> {
    let client = reqwest::Client::new();
    let hourly_info = vec![
        "weathercode",
        "apparent_temperature",
        "precipitation_probability",
    ];
    // comma-separated lists don't work in reqwests using query()
    let hourly_info_str = format!("hourly={}", hourly_info.join(","));
    let r = units
        .query_args(
            client
                .get(format!(
                    "https://api.open-meteo.com/v1/forecast?{}",
                    hourly_info_str
                ))
                .query(&[("latitude", loc.latitude), ("longitude", loc.longitude)])
                .query(&[("timeformat", "unixtime"), ("timezone", "auto")]),
        )
        .send()
        .await;

    let locs: Option<WeatherResponse> = dbg!(r.ok()?.json().await).ok();
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
        _ => "unknown",                              // Unknown, including "fog"
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
pub(crate) async fn weather_forecast_msg<'a, 'b>(
    loc: &Location,
    forecast: &WeatherResponse,
    command: &ApplicationCommandInteraction,
    ctx: &Context,
) {
    let temp_code = forecast.hourly_units.apparent_temperature;
    let hourly = &forecast.hourly;

    let menu = MenuBuilder::new_paginator().timeout(Duration::from_secs(120));
    let mut pages = vec![];
    const NUM_PAGES: usize = 10;

    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    for i in 0..hourly.len() {
        if (hourly.time[i] as i64 + forecast.utc_offset_seconds)
            .try_into()
            .unwrap_or(0)
            < now
        {
            // this is for the past, don't include
            continue;
        }
        let page_num = pages.len();
        let mut msg_page = CreateMessage::default();
        msg_page.embed(|e| {
            e.image(get_weather_icon_url(hourly.weathercode[0]))
                .title(format!(
                    "Forecast for {}, {}, {}",
                    loc.name, loc.admin1, loc.country_code
                ))
                .description(format!("T+{}", page_num))
                .url(format!(
                    "https://merrysky.net/forecast/{},{}",
                    loc.latitude, loc.longitude
                ))
                .color((229, 100, 255))
                .field(
                    "Felt Temperature",
                    format!("{} {}", hourly.apparent_temperature[i], temp_code),
                    false,
                )
                .field(
                    "Precipitation Chance",
                    format!("{}%", hourly.precipitation_probability[i]),
                    false,
                )
                .footer(|f| f.text("Courtesy of OpenMeteo"))
        });
        pages.push(Page::new_static(msg_page));
        if pages.len() >= NUM_PAGES {
            // added enough hours
            break;
        }
    }
    match menu
        .add_pages(pages)
        .show_help()
        .build(ctx, command.channel_id)
        .await
    {
        Ok(_) => {}
        Err(_) => {}
    }
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
