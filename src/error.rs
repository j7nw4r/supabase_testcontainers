use thiserror::Error;

#[derive(Debug, Error, Default)]
pub enum Error {
    #[default]
    Unknown
}