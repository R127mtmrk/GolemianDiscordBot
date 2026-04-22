use serde_json::{json, Value};
use serenity::builder::{CreateAttachment, CreateChannel, CreateEmbed, CreateMessage, EditRole};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::model::channel::ChannelType;
use serenity::model::id::ChannelId;
use serenity::model::permissions::Permissions;
use serenity::model::prelude::*;
use serenity::prelude::*;

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

#[group]
#[commands(sbackup, srestore)]
#[only_in(guilds)]
pub struct Server;

#[command]
#[required_permissions(MANAGE_GUILD)]
#[description("Exporter la structure du serveur (rôles, catégories, salons) en JSON")]
async fn sbackup(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let guild = guild_id.to_partial_guild(&ctx.http).await?;
    let channels = guild_id.channels(&ctx.http).await?;

    let mut roles_json: Vec<Value> = guild
        .roles
        .values()
        .filter(|r| r.name != "@everyone")
        .map(|r| {
            json!({
                "name": r.name,
                "color": r.colour.0,
                "permissions": r.permissions.bits(),
                "hoist": r.hoist,
                "mentionable": r.mentionable,
                "position": r.position,
            })
        })
        .collect();
    roles_json.sort_by_key(|r| r["position"].as_i64().unwrap_or(0));

    let mut categories_json: Vec<Value> = Vec::new();
    let mut orphan_channels_json: Vec<Value> = Vec::new();

    for (cat_id, cat_ch) in channels.iter().filter(|(_, c)| c.kind == ChannelType::Category) {
        let mut channels_in_cat: Vec<Value> = channels
            .values()
            .filter(|c| c.parent_id == Some(*cat_id))
            .map(channel_to_json)
            .collect();
        channels_in_cat.sort_by_key(|c| c["position"].as_i64().unwrap_or(0));

        categories_json.push(json!({
            "name": cat_ch.name,
            "position": cat_ch.position,
            "channels": channels_in_cat,
        }));
    }
    categories_json.sort_by_key(|c| c["position"].as_i64().unwrap_or(0));

    for ch in channels.values() {
        if ch.kind != ChannelType::Category && ch.parent_id.is_none() {
            orphan_channels_json.push(channel_to_json(ch));
        }
    }
    orphan_channels_json.sort_by_key(|c| c["position"].as_i64().unwrap_or(0));

    let backup = json!({
        "guild_name": guild.name,
        "roles": roles_json,
        "categories": categories_json,
        "channels": orphan_channels_json,
    });

    let json_bytes = serde_json::to_vec_pretty(&backup)?;

    msg.channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .content(format!("📦 Sauvegarde de **{}**", guild.name))
                .add_file(CreateAttachment::bytes(json_bytes, "backup.json")),
        )
        .await?;

    Ok(())
}

fn channel_to_json(ch: &serenity::model::channel::GuildChannel) -> Value {
    let kind_str = match ch.kind {
        ChannelType::Voice => "Voice",
        ChannelType::Text => "Text",
        ChannelType::News => "News",
        _ => "Text",
    };
    json!({
        "name": ch.name,
        "type": kind_str,
        "topic": ch.topic,
        "nsfw": ch.nsfw,
        "slowmode": ch.rate_limit_per_user.unwrap_or(0),
        "position": ch.position,
        "bitrate": ch.bitrate,
        "user_limit": ch.user_limit,
    })
}

#[command]
#[required_permissions(ADMINISTRATOR)]
#[description("Importer une structure serveur depuis un fichier backup.json (joindre le fichier)")]
async fn srestore(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let attachment = match msg.attachments.first() {
        Some(a) => a,
        None => {
            error_embed(ctx, msg, "❌ Joindre un fichier `backup.json` à la commande.").await;
            return Ok(());
        }
    };

    let bytes = attachment.download().await?;
    let backup: Value = match serde_json::from_slice(&bytes) {
        Ok(v) => v,
        Err(_) => {
            error_embed(ctx, msg, "❌ Fichier JSON invalide.").await;
            return Ok(());
        }
    };

    let status = msg
        .channel_id
        .send_message(&ctx.http, CreateMessage::new().content("⏳ Restauration en cours..."))
        .await?;

    let empty_vec = vec![];

    // Rôles
    let mut roles = backup["roles"].as_array().unwrap_or(&empty_vec).clone();
    roles.sort_by_key(|r| r["position"].as_i64().unwrap_or(0));

    for role in &roles {
        let name = role["name"].as_str().unwrap_or("role");
        let color = role["color"].as_u64().unwrap_or(0) as u32;
        let perms = role["permissions"].as_u64().unwrap_or(0);
        let hoist = role["hoist"].as_bool().unwrap_or(false);
        let mentionable = role["mentionable"].as_bool().unwrap_or(false);

        let _ = guild_id
            .create_role(
                &ctx.http,
                EditRole::new()
                    .name(name)
                    .colour(color)
                    .permissions(Permissions::from_bits_truncate(perms))
                    .hoist(hoist)
                    .mentionable(mentionable),
            )
            .await;

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Catégories et leurs salons
    let categories = backup["categories"].as_array().unwrap_or(&empty_vec);
    for cat in categories {
        let cat_name = cat["name"].as_str().unwrap_or("category");

        let category = match guild_id
            .create_channel(&ctx.http, CreateChannel::new(cat_name).kind(ChannelType::Category))
            .await
        {
            Ok(c) => c,
            Err(_) => continue,
        };

        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        for ch in cat["channels"].as_array().unwrap_or(&empty_vec) {
            let _ = create_channel_from_json(ctx, guild_id, ch, Some(category.id)).await;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    }

    // Salons hors catégorie
    for ch in backup["channels"].as_array().unwrap_or(&empty_vec) {
        let _ = create_channel_from_json(ctx, guild_id, ch, None).await;
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    let _ = status.delete(&ctx.http).await;

    let guild_name = backup["guild_name"].as_str().unwrap_or("backup");
    msg.channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .color(GREEN)
                    .description(format!("✅ Structure de **{}** restaurée.", guild_name)),
            ),
        )
        .await?;

    Ok(())
}

async fn create_channel_from_json(
    ctx: &Context,
    guild_id: GuildId,
    ch: &Value,
    category_id: Option<ChannelId>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let name = ch["name"].as_str().unwrap_or("channel");
    let is_voice = ch["type"].as_str().unwrap_or("Text") == "Voice";
    let kind = if is_voice { ChannelType::Voice } else { ChannelType::Text };

    let mut builder = CreateChannel::new(name).kind(kind);

    if let Some(cat_id) = category_id {
        builder = builder.category(cat_id);
    }

    if is_voice {
        if let Some(bitrate) = ch["bitrate"].as_u64().filter(|&b| b > 0) {
            builder = builder.bitrate(bitrate as u32);
        }
        if let Some(limit) = ch["user_limit"].as_u64() {
            builder = builder.user_limit(limit as u32);
        }
    } else {
        if let Some(topic) = ch["topic"].as_str().filter(|s| !s.is_empty()) {
            builder = builder.topic(topic);
        }
        let slowmode = ch["slowmode"].as_u64().unwrap_or(0) as u16;
        if slowmode > 0 {
            builder = builder.rate_limit_per_user(slowmode);
        }
        if ch["nsfw"].as_bool().unwrap_or(false) {
            builder = builder.nsfw(true);
        }
    }

    guild_id.create_channel(&ctx.http, builder).await?;
    Ok(())
}
