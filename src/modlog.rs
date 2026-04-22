use crate::database::{get_mod_log_channel, DatabaseKey};
use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::model::id::{ChannelId, GuildId, UserId};
use serenity::prelude::Context;

pub fn mod_log_embed(action: &str, color: u32, moderator_id: UserId, target_id: UserId, reason: &str) -> CreateEmbed {
    CreateEmbed::new()
        .color(color)
        .title(action)
        .field("Membre", format!("<@{}>", target_id), true)
        .field("Modérateur", format!("<@{}>", moderator_id), true)
        .field("Raison", reason, false)
        .timestamp(serenity::model::Timestamp::now())
}

pub async fn send_mod_log(ctx: &Context, guild_id: GuildId, embed: CreateEmbed) {
    let pool = {
        let data = ctx.data.read().await;
        match data.get::<DatabaseKey>() {
            Some(p) => p.clone(),
            None => return,
        }
    };

    if let Some(channel_id) = get_mod_log_channel(&pool, &guild_id.get().to_string()).await {
        let _ = ChannelId::new(channel_id)
            .send_message(&ctx.http, CreateMessage::new().embed(embed))
            .await;
    }
}
