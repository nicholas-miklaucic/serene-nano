//! Functionality to get the sources of GIFs in a message.


use regex::Regex;
use serenity::{model::channel::Message};

use crate::utils::{Context, Error};

#[poise::command(context_menu_command = "Anime Sauce", slash_command)]
pub(crate) async fn find_anime_source(
    ctx: Context<'_>,
    #[description = "Message (link or ID)"] msg: Message,
) -> Result<(), Error> {
    let img_url_re = Regex::new(r"(https://tenor.com/view/[A-Za-z0-9/-]+)|(https?://(?:[a-z0-9\-]+\.)+[a-z]{2,6}(?:/[^/#?]+)+\.(?:jpe?g|gif|png))").unwrap();

    ctx.send(|mut m| {
        let mut num_imgs = 0;
        for mat in img_url_re.find_iter(&msg.content) {
            num_imgs += 1;
            let url = mat.as_str();
            m = m.embed(|e| {
                e.title("Anime Source Results")
                    .image(url)
                    .url(format!("https://trace.moe/?url={}", url))
            });
        }

        if num_imgs == 0 {
            m.content("No images found, sorry!")
        } else {
            m
        }
    })
    .await?;

    Ok(())
}
