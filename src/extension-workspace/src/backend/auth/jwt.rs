use std::result::Result;

use jsonwebtoken::{
    Algorithm, DecodingKey, EncodingKey, Header, TokenData, Validation, decode, encode,
    get_current_timestamp,
};
use serde::{Deserialize, Serialize};
use uuid::{Uuid, fmt::Simple};

use crate::prelude::*;

// Cookie configuration
const ACCESS_TOKEN_DURATION: u64 = 60 * 15; // 15 minutes in seconds
const REFRESH_TOKEN_DURATION: u64 = 60 * 60 * 24 * 30; // 30 days in seconds

const VERIFIED_ROLE_ID: u64 = to_u64(env!("VERIFIED_ROLE_ID"));
const PREMIUM_ROLE_ID: u64 = to_u64(env!("PREMIUM_ROLE_ID"));
const BOOSTER_ROLE_ID: u64 = to_u64(env!("BOOSTER_ROLE_ID"));
const TEST_ROLE_ID: u64 = to_u64(env!("TEST_ROLE_ID"));
const DEV_ROLE_ID: u64 = to_u64(env!("DEV_ROLE_ID"));
const ANTYDUCH_ROLE_ID: u64 = to_u64(env!("ANTYDUCH_ROLE_ID"));

const fn to_u64(s: &str) -> u64 {
    let mut res = 0;
    let mut i = 0;
    while i < s.len() {
        let b = s.as_bytes()[i];
        res = 10 * res + (b - b'0') as u64;
        i += 1;
    }
    res
}

/// 0 - no verified role || no verified email
/// 1 - verified role && verified email
/// 2 - premium role || booster role
/// 3 - tester role
/// 4 - antyduch | dev role
#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq)]
pub struct AccessLevel(u8);

impl AccessLevel {
    const ANTYDUCH: u8 = 4;
    // const DEV: u8 = 4;

    /// Users without a verified email can only use the addon manager if they
    /// buy premium.
    pub fn new(email_verified: bool, roles: &[serenity::RoleId]) -> Self {
        let mut access_lvl = 0;

        roles.iter().for_each(|role| match role.get() {
            ANTYDUCH_ROLE_ID | DEV_ROLE_ID => return access_lvl = 4,
            TEST_ROLE_ID if access_lvl < 3 => access_lvl = 3,
            PREMIUM_ROLE_ID | BOOSTER_ROLE_ID if access_lvl < 2 => access_lvl = 2,
            VERIFIED_ROLE_ID if access_lvl < 1 && email_verified => access_lvl = 1,
            _ => {}
        });

        Self(access_lvl)
    }

    pub fn antyduch(&self) -> bool {
        self.0 >= Self::ANTYDUCH
    }

    pub fn get(self) -> u8 {
        self.0
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct AccessClaims {
    pub sub: Uuid, // Subject (UUID)
    pub exp: u64,  // Expiration time
    pub access: AccessLevel,
}

impl AccessClaims {
    fn new(sub: Uuid, access: AccessLevel) -> Self {
        Self {
            sub,
            exp: get_current_timestamp() + ACCESS_TOKEN_DURATION,
            access,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct RefreshClaims {
    pub sub: serenity::UserId, // Subject (discord user id)
    pub exp: u64,              // Expiration time
    pub ver: usize,            // Token version
}

impl RefreshClaims {
    fn new(sub: serenity::UserId, ver: usize) -> Self {
        Self {
            sub,
            exp: get_current_timestamp() + REFRESH_TOKEN_DURATION,
            ver,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Jwt<T> {
    pub token: String,
    pub claims: T,
}

impl Jwt<AccessClaims> {
    pub fn new(access_level: AccessLevel, cid: Simple) -> Result<Self, AuthError> {
        let claims = AccessClaims::new(cid.into_uuid(), access_level);
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(
                std::env::var("JWT_ACCESS_SECRET")
                    .map_err(|err| {
                        warn!("Error during token creation! {err:#?}");
                        AuthError::TokenCreation
                    })?
                    .as_bytes(),
            ),
        )
        .map_err(|err| {
            warn!("Error during token creation! {err:#?}");
            AuthError::TokenCreation
        })?;

        Ok(Self { token, claims })
    }

    pub fn decode(token: &str) -> Result<TokenData<AccessClaims>, AuthError> {
        decode::<AccessClaims>(
            token,
            &DecodingKey::from_secret(
                std::env::var("JWT_ACCESS_SECRET")
                    .map_err(|_| AuthError::MissingCredentials)?
                    .as_bytes(),
            ),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|_| AuthError::InvalidToken)
    }
}

impl Jwt<RefreshClaims> {
    pub fn new(uid: serenity::UserId, version: usize) -> Result<Self, AuthError> {
        let claims = RefreshClaims::new(uid, version);
        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(
                std::env::var("JWT_REFRESH_SECRET")
                    .map_err(|err| {
                        warn!("Error during token creation! {err:#?}");
                        AuthError::TokenCreation
                    })?
                    .as_bytes(),
            ),
        )
        .map_err(|err| {
            warn!("Error during token creation! {err:#?}");
            AuthError::TokenCreation
        })?;

        Ok(Self { token, claims })
    }

    pub fn decode(token: &str) -> Result<TokenData<RefreshClaims>, AuthError> {
        decode::<RefreshClaims>(
            token,
            &DecodingKey::from_secret(
                std::env::var("JWT_REFRESH_SECRET")
                    .map_err(|_| AuthError::MissingCredentials)?
                    .as_bytes(),
            ),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|_| AuthError::InvalidToken)
    }
}

impl<T> From<Jwt<T>> for String {
    #[inline]
    fn from(value: Jwt<T>) -> Self {
        value.token
    }
}

// impl<T> AsRef<str> for Jwt<T>
// where
//     T: Claims + Clone + Serialize + DeserializeOwned,
// {
//     #[inline]
//     fn as_ref(&self) -> &str {
//         self.token.as_str()
//     }
// }

// impl<T> FromStr for Jwt<T>
// where
//     T: Claims + Clone + DeserializeOwned,
// {
//     type Err = ApiError;

//     fn from_str(token: &str) -> Result<Self> {
//         if let Ok(TokenData { claims, .. }) = Jwt::try_decode(token,
// std::env::var("JWT_ACCESS_SECRET")?.as_bytes()) {             return Ok(Self
// {                 token: token.to_string(),
//                 claims,
//             });
//         }
//         if let Ok(TokenData { claims, .. }) = Jwt::try_decode(token,
// std::env::var("JWT_REFRESH_SECRET")?.as_bytes()) {             return Ok(Self
// {                 token: token.to_string(),
//                 claims,
//             });
//         }

//         Err(ApiError::Token(ErrorKind::InvalidToken.into()))
//     }
// }
