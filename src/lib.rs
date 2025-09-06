use thiserror::Error;

pub mod de;
pub mod ser;
pub mod wrapper;

pub use de::from_str;
pub use ser::to_string;

#[derive(Debug, Error)]
pub enum Error {
    #[error(transparent)]
    Ser(#[from] ser::Errors),
    #[error(transparent)]
    De(#[from] serde::de::value::Error),
}
