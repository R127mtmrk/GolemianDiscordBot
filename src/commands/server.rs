use std::collections::HashMap;

use serde_json::{json, Value};
use serenity::builder::{CreateAttachment, CreateChannel, CreateEmbed, CreateMessage, EditRole};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::model::channel::{
    ChannelType, GuildChannel, PermissionOverwrite, PermissionOverwriteType,
};
use serenity::model::id::{ChannelId, GuildId, RoleId};
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
#[description("Exporter la structure complète du serveur (rôles, catégories, salons, permissions) en JSON")]
async fn sbackup(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let guild = guild_id.to_partial_guild(&ctx.http).await?;
    let channels = guild_id.channels(&ctx.http).await?;

    let everyone_id = guild_id.get().to_string();

    let mut roles_json: Vec<Value> = guild
        .roles
        .values()
        .filter(|r| r.name != "@everyone")
        .map(|r| {
            json!({
                "id": r.id.get().to_string(),
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
            "permission_overwrites": overwrites_to_json(&cat_ch.permission_overwrites),
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
        "guild_id": guild_id.get().to_string(),
        "everyone_role_id": everyone_id,
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

fn channel_type_str(kind: ChannelType) -> &'static str {
    match kind {
        ChannelType::Voice => "Voice",
        ChannelType::Text => "Text",
        ChannelType::News => "News",
        ChannelType::Stage => "Stage",
        ChannelType::Forum => "Forum",
        _ => "Text",
    }
}

fn overwrites_to_json(ows: &[PermissionOverwrite]) -> Vec<Value> {
    ows.iter()
        .map(|ow| {
            let (kind, id) = match ow.kind {
                PermissionOverwriteType::Role(rid) => ("role", rid.get().to_string()),
                PermissionOverwriteType::Member(uid) => ("member", uid.get().to_string()),
                _ => ("role", "0".to_string()),
            };
            json!({
                "type": kind,
                "id": id,
                "allow": ow.allow.bits(),
                "deny": ow.deny.bits(),
            })
        })
        .collect()
}

fn channel_to_json(ch: &GuildChannel) -> Value {
    json!({
        "name": ch.name,
        "type": channel_type_str(ch.kind),
        "topic": ch.topic,
        "nsfw": ch.nsfw,
        "slowmode": ch.rate_limit_per_user.unwrap_or(0),
        "position": ch.position,
        "bitrate": ch.bitrate,
        "user_limit": ch.user_limit,
        "permission_overwrites": overwrites_to_json(&ch.permission_overwrites),
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

    let old_everyone_id = backup["everyone_role_id"]
        .as_str()
        .or_else(|| backup["guild_id"].as_str())
        .unwrap_or("")
        .to_string();

    // Rôles — on construit une map old_id -> new_role_id pour remapper les overwrites
    let mut role_map: HashMap<String, RoleId> = HashMap::new();

    let mut roles = backup["roles"].as_array().unwrap_or(&empty_vec).clone();
    roles.sort_by_key(|r| r["position"].as_i64().unwrap_or(0));

    for role in &roles {
        let old_id = role["id"].as_str().unwrap_or("").to_string();
        let name = role["name"].as_str().unwrap_or("role");
        let color = role["color"].as_u64().unwrap_or(0) as u32;
        let perms = role["permissions"].as_u64().unwrap_or(0);
        let hoist = role["hoist"].as_bool().unwrap_or(false);
        let mentionable = role["mentionable"].as_bool().unwrap_or(false);

        let created = guild_id
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

        if let (Ok(new_role), false) = (created, old_id.is_empty()) {
            role_map.insert(old_id, new_role.id);
        }

        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
    }

    // Catégories et leurs salons
    let categories = backup["categories"].as_array().unwrap_or(&empty_vec);
    for cat in categories {
        let cat_name = cat["name"].as_str().unwrap_or("category");
        let cat_overwrites = build_overwrites(
            cat["permission_overwrites"].as_array().unwrap_or(&empty_vec),
            &role_map,
            &old_everyone_id,
            guild_id,
        );

        let mut cat_builder = CreateChannel::new(cat_name).kind(ChannelType::Category);
        if !cat_overwrites.is_empty() {
            cat_builder = cat_builder.permissions(cat_overwrites);
        }

        let category = match guild_id.create_channel(&ctx.http, cat_builder).await {
            Ok(c) => c,
            Err(_) => continue,
        };

        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        for ch in cat["channels"].as_array().unwrap_or(&empty_vec) {
            let _ = create_channel_from_json(
                ctx,
                guild_id,
                ch,
                Some(category.id),
                &role_map,
                &old_everyone_id,
            )
            .await;
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    }

    // Salons hors catégorie
    for ch in backup["channels"].as_array().unwrap_or(&empty_vec) {
        let _ =
            create_channel_from_json(ctx, guild_id, ch, None, &role_map, &old_everyone_id).await;
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

fn build_overwrites(
    overwrites_json: &[Value],
    role_map: &HashMap<String, RoleId>,
    old_everyone_id: &str,
    new_guild_id: GuildId,
) -> Vec<PermissionOverwrite> {
    overwrites_json
        .iter()
        .filter_map(|ow| {
            let kind_str = ow["type"].as_str()?;
            let id_str = ow["id"].as_str()?;
            let allow = Permissions::from_bits_truncate(ow["allow"].as_u64().unwrap_or(0));
            let deny = Permissions::from_bits_truncate(ow["deny"].as_u64().unwrap_or(0));

            let kind = match kind_str {
                "role" => {
                    let role_id = if !old_everyone_id.is_empty() && id_str == old_everyone_id {
                        RoleId::new(new_guild_id.get())
                    } else {
                        *role_map.get(id_str)?
                    };
                    PermissionOverwriteType::Role(role_id)
                }
                // Les membres ne sont pas remappables : on les ignore pour ne pas
                // recréer des overwrites qui référenceraient des utilisateurs absents.
                _ => return None,
            };

            Some(PermissionOverwrite { allow, deny, kind })
        })
        .collect()
}

async fn create_channel_from_json(
    ctx: &Context,
    guild_id: GuildId,
    ch: &Value,
    category_id: Option<ChannelId>,
    role_map: &HashMap<String, RoleId>,
    old_everyone_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let name = ch["name"].as_str().unwrap_or("channel");
    let kind_str = ch["type"].as_str().unwrap_or("Text");
    let kind = match kind_str {
        "Voice" => ChannelType::Voice,
        "News" => ChannelType::News,
        "Stage" => ChannelType::Stage,
        "Forum" => ChannelType::Forum,
        _ => ChannelType::Text,
    };

    let mut builder = CreateChannel::new(name).kind(kind);

    if let Some(cat_id) = category_id {
        builder = builder.category(cat_id);
    }

    match kind {
        ChannelType::Voice | ChannelType::Stage => {
            if let Some(bitrate) = ch["bitrate"].as_u64().filter(|&b| b > 0) {
                builder = builder.bitrate(bitrate as u32);
            }
            if let Some(limit) = ch["user_limit"].as_u64() {
                builder = builder.user_limit(limit as u32);
            }
        }
        _ => {
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
    }

    let overwrites = build_overwrites(
        ch["permission_overwrites"].as_array().unwrap_or(&vec![]),
        role_map,
        old_everyone_id,
        guild_id,
    );
    if !overwrites.is_empty() {
        builder = builder.permissions(overwrites);
    }

    guild_id.create_channel(&ctx.http, builder).await?;
    Ok(())
}
