//! Message handler functionality.

use crate::translate;

use lingua::Language;

use crate::message_filter::{get_message_type, MessageType};

use serenity::collector::EventCollectorBuilder;
use serenity::futures::StreamExt;

use serenity::model::channel::AttachmentType;

use serenity::model::prelude::{AttachmentId, Event, EventType};
use serenity::utils::MessageBuilder;

use std::time::Duration;

use crate::math_markup::{catch_typst_message, render_math};
use crate::utils::Error;

use serenity::{self, model::channel::Message, prelude::*};

/// Appropriately deals wih the different potential message types.
pub(crate) async fn handle_message(ctx: &Context, new_message: &Message) -> Result<(), Error> {
    match get_message_type(new_message, ctx).await {
        MessageType::Normal | MessageType::BotMessage => {}
        MessageType::Thank => {
            dbg!(&new_message.content);
            crate::rep::thank(ctx, new_message).await?;
        }
        MessageType::GoodNano => {
            new_message
                .reply(
                    &ctx,
                    MessageBuilder::new()
                        .push("https://i.imgur.com/bgiANhm.gif")
                        .build(),
                )
                .await?;
        }
        MessageType::BadNano => {
            new_message
                .reply(
                    &ctx,
                    MessageBuilder::new()
                        .push("https://c.tenor.com/8QjR5hC91b0AAAAC/nichijou-nano.gif")
                        .build(),
                )
                .await?;
        }
        MessageType::Translate(other_language) => {
            let (_src, res) = translate::translate_content(
                &new_message.content,
                Some(other_language),
                Language::English,
            )
            .await?;

            if edit_distance::edit_distance(&new_message.content, &res) >= 6 {
                new_message.reply(&ctx, res).await?;
            } else {
                println!(
                    "Tried to translate {:?}\n{:?} -> English, but was too close to original:\n{:?}",
                    &new_message.content,
                    other_language,
                    &res
                )
            }
        }
        MessageType::Typst(typst_src) => {
            let res = render_math(typst_src.as_str(), &new_message.author).await;
            let mut typst_reply = new_message
                .channel_id
                .send_message(&ctx.http, |m| match res {
                    Ok(im) => m.add_file(AttachmentType::Bytes {
                        data: im.into(),
                        filename: "Rendered.png".into(),
                    }),
                    Err(e) => {
                        println!("`n{}n`\n{}", typst_src, e);
                        m.content(format!("`n{}n`\n{}", typst_src, e))
                    }
                })
                .await?;

            let mut prev_img_id = match typst_reply.attachments.first() {
                Some(img) => img.id,
                None => {
                    println!("No image!");
                    AttachmentId(0)
                }
            };

            let mut collector = EventCollectorBuilder::new(ctx)
                .add_event_type(EventType::MessageUpdate)
                .add_message_id(new_message.id)
                .timeout(Duration::from_secs(180))
                .build()
                .unwrap();

            while let Some(Event::MessageUpdate(e)) = collector.next().await.as_deref() {
                if let Some(new_typst_content) =
                    catch_typst_message(e.content.clone().unwrap().as_str())
                {
                    let res = render_math(new_typst_content.as_str(), &new_message.author).await;
                    typst_reply
                        .edit(&ctx, |m| match res {
                            Ok(im) => m
                                .remove_existing_attachment(prev_img_id)
                                .content("")
                                .attachment(AttachmentType::Bytes {
                                    data: im.into(),
                                    filename: "Rendered.png".into(),
                                }),
                            Err(e) => m
                                .remove_existing_attachment(prev_img_id)
                                .content(format!("`n{}n`\n{}", new_typst_content, e)),
                        })
                        .await?;
                    prev_img_id = match typst_reply.attachments.first() {
                        Some(img) => img.id,
                        None => {
                            println!("No image!");
                            AttachmentId(0)
                        }
                    };
                }
            }
        }
    };
    Ok(())
}
