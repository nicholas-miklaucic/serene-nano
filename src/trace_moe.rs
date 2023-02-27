//! Functionality to get the sources of GIFs in a message.

use regex::Regex;
use serenity::{builder::CreateInteractionResponseData, model::channel::Message};

pub(crate) fn trace_response<'a, 'b>(
    src: &Message,
    msg: &'a mut CreateInteractionResponseData<'b>,
) -> &'a mut CreateInteractionResponseData<'b> {
    let img_url_re = Regex::new(r"(https://tenor.com/view/[A-Za-z0-9/-]+)").unwrap();

    for mat in img_url_re.find_iter(&src.content) {
        let url = mat.as_str();
        msg.embed(|e| {
            e.title("Image source")
                .description(url)
                .url(format!("https://trace.moe/?url={}", url))
        });
    }

    msg
}
