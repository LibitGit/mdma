use serde_repr::{Deserialize_repr, Serialize_repr};

/// Whether the settings should be saved for the game account, character or the
/// discord user.
#[derive(Debug, Clone, Copy, Serialize_repr, Deserialize_repr, Default)]
#[repr(u8)]
pub enum SessionScope {
    GameCharacter = 0,
    #[default]
    GameAccount,
    DiscordAccount,
}
