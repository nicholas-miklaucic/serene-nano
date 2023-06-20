//! Trait that responds to commands.

use serenity::{
    builder::{CreateApplicationCommandOption, CreateInteractionResponseData},
    client::Context,
    model::application::interaction::{
        application_command::ApplicationCommandInteraction, Interaction,
    },
};
use typst::diag::StrResult;

pub(crate) trait CommandResponder: Sync + Send {
    /// Responds to a command.
    fn response<'a, 'b>(
        &self,
        command: &ApplicationCommandInteraction,
        ctx: &Context,
        msg: &'a mut CreateInteractionResponseData<'b>,
    ) -> &'a mut CreateInteractionResponseData<'b>;
}

/// Simply adds a string to the message.
#[derive(Debug, Clone, Default)]
pub(crate) struct StringContent {
    content: String,
}

impl StringContent {
    pub fn new<D: ToString>(content: D) -> Self {
        Self {
            content: content.to_string(),
        }
    }
}

impl CommandResponder for StringContent {
    fn response<'a, 'b>(
        &self,
        _command: &ApplicationCommandInteraction,
        _ctx: &Context,
        msg: &'a mut CreateInteractionResponseData<'b>,
    ) -> &'a mut CreateInteractionResponseData<'b> {
        msg.content(&self.content)
    }
}

pub trait Command: Sync + Send{
    fn name(&self) -> &str;
    fn description(&self) -> &str;
    // fn get_options(&self, option: &mut CreateApplicationCommandOption)-> Vec<&mut CreateApplicationCommandOption>;
    fn interaction<'a, 'b>(
        &self,
        ctx: &Context,
        command: &ApplicationCommandInteraction,
        msg: &'a mut CreateInteractionResponseData<'b>
    ) ->&'a mut CreateInteractionResponseData<'b>;

    fn options(
        &self,
    ) -> Vec<fn(&mut CreateApplicationCommandOption) -> &mut CreateApplicationCommandOption>;

    fn option_names(&self) -> Vec<String> {
        //TODO: A smart way of getting the option names
        todo!()
    }
}

// /// Uses the weather forecast information to add an embed.
// #[derive(Debug, Clone)]
// pub(crate) struct WeatherEmbed {
//     location: Option<Location>,
//     forecast: Option<WeatherResponse>,
// }

// impl WeatherEmbed {
//     pub async fn new(name: &str, units: UnitSystem) -> Self {
//         let location = find_location(name).await;
//         let forecast = match &location {
//             Some(l) => get_weather_forecast_from_loc(&l, &units).await,
//             None => None,
//         };
//         Self { location, forecast }
//     }
// }

// impl CommandResponder for WeatherEmbed {
//     fn response<'a, 'b>(
//         &self,
//         command: &ApplicationCommandInteraction,
//         ctx: &Context,
//         msg: &'a mut CreateInteractionResponseData<'b>,
//     ) -> &'a mut CreateInteractionResponseData<'b> {
//         match self.location.as_ref().zip(self.forecast.as_ref()) {
//             Some((l, f)) => weather_forecast_msg(l, f, command, ctx, msg),
//             None => msg.content("Weather could not be found :("),
//         }
//     }
// }
