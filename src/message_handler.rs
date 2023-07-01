//! Message handler functionality.

use crate::typst_base::TYPST_BASE;
use crate::utils::log_err;
use crate::{translate, typst_main};

use lingua::Language;
use serenity::builder::EditMessage;

use crate::message_filter::{get_message_type, MessageType};

use serenity::collector::EventCollectorBuilder;
use serenity::futures::StreamExt;

use serenity::model::channel::AttachmentType;

use serenity::model::prelude::{AttachmentId, Event, EventType};
use serenity::utils::MessageBuilder;

use std::time::Duration;

use crate::typst_main::catch_typst_message;
use crate::utils::Error;

use serenity::{self, model::channel::Message, prelude::*};

/// Appropriately deals wih the different potential message types.
pub(crate) async fn handle_message(_ctx: &Context, _new_message: &Message) -> Result<(), Error> {
    match get_message_type(_new_message, _ctx).await {
        MessageType::Normal | MessageType::BotMessage => {}
        MessageType::Thank => {
            dbg!(&_new_message.content);
            crate::rep::thank(_ctx, _new_message).await?;
        }
        MessageType::GoodNano => {
            _new_message
                .reply(
                    &_ctx,
                    MessageBuilder::new()
                        .push("https://i.imgur.com/bgiANhm.gif")
                        .build(),
                )
                .await?;
        }
        MessageType::BadNano => {
            _new_message
                .reply(
                    &_ctx,
                    MessageBuilder::new()
                        .push("https://c.tenor.com/8QjR5hC91b0AAAAC/nichijou-nano.gif")
                        .build(),
                )
                .await?;
        }
        MessageType::Translate(other_language) => {
            let res = translate::translate_content(
                &_new_message.content,
                Some(other_language),
                Language::English,
            )
            .await?;
            _new_message.reply(&_ctx, res).await?;
        }
        MessageType::Typst(typst_src) => {
            let mut typst_reply = _new_message
                .channel_id
                .send_message(&_ctx.http, |m| {
                    match crate::typst_main::render(TYPST_BASE.clone(), typst_src.as_str()) {
                        Ok(im) => m.add_file(AttachmentType::Bytes {
                            data: im.into(),
                            filename: "Rendered.png".into(),
                        }),
                        Err(e) => m.content(format!("`n{}n`\n{}", typst_src, e)),
                    }
                })
                .await?;

            let mut prev_img_id = match typst_reply.attachments.get(0) {
                Some(img) => img.id,
                None => {
                    println!("No image!");
                    AttachmentId(0)
                }
            };

            let mut collector = EventCollectorBuilder::new(_ctx)
                .add_event_type(EventType::MessageUpdate)
                .add_message_id(_new_message.id)
                .timeout(Duration::from_secs(180))
                .build()
                .unwrap();

            while let Some(Event::MessageUpdate(e)) = collector.next().await.as_deref() {
                if let Some(new_typst_content) =
                    catch_typst_message(e.content.clone().unwrap().as_str())
                {
                    typst_reply
                        .edit(&_ctx, |m| {
                            match typst_main::render(TYPST_BASE.clone(), new_typst_content.as_str())
                            {
                                Ok(im) => m
                                    .remove_existing_attachment(prev_img_id)
                                    .content("")
                                    .attachment(AttachmentType::Bytes {
                                        data: im.into(),
                                        filename: "Rendered.png".into(),
                                    }),
                                Err(e) => m.content(format!("`n{}n`\n{}", typst_src, e)),
                            }
                        })
                        .await?;
                    prev_img_id = match typst_reply.attachments.get(0) {
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
