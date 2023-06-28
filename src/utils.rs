//! Utilities.
use std::fmt::Debug;

pub(crate) type Data = ();
pub(crate) type Error = Box<dyn std::error::Error + Send + Sync>;
pub(crate) type Context<'a> = poise::Context<'a, Data, Error>;

/// Log errors to the console. Used for actions like sending a message where the fallback is "do nothing".
pub(crate) fn log_err<T, E: Debug>(res: Result<T, E>) {
    // turns on backtraces to find errors
    // res.as_ref().unwrap();
    match res {
        Ok(_) => {}
        Err(e) => println!("Errored: {:?}", e),
    }
}
