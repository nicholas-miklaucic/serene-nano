//! Command to get data from OpenWeatherMap.

use std::env;

use openweathermap::blocking::weather;
use openweathermap::{self, CurrentWeather};
use serenity::builder::{CreateEmbed, CreateInteractionResponseData, CreateMessage};
use serenity::client::Context;
use serenity::model::channel::Message;

const OWM_ICON_URL: &'static str = "http://openweathermap.org/img/wn/{code}@2x.png";

/// Gets the weather data from OWM as a Result.
fn weather_data(location: &str, units: &str) -> Result<CurrentWeather, String> {
    let api_key = env::var("OWM_KEY").unwrap_or("bad".to_string());
    weather(location, units, "en", api_key.as_str())
}

/// Given a location and units, either prints an error message or an embed
/// containing the weather information.
pub(crate) fn weather_msg<'a>(
    location: &str,
    units: &str,
    msg: &'a mut CreateInteractionResponseData,
) -> &'a mut CreateInteractionResponseData {
    match weather_data(location, units) {
        Ok(data) => msg.create_embed(|e| {
            if data.weather.is_empty() {
                e.field(
                    "Error",
                    "Weather data returned incorrectly: please report to Pollards",
                    false,
                )
            } else {
                let w = &data.weather[0];
                let unit = match units {
                    "imperial" => "F",
                    "metric" => "C",
                    "standard" => "K",
                    _ => "N/A",
                };
                e.image(OWM_ICON_URL.replace("{code}", &w.icon));
                e.field("City", data.name, false);
                e.field("Country", data.sys.country, false);
                e.field("Conditions", &w.description, true);
                e.field(
                    "Feels Like",
                    format!("{:.0} °{}", data.main.feels_like, unit),
                    true,
                );
                e.footer(|f| f.text("Courtesy of OpenWeatherMap"))
            }
        }),
        Err(err_msg) => {
            if err_msg == "404 Not Found" {
                msg.content("City could not be found.")
            } else {
                msg.content(err_msg)
            }
        }
    }
}
