use std::time::Duration;

use common::connection::SessionScope;
use mongodb::{
    Collection, IndexModel,
    bson::{self, DateTime, Document, doc},
    options::{
        CreateCollectionOptions, IndexOptions, ReturnDocument, ValidationAction, ValidationLevel,
    },
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_with::{DisplayFromStr, serde_as};

use crate::prelude::*;

const DB_NAME: &str = "margonem";

#[derive(Debug, Clone)]
pub struct Client(mongodb::Client);

impl std::ops::Deref for Client {
    type Target = mongodb::Client;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Client {
    pub async fn connect() -> Result<Self> {
        let db_uri = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "mongodb://libit:thisIsALocalHostPassword@127.0.0.1:27017/?authSource=admin".to_string()
        });
        let client = Self(mongodb::Client::with_uri_str(db_uri).await?);
        let db = client.database(DB_NAME);

        db.run_command(mongodb::bson::doc! {"ping": 1}).await?;

        info!("Successfully connected to MongoDB!");

        let mut collections = db.list_collection_names().await?;

        for (collection_name, validator) in Self::default_collections() {
            if collections.contains(&collection_name) {
                continue;
            }

            let options = CreateCollectionOptions::builder()
                .validator(validator)
                .validation_level(ValidationLevel::Strict)
                .validation_action(ValidationAction::Error)
                .build();

            db.create_collection(&collection_name)
                .with_options(options)
                .await?;

            info!("Created collection '{}'.", collection_name);
            collections.push(collection_name);
        }

        let premium_details = client.get_collection::<Premium>();
        let index_name = "premium_ttl";
        if !premium_details
            .list_index_names()
            .await?
            .iter()
            .any(|name| name == index_name)
        {
            let index_name = premium_details
                .create_index(
                    IndexModel::builder()
                        .keys(doc! { "exp": 1 })
                        .options(
                            IndexOptions::builder()
                                .expire_after(Duration::ZERO)
                                .name(index_name.to_string())
                                .build(),
                        )
                        .build(),
                )
                .await?
                .index_name;

            info!(
                "Created index '{index_name}' in collection '{}'.",
                <Premium as IntoCollection>::COLLECTION_NAME
            );
        }

        info!("--- Collections in '{DB_NAME}' database ---");

        for name in collections {
            let count = db
                .collection::<Document>(&name)
                .count_documents(doc! {})
                .await?;

            match count {
                0 => {
                    info!("{name}: no documents found!");
                    continue;
                }
                _ => info!("{name}: documents count '{}'", count),
            }
        }

        info!("--- End of collections in '{DB_NAME}' database ---");

        Ok(client)
    }

    // TODO: Better impl of this ?
    fn default_collections() -> [(String, Document); COLLECTION_COUNT] {
        [
            (
                <DiscordAccount as IntoCollection>::COLLECTION_NAME.into(),
                <DiscordAccount as IntoCollection>::validator(),
            ),
            (
                <GameAccount as IntoCollection>::COLLECTION_NAME.into(),
                <GameAccount as IntoCollection>::validator(),
            ),
            (
                <GameCharacter as IntoCollection>::COLLECTION_NAME.into(),
                <GameCharacter as IntoCollection>::validator(),
            ),
            (
                <Premium as IntoCollection>::COLLECTION_NAME.into(),
                <Premium as IntoCollection>::validator(),
            ),
        ]
    }

    pub async fn validate_refresh_token(
        &self,
        refresh_token: &str,
    ) -> std::result::Result<DiscordAccount, AuthError> {
        let claims = Jwt::<RefreshClaims>::decode(refresh_token)?.claims;
        let discord_accounts = self.get_collection::<DiscordAccount>();
        let filter = doc! { "_id": claims.sub.to_string() };
        let account = discord_accounts
            .find_one(filter)
            .await
            .map_err(|err| {
                warn!("MongoDB error when validating refresh token! {err}");
                AuthError::MongoDbError
            })?
            .ok_or_else(|| AuthError::WrongCredentials)?;

        match claims.ver {
            ver if ver > account.version => Err(AuthError::TokenFraudulent),
            ver if ver == account.version => Ok(account),
            _ => Err(AuthError::InvalidToken),
        }
    }

    fn get_collection<T>(&self) -> Collection<T>
    where
        T: IntoCollection + Send + Sync,
    {
        self.database(DB_NAME).collection(T::COLLECTION_NAME)
    }

    /// Fetch a discord account instance with the provided id.
    pub async fn get_discord_account(
        &self,
        uid: serenity::UserId,
    ) -> Result<Option<DiscordAccount>> {
        let discord_accounts = self.get_collection::<DiscordAccount>();
        let filter = doc! { "_id": uid.to_string() };

        Ok(discord_accounts.find_one(filter).await?)
    }

    pub async fn get_premium_details(&self, uid: serenity::UserId) -> Result<Option<Premium>> {
        let premium_details = self.get_collection::<Premium>();
        let filter = doc! { "_id": uid.to_string() };

        premium_details
            .find_one(filter)
            .await
            .with_context(|| format!("Could not get premium details for {uid}"))
    }

    /// Find one or insert a new [`DiscordAccount`], updating it with the
    /// provided `email_verified`.
    pub(super) async fn update_discord_account_login(&self, uid: serenity::UserId) -> Result<()> {
        let discord_accounts = self.get_collection::<DiscordAccount>();
        let query = doc! { "_id": uid.to_string() };
        let update = doc! { "$currentDate": { "last_login": true } };

        discord_accounts.update_one(query, update).await?;

        Ok(())
    }

    /// Find one or insert a new [`DiscordAccount`], updating it with the
    /// provided `email_verified`.
    pub(super) async fn upsert_discord_account_login(
        &self,
        uid: serenity::UserId,
        email_verified: bool,
    ) -> Result<DiscordAccount> {
        let discord_accounts = self.get_collection::<DiscordAccount>();
        let filter = doc! { "_id": uid.to_string() };

        let mut base_doc = bson::to_document(&DiscordAccount::new(uid, email_verified))?;
        base_doc.remove("email_verified");
        base_doc.remove("last_login");

        let update = doc! {
            "$set": {
                "email_verified": email_verified,
            },
            "$currentDate": {
                "last_login": true,
            },
            "$setOnInsert": base_doc,
        };

        discord_accounts
            .find_one_and_update(filter, update)
            .return_document(ReturnDocument::After)
            .upsert(true)
            .await?
            .ok_or_else(|| anyhow!("Failed to upsert DiscordAccount with uid: '{uid}'!"))
    }
}

pub trait IntoCollection {
    const COLLECTION_NAME: &str;

    fn validator() -> Document;
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct DiscordAccount {
    #[serde(rename = "_id")]
    #[serde_as(as = "DisplayFromStr")]
    pub id: serenity::UserId,
    // pub email: String,
    pub email_verified: bool,
    pub version: usize,
    pub session_scope: SessionScope,
    pub inserted_at: DateTime,
    pub last_login: DateTime,
    pub settings: Value,
}

impl DiscordAccount {
    pub fn new(id: serenity::UserId, email_verified: bool) -> Self {
        let now = DateTime::now();

        Self {
            id,
            email_verified,
            version: 0,
            session_scope: SessionScope::default(),
            inserted_at: now,
            last_login: now,
            settings: Value::Object(Default::default()),
        }
    }
}

/// Margonem account structure
#[derive(Debug, Serialize, Deserialize)]
pub struct GameAccount {
    #[serde(rename = "_id")]
    id: String,
    inserted_at: DateTime,
    last_login: DateTime,
}

/// Character structure
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct GameCharacter {
    #[serde(rename = "_id")]
    id: String, // Character identifier
    account_id: String, // Reference to margonem account
    settings: Value,
    inserted_at: DateTime,
    last_login: DateTime,
}

#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
pub struct Premium {
    #[serde(rename = "_id")]
    #[serde_as(as = "DisplayFromStr")]
    id: serenity::UserId,
    pub(super) exp: DateTime,
    pub(super) neon: bool,
    pub(super) animation: bool,
}

into_collection! {
    DiscordAccount: {
        @name: "discord_accounts",
        @validator: doc! {
            "$jsonSchema": doc! {
                "bsonType": "object",
                "title": "Discord Account Validation",
                "additionalProperties": false,
                "required": [ "_id", "email_verified", "version", "inserted_at", "last_login", "settings" ],
                "properties": doc! {
                    "_id": {
                        "bsonType": "string",
                        "description": "Discord user identifier"
                    },
                    "email_verified": {
                        "bsonType": "bool",
                        "description": "Boolean stating if email has been verified or not"
                    },
                    "version": {
                        "bsonType": "long",
                        "description": "Refresh token version for login control"
                    },
                    "session_scope": {
                        "bsonType": "int",
                        "description": "Whether the settings should be saved for the game account, character or the discord user."
                    },
                    "inserted_at": {
                        "bsonType": "date",
                        "description": "Account insertion timestamp"
                    },
                    "last_login": {
                        "bsonType": "date",
                        "description": "Last login timestamp"
                    },
                    "settings": {
                        "bsonType": "object",
                        "description": "Addons settings object"
                    },
                }
            }
        }
    },
    GameAccount: {
        @name: "accounts",
        @validator: doc! {
            "$jsonSchema": doc! {
                "bsonType": "object",
                "title": "Margonem Account Validation",
                "additionalProperties": false,
                "required": [ "_id", "inserted_at", "last_login" ],
                "properties": doc! {
                    "_id": {
                        "bsonType": "string",
                        "description": "Margonem account identifier"
                    },
                    "inserted_at": {
                        "bsonType": "date",
                        "description": "Account insertion timestamp"
                    },
                    "last_login": {
                        "bsonType": "date",
                        "description": "Last login timestamp"
                    }
                }
            }
        },
    },
    GameCharacter: {
        @name: "characters",
        @validator: doc! {
            "$jsonSchema": doc! {
                "bsonType": "object",
                "title": "Margonem Character Validation",
                "additionalProperties": false,
                "required": [ "_id", "account_id", "settings", "inserted_at", "last_login" ],
                "properties": doc! {
                    "_id": {
                        "bsonType":  "string",
                        "description": "Margonem character identifier"
                    },
                    "account_id": {
                        "bsonType": "string",
                        "description": "Margonem account identifier"
                    },
                    "settings": {
                        "bsonType": "object",
                        "description": "Addons settings object"
                    },
                    "inserted_at": {
                        "bsonType": "date",
                        "description": "Account insertion timestamp"
                    },
                    "last_login": {
                        "bsonType": "date",
                        "description": "Last login timestamp"
                    }
                }
            }
        },
    },
    Premium: {
        @name: "premium",
        @validator: doc! {
            "$jsonSchema": doc! {
                "bsonType": "object",
                "title": "Premium Data Validation",
                "additionalProperties": false,
                "required": [ "_id", "exp", "neon", "animation" ],
                "properties": doc! {
                    "_id": {
                        "bsonType": "string",
                        "description": "Discord user identifier."
                    },
                    "exp": {
                        "bsonType": "date",
                        "description": "Premium expiration date."
                    },
                    "neon": {
                        "bsonType": "bool",
                        "description": "Whether the user has the hero neon addon available."
                    },
                    "animation": {
                        "bsonType": "bool",
                        "description": "Whether the user has the hero animation addon available."
                    }
                }
            }
        }
    }
}
