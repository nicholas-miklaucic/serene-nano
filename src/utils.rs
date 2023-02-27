//! Utilities.
use std::fmt::Debug;

/// Log errors to the console. Used for actions like sending a message where the fallback is "do nothing".
pub(crate) fn log_err<T, E: Debug>(res: Result<T, E>) {
    match res {
        Ok(_) => {}
        Err(e) => println!("Errored: {:?}", e),
    }
}
