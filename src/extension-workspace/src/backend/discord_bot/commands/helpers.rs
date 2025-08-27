use crate::{discord_bot::Context, prelude::*};

/// Wyświetla dokumentację zestawu.
#[poise::command(prefix_command)]
pub(in crate::discord_bot) async fn doc(ctx: Context<'_>) -> Result<()> {
    let response = "\
- **Dokumentacja zestawu:** <https://libit.ovh>
- **Lista dodatków:** <https://libit.ovh/addons/index.html>\
    ";
    ctx.say(response).await?;
    
    Ok(())
}
