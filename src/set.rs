//! Allows user-defined and editable sets of strings to be stored by Nano.

extern crate redis;
use crate::config::REDIS_URL;
use redis::{Commands, Connection, RedisResult};
use serenity::{
    builder::CreateInteractionResponseData,
    model::{
        application::interaction::application_command::{
            ApplicationCommandInteraction, CommandDataOptionValue,
        },
        prelude::{interaction::application_command::CommandDataOption, User},
    },
};

/// Gets the name of the Redis key corresponding to the given user and name.
fn key_name(user: &User, name: &str) -> String {
    format!("{}#{}:{}", user.name, user.discriminator, name)
}

fn get_list(user: &User, name: &str, con: &mut Connection) -> RedisResult<Vec<String>> {
    let key = key_name(user, name);
    con.smembers(key)
}

fn add_elements(
    user: &User,
    name: &str,
    elements: Vec<String>,
    con: &mut Connection,
) -> RedisResult<usize> {
    let key = key_name(user, name);
    con.sadd(key, elements)
}

fn rem_elements(
    user: &User,
    name: &str,
    elements: Vec<String>,
    con: &mut Connection,
) -> RedisResult<usize> {
    let key = key_name(user, name);
    con.srem(key, elements)
}

pub(crate) fn add_elements_command<'a, 'b>(
    command: &ApplicationCommandInteraction,
    msg: &'a mut CreateInteractionResponseData<'b>,
) -> (&'a mut CreateInteractionResponseData<'b>, RedisResult<()>) {
    let client_res = redis::Client::open(REDIS_URL);
    let mut client;
    match client_res {
        Ok(client_val) => {
            client = client_val;
        }
        Err(e) => return (msg, Err(e)),
    }
    let mut con;
    let mut con_res = client.get_connection();
    match con_res {
        Ok(con_val) => {
            con = con_val;
        }
        Err(e) => return (msg, Err(e)),
    }
    let mut name = None;
    let mut elements: Vec<String> = vec![];
    for opt in &command.data.options {
        if &opt.name == "list_name" {
            if let Some(serde_json::Value::String(val)) = &opt.value {
                name = Some(val);
            }
        } else if opt.name.starts_with("element") {
            match &opt.value {
                Some(serde_json::Value::String(el)) => elements.push(el.to_string()),
                _ => {}
            };
        }
    }
    if let Some(n) = name {
        let num_added_res = add_elements(&command.user, n.as_str(), elements, &mut con);
        match num_added_res {
            Ok(num_added) => (
                msg.content(format!("Successfully added {} elements", num_added)),
                Ok(()),
            ),
            Err(e) => (msg, Err(e)),
        }
    } else {
        (msg.content("Couldn't find name"), Ok(()))
    }
}

pub(crate) fn rem_elements_command<'a, 'b>(
    command: &ApplicationCommandInteraction,
    msg: &'a mut CreateInteractionResponseData<'b>,
) -> (&'a mut CreateInteractionResponseData<'b>, RedisResult<()>) {
    let client_res = redis::Client::open(REDIS_URL);
    let mut client;
    match client_res {
        Ok(client_val) => {
            client = client_val;
        }
        Err(e) => return (msg, Err(e)),
    }
    let mut con;
    let mut con_res = client.get_connection();
    match con_res {
        Ok(con_val) => {
            con = con_val;
        }
        Err(e) => return (msg, Err(e)),
    }
    let mut name = None;
    let mut elements: Vec<String> = vec![];
    for opt in &command.data.options {
        if &opt.name == "list_name" {
            if let Some(serde_json::Value::String(val)) = &opt.value {
                name = Some(val);
            }
        } else if opt.name.starts_with("element") {
            match &opt.value {
                Some(serde_json::Value::String(el)) => elements.push(el.to_string()),
                _ => {}
            };
        }
    }
    if let Some(n) = name {
        let num_added_res = rem_elements(&command.user, n.as_str(), elements, &mut con);
        match num_added_res {
            Ok(num_added) => (
                msg.content(format!("Successfully removed {} elements", num_added)),
                Ok(()),
            ),
            Err(e) => (msg, Err(e)),
        }
    } else {
        (msg.content("Couldn't find name"), Ok(()))
    }
}

pub(crate) fn get_list_command<'a, 'b>(
    command: &ApplicationCommandInteraction,
    msg: &'a mut CreateInteractionResponseData<'b>,
) -> (&'a mut CreateInteractionResponseData<'b>, RedisResult<()>) {
    let client_res = redis::Client::open(REDIS_URL);
    let mut client;
    match client_res {
        Ok(client_val) => {
            client = client_val;
        }
        Err(e) => return (msg, Err(e)),
    }
    let mut con;
    let mut con_res = client.get_connection();
    match con_res {
        Ok(con_val) => {
            con = con_val;
        }
        Err(e) => return (msg, Err(e)),
    }

    let mut name = None;
    let mut elements: Vec<String> = vec![];
    let mut user = &command.user;
    for opt in &command.data.options {
        if &opt.name == "list_name" {
            if let Some(serde_json::Value::String(val)) = &opt.value {
                name = Some(val);
            }
        } else if opt.name.starts_with("element") {
            if let Some(serde_json::Value::String(el)) = &opt.value {
                elements.push(el.to_string())
            };
        } else if &opt.name == "user" {
            if let Some(CommandDataOptionValue::User(user_arg, _)) = &opt.resolved {
                user = &user_arg;
            }
        }
    }
    match name {
        Some(n) => {
            let elements_res = get_list(user, n.as_str(), &mut con);
            match elements_res {
                Ok(elements) => {
                    let mut lines = vec![format!("{} values:", n)];
                    for el in elements {
                        lines.push(format!("- {}", el));
                    }
                    (msg.content(lines.join("\n")), Ok(()))
                }
                Err(e) => (msg, Err(e)),
            }
        }
        None => (msg.content("An error occured: no name found"), Ok(())),
    }
}
