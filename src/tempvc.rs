use crate::database::DatabaseKey;
use serenity::builder::{CreateChannel, EditMember};
use serenity::model::channel::ChannelType;
use serenity::model::voice::VoiceState;
use serenity::prelude::Context;

pub async fn handle_voice_state(ctx: &Context, old: Option<VoiceState>, new: VoiceState) {
    let guild_id = match new.guild_id {
        Some(id) => id,
        None => return,
    };

    let pool = {
        let data = ctx.data.read().await;
        match data.get::<DatabaseKey>() {
            Some(p) => p.clone(),
            None => return,
        }
    };

    // Utilisateur a rejoint un salon
    if let Some(new_channel_id) = new.channel_id {
        let hub: Option<(String,)> = sqlx::query_as(
            "SELECT hub_channel_id FROM temp_vc_config WHERE guild_id = ?",
        )
        .bind(guild_id.get().to_string())
        .fetch_optional(&pool)
        .await
        .ok()
        .flatten();

        if let Some((hub_id_str,)) = hub {
            if hub_id_str == new_channel_id.get().to_string() {
                let user_name = new
                    .member
                    .as_ref()
                    .map(|m| m.display_name().to_string())
                    .unwrap_or_else(|| "Salon".to_string());

                // Créer le salon dans la même catégorie que le hub
                let category_id = ctx
                    .cache
                    .channel(new_channel_id)
                    .and_then(|c| c.parent_id);

                let mut builder =
                    CreateChannel::new(format!("🔊 {}", user_name)).kind(ChannelType::Voice);
                if let Some(cat) = category_id {
                    builder = builder.category(cat);
                }

                if let Ok(temp_channel) = guild_id.create_channel(&ctx.http, builder).await {
                    let _ = sqlx::query(
                        "INSERT OR IGNORE INTO temp_channels (channel_id, guild_id) VALUES (?, ?)",
                    )
                    .bind(temp_channel.id.get().to_string())
                    .bind(guild_id.get().to_string())
                    .execute(&pool)
                    .await;

                    let _ = guild_id
                        .edit_member(
                            &ctx.http,
                            new.user_id,
                            EditMember::new().voice_channel(temp_channel.id),
                        )
                        .await;
                }
            }
        }
    }

    // Utilisateur a quitté un salon
    if let Some(old_state) = old {
        if let Some(old_channel_id) = old_state.channel_id {
            let is_temp: Option<(String,)> = sqlx::query_as(
                "SELECT channel_id FROM temp_channels WHERE channel_id = ?",
            )
            .bind(old_channel_id.get().to_string())
            .fetch_optional(&pool)
            .await
            .ok()
            .flatten();

            if is_temp.is_some() {
                let is_empty = ctx
                    .cache
                    .guild(guild_id)
                    .map(|g| {
                        !g.voice_states
                            .values()
                            .any(|vs| vs.channel_id == Some(old_channel_id))
                    })
                    .unwrap_or(true);

                if is_empty {
                    let _ = old_channel_id.delete(&ctx.http).await;
                    let _ = sqlx::query("DELETE FROM temp_channels WHERE channel_id = ?")
                        .bind(old_channel_id.get().to_string())
                        .execute(&pool)
                        .await;
                }
            }
        }
    }
}
