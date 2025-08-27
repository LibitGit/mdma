use std::fmt;

pub mod jwt;

#[derive(Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum AuthError {
    WrongCredentials,
    MissingCredentials,
    TokenCreation,
    InvalidToken,
    TokenFraudulent,
    MissingConnection,
    MongoDbError,
    MessagingError,
}

impl fmt::Display for AuthError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for AuthError {}
