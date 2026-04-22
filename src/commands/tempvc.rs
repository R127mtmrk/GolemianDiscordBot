use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::model::id::ChannelId;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::database::DatabaseKey;

const GREEN: u32 = 0x2ECC71;
const RED: u32 = 0xE74C3C;

async fn error_embed(ctx: &Context, msg: &Message, text: &str) {
    let _ = msg
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(CreateEmbed::new().color(RED).description(text)),
        )
        .await;
}

async fn success_embed(ctx: &Context, msg: &Message, text: &str) {
    let _ = msg
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(CreateEmbed::new().color(GREEN).description(text)),
        )
        .await;
}

#[group]
#[commands(settempvc, deltempvc)]
#[only_in(guilds)]
pub struct TempVc;

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("Définir le salon vocal hub pour les salons temporaires")]
#[usage("#salon-vocal")]
async fn settempvc(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().unwrap().clone()
    };

    let raw = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!settempvc #salon-vocal`").await;
            return Ok(());
        }
    };

    let id_str = raw.trim().trim_start_matches("<#").trim_end_matches('>');
    let channel_id = match id_str.parse::<u64>() {
        Ok(id) => ChannelId::new(id),
        Err(_) => {
            error_embed(ctx, msg, "❌ Salon invalide.").await;
            return Ok(());
        }
    };

    sqlx::query(
        "INSERT INTO temp_vc_config (guild_id, hub_channel_id) VALUES (?, ?)
         ON CONFLICT(guild_id) DO UPDATE SET hub_channel_id = excluded.hub_channel_id",
    )
    .bind(guild_id.get().to_string())
    .bind(channel_id.get().to_string())
    .execute(&pool)
    .await?;

    success_embed(
        ctx,
        msg,
        &format!(
            "✅ Salon hub défini sur <#{}>.\nRejoins ce salon pour créer un salon vocal temporaire.",
            channel_id
        ),
    )
    .await;
    Ok(())
}

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("Désactiver les salons vocaux temporaires")]
async fn deltempvc(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().unwrap().clone()
    };

    sqlx::query("DELETE FROM temp_vc_config WHERE guild_id = ?")
        .bind(guild_id.get().to_string())
        .execute(&pool)
        .await?;

    success_embed(ctx, msg, "✅ Salons vocaux temporaires **désactivés**.").await;
    Ok(())
}
