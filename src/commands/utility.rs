use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage, EditChannel};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::model::channel::{PermissionOverwrite, PermissionOverwriteType};
use serenity::model::id::{ChannelId, RoleId};
use serenity::model::permissions::Permissions;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::database::{set_mod_log_channel, DatabaseKey};

const BLUE: u32 = 0x5865F2;
const GREEN: u32 = 0x2ECC71;
const RED: u32 = 0xE74C3C;

fn parse_channel_mention(s: &str) -> Option<ChannelId> {
    let s = s.trim();
    let id_str = if s.starts_with("<#") && s.ends_with('>') {
        &s[2..s.len() - 1]
    } else {
        s
    };
    id_str.parse::<u64>().ok().map(ChannelId::new)
}

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
#[commands(slowmode, lock, unlock, userinfo, setmodlog)]
#[only_in(guilds)]
pub struct Utility;

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("Définir le mode lent sur le salon actuel (0 pour désactiver)")]
#[usage("<secondes>  (0-21600)")]
async fn slowmode(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let seconds = match args.single::<u16>() {
        Ok(s) if s <= 21600 => s,
        _ => {
            error_embed(ctx, msg, "❌ Usage: `!slowmode <secondes>` (0–21600)").await;
            return Ok(());
        }
    };

    match msg
        .channel_id
        .edit(&ctx.http, EditChannel::new().rate_limit_per_user(seconds))
        .await
    {
        Ok(_) => {
            if seconds == 0 {
                success_embed(ctx, msg, "✅ Mode lent **désactivé**.").await;
            } else {
                success_embed(ctx, msg, &format!("✅ Mode lent réglé à **{}s**.", seconds)).await;
            }
        }
        Err(e) => {
            error_embed(ctx, msg, &format!("❌ Erreur: {}", e)).await;
        }
    }
    Ok(())
}

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("Verrouiller un salon (bloque les messages de @everyone)")]
#[usage("[#salon]")]
async fn lock(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let channel_id = if let Ok(raw) = args.single::<String>() {
        parse_channel_mention(&raw).unwrap_or(msg.channel_id)
    } else {
        msg.channel_id
    };

    let overwrite = PermissionOverwrite {
        allow: Permissions::empty(),
        deny: Permissions::SEND_MESSAGES,
        kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
    };

    match channel_id.create_permission(&ctx.http, overwrite).await {
        Ok(_) => {
            success_embed(
                ctx,
                msg,
                &format!("🔒 <#{}> est maintenant **verrouillé**.", channel_id),
            )
            .await;
        }
        Err(e) => {
            error_embed(ctx, msg, &format!("❌ Impossible de verrouiller: {}", e)).await;
        }
    }
    Ok(())
}

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("Déverrouiller un salon")]
#[usage("[#salon]")]
async fn unlock(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let channel_id = if let Ok(raw) = args.single::<String>() {
        parse_channel_mention(&raw).unwrap_or(msg.channel_id)
    } else {
        msg.channel_id
    };

    match channel_id
        .delete_permission(
            &ctx.http,
            PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
        )
        .await
    {
        Ok(_) => {
            success_embed(
                ctx,
                msg,
                &format!("🔓 <#{}> est maintenant **déverrouillé**.", channel_id),
            )
            .await;
        }
        Err(e) => {
            error_embed(ctx, msg, &format!("❌ Impossible de déverrouiller: {}", e)).await;
        }
    }
    Ok(())
}

#[command]
#[description("Afficher les informations d'un membre")]
#[usage("[@utilisateur]")]
async fn userinfo(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().unwrap().clone()
    };

    let user_id = if let Ok(raw) = args.single::<String>() {
        let s = raw.trim();
        let id_str = if s.starts_with("<@") && s.ends_with('>') {
            s.trim_start_matches("<@!").trim_start_matches("<@").trim_end_matches('>')
        } else {
            s
        };
        id_str
            .parse::<u64>()
            .map(UserId::new)
            .unwrap_or(msg.author.id)
    } else {
        msg.author.id
    };

    let member = match guild_id.member(&ctx.http, user_id).await {
        Ok(m) => m,
        Err(_) => {
            error_embed(ctx, msg, "❌ Membre introuvable.").await;
            return Ok(());
        }
    };

    let user = &member.user;
    let created_at = user_id.created_at().unix_timestamp();
    let joined_at = member.joined_at.map(|t| t.unix_timestamp()).unwrap_or(0);

    let roles_str = if member.roles.is_empty() {
        "Aucun".to_string()
    } else {
        member
            .roles
            .iter()
            .map(|r| format!("<@&{}>", r))
            .collect::<Vec<_>>()
            .join(", ")
    };

    let warn_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM warnings WHERE guild_id = ? AND user_id = ?",
    )
    .bind(guild_id.get().to_string())
    .bind(user_id.get().to_string())
    .fetch_one(&pool)
    .await
    .unwrap_or((0,));

    let embed = CreateEmbed::new()
        .color(BLUE)
        .title(format!("👤 {}", user.name))
        .thumbnail(user.avatar_url().unwrap_or_default())
        .field("ID", user_id.to_string(), true)
        .field("Compte créé", format!("<t:{}:R>", created_at), true)
        .field("A rejoint", format!("<t:{}:R>", joined_at), true)
        .field("Rôles", &roles_str, false)
        .field("Avertissements", warn_count.0.to_string(), true)
        .footer(CreateEmbedFooter::new(format!("Bot Golemian")));

    msg.channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await?;
    Ok(())
}

#[command]
#[required_permissions(MANAGE_GUILD)]
#[description("Définir le salon des logs de modération")]
#[usage("#salon")]
async fn setmodlog(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().unwrap().clone()
    };

    let raw = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!setmodlog #salon`").await;
            return Ok(());
        }
    };

    let channel_id = match parse_channel_mention(&raw) {
        Some(id) => id,
        None => {
            error_embed(ctx, msg, "❌ Salon invalide.").await;
            return Ok(());
        }
    };

    set_mod_log_channel(
        &pool,
        &guild_id.get().to_string(),
        &channel_id.get().to_string(),
    )
    .await?;

    success_embed(
        ctx,
        msg,
        &format!("✅ Logs de modération définis sur <#{}>.", channel_id),
    )
    .await;
    Ok(())
}
