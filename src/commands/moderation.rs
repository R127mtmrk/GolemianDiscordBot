use chrono::Utc;
use serenity::builder::{CreateEmbed, CreateMessage, EditMember, GetMessages};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;
use sqlx::SqlitePool;

use crate::database::{DatabaseKey, Warning};
use crate::modlog::{mod_log_embed, send_mod_log};

const RED: u32 = 0xE74C3C;
const GREEN: u32 = 0x2ECC71;

fn parse_duration(s: &str) -> Option<chrono::Duration> {
    let s = s.trim();
    if s.ends_with('s') {
        s[..s.len() - 1].parse::<i64>().ok().map(chrono::Duration::seconds)
    } else if s.ends_with('m') {
        s[..s.len() - 1].parse::<i64>().ok().map(chrono::Duration::minutes)
    } else if s.ends_with('h') {
        s[..s.len() - 1].parse::<i64>().ok().map(chrono::Duration::hours)
    } else if s.ends_with('d') {
        s[..s.len() - 1].parse::<i64>().ok().map(chrono::Duration::days)
    } else {
        None
    }
}

fn parse_user_id(s: &str) -> Option<UserId> {
    let s = s.trim();
    let id_str = if s.starts_with("<@") && s.ends_with('>') {
        s.trim_start_matches("<@!").trim_start_matches("<@").trim_end_matches('>')
    } else {
        s
    };
    id_str.parse::<u64>().ok().map(UserId::new)
}

fn parse_users_and_rest(args: &mut Args) -> (Vec<UserId>, String) {
    let mut users = Vec::new();
    loop {
        match args.single::<String>() {
            Ok(raw) => {
                if let Some(uid) = parse_user_id(&raw) {
                    users.push(uid);
                } else {
                    let rest = if args.is_empty() {
                        raw
                    } else {
                        format!("{} {}", raw, args.rest())
                    };
                    return (users, rest);
                }
            }
            Err(_) => break,
        }
    }
    (users, String::new())
}

async fn send_embed(ctx: &Context, msg: &Message, color: u32, text: &str) {
    let _ = msg
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(CreateEmbed::new().color(color).description(text)),
        )
        .await;
}

async fn get_pool(ctx: &Context) -> SqlitePool {
    ctx.data.read().await.get::<DatabaseKey>().unwrap().clone()
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
#[commands(ban, unban, kick, mute, unmute, warn, warnings, clearwarns, clear)]
#[only_in(guilds)]
pub struct Moderation;

#[command]
#[required_permissions(BAN_MEMBERS)]
#[description("Bannir un membre du serveur")]
#[usage("@utilisateur [raison]")]
async fn ban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let (user_ids, reason) = parse_users_and_rest(&mut args);
    let reason = if reason.is_empty() { "Aucune raison fournie".to_string() } else { reason };

    if user_ids.is_empty() {
        error_embed(ctx, msg, "❌ Usage: `!ban @user1 [@user2 ...] [raison]`").await;
        return Ok(());
    }

    let mut success = Vec::new();
    let mut errors = Vec::new();

    for user_id in &user_ids {
        match guild_id.ban_with_reason(&ctx.http, *user_id, 0, &reason).await {
            Ok(_) => {
                success.push(format!("<@{}>", user_id));
                send_mod_log(ctx, guild_id, mod_log_embed("🔨 Ban", RED, msg.author.id, *user_id, &reason)).await;
            }
            Err(e) => errors.push(format!("<@{}>: {}", user_id, e)),
        }
    }

    let mut text = String::new();
    if !success.is_empty() { text.push_str(&format!("✅ Bannis : {}\n**Raison :** {}", success.join(", "), reason)); }
    if !errors.is_empty() { text.push_str(&format!("\n❌ Erreurs : {}", errors.join(", "))); }
    let color = if errors.is_empty() { GREEN } else if success.is_empty() { RED } else { 0xF39C12 };
    send_embed(ctx, msg, color, &text).await;
    Ok(())
}

#[command]
#[required_permissions(BAN_MEMBERS)]
#[description("Débannir un membre du serveur")]
#[usage("<user_id> [raison]")]
async fn unban(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let (user_ids, _) = parse_users_and_rest(&mut args);

    if user_ids.is_empty() {
        error_embed(ctx, msg, "❌ Usage: `!unban <id1> [<id2> ...]`").await;
        return Ok(());
    }

    let mut success = Vec::new();
    let mut errors = Vec::new();

    for user_id in &user_ids {
        match guild_id.unban(&ctx.http, *user_id).await {
            Ok(_) => {
                success.push(format!("<@{}>", user_id));
                send_mod_log(ctx, guild_id, mod_log_embed("🔓 Unban", GREEN, msg.author.id, *user_id, "—")).await;
            }
            Err(e) => errors.push(format!("<@{}>: {}", user_id, e)),
        }
    }

    let mut text = String::new();
    if !success.is_empty() { text.push_str(&format!("✅ Débannis : {}", success.join(", "))); }
    if !errors.is_empty() { text.push_str(&format!("\n❌ Erreurs : {}", errors.join(", "))); }
    let color = if errors.is_empty() { GREEN } else if success.is_empty() { RED } else { 0xF39C12 };
    send_embed(ctx, msg, color, &text).await;
    Ok(())
}

#[command]
#[required_permissions(KICK_MEMBERS)]
#[description("Expulser un membre du serveur")]
#[usage("@utilisateur [raison]")]
async fn kick(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let (user_ids, reason) = parse_users_and_rest(&mut args);
    let reason = if reason.is_empty() { "Aucune raison fournie".to_string() } else { reason };

    if user_ids.is_empty() {
        error_embed(ctx, msg, "❌ Usage: `!kick @user1 [@user2 ...] [raison]`").await;
        return Ok(());
    }

    let mut success = Vec::new();
    let mut errors = Vec::new();

    for user_id in &user_ids {
        match guild_id.kick_with_reason(&ctx.http, *user_id, &reason).await {
            Ok(_) => {
                success.push(format!("<@{}>", user_id));
                send_mod_log(ctx, guild_id, mod_log_embed("👢 Kick", 0xE67E22, msg.author.id, *user_id, &reason)).await;
            }
            Err(e) => errors.push(format!("<@{}>: {}", user_id, e)),
        }
    }

    let mut text = String::new();
    if !success.is_empty() { text.push_str(&format!("✅ Expulsés : {}\n**Raison :** {}", success.join(", "), reason)); }
    if !errors.is_empty() { text.push_str(&format!("\n❌ Erreurs : {}", errors.join(", "))); }
    let color = if errors.is_empty() { GREEN } else if success.is_empty() { RED } else { 0xF39C12 };
    send_embed(ctx, msg, color, &text).await;
    Ok(())
}

#[command]
#[required_permissions(MODERATE_MEMBERS)]
#[description("Rendre muet un membre (timeout Discord)")]
#[usage("@utilisateur <durée> [raison]  — durée: 10m, 2h, 1d")]
async fn mute(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let (user_ids, rest) = parse_users_and_rest(&mut args);

    if user_ids.is_empty() {
        error_embed(ctx, msg, "❌ Usage: `!mute @user1 [@user2 ...] <durée> [raison]`").await;
        return Ok(());
    }

    // Le premier mot du reste est la durée
    let mut rest_parts = rest.splitn(2, ' ');
    let dur_str = match rest_parts.next().filter(|s| !s.is_empty()) {
        Some(s) => s.to_string(),
        None => {
            error_embed(ctx, msg, "❌ Durée manquante. Exemple: `10m`, `2h`, `1d`").await;
            return Ok(());
        }
    };
    let duration = match parse_duration(&dur_str) {
        Some(d) => d,
        None => {
            error_embed(ctx, msg, "❌ Durée invalide. Utilisez `10m`, `2h`, `1d`...").await;
            return Ok(());
        }
    };
    let reason = rest_parts.next().unwrap_or("").trim();
    let reason = if reason.is_empty() { "Aucune raison fournie".to_string() } else { reason.to_string() };

    let until = Utc::now() + duration;
    let ts = match serenity::model::Timestamp::from_unix_timestamp(until.timestamp()) {
        Ok(t) => t,
        Err(_) => {
            error_embed(ctx, msg, "❌ Timestamp invalide.").await;
            return Ok(());
        }
    };

    let mut success = Vec::new();
    let mut errors = Vec::new();

    for user_id in &user_ids {
        match guild_id
            .edit_member(&ctx.http, *user_id, EditMember::new().disable_communication_until_datetime(ts))
            .await
        {
            Ok(_) => {
                success.push(format!("<@{}>", user_id));
                send_mod_log(ctx, guild_id, mod_log_embed("🔇 Mute", 0x9B59B6, msg.author.id, *user_id, &format!("{} — {}", dur_str, reason))).await;
            }
            Err(e) => errors.push(format!("<@{}>: {}", user_id, e)),
        }
    }

    let mut text = String::new();
    if !success.is_empty() { text.push_str(&format!("🔇 Mutés **{}** : {}\n**Raison :** {}", dur_str, success.join(", "), reason)); }
    if !errors.is_empty() { text.push_str(&format!("\n❌ Erreurs : {}", errors.join(", "))); }
    let color = if errors.is_empty() { GREEN } else if success.is_empty() { RED } else { 0xF39C12 };
    send_embed(ctx, msg, color, &text).await;
    Ok(())
}

#[command]
#[required_permissions(MODERATE_MEMBERS)]
#[description("Retirer le mute d'un membre")]
#[usage("@utilisateur")]
async fn unmute(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();

    let raw = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!unmute @utilisateur`").await;
            return Ok(());
        }
    };
    let user_id = match parse_user_id(&raw) {
        Some(id) => id,
        None => {
            error_embed(ctx, msg, "❌ Utilisateur introuvable.").await;
            return Ok(());
        }
    };

    // Set timeout to past to clear it
    let past = serenity::model::Timestamp::from_unix_timestamp(0).unwrap();
    match guild_id
        .edit_member(
            &ctx.http,
            user_id,
            EditMember::new().disable_communication_until_datetime(past),
        )
        .await
    {
        Ok(_) => {
            success_embed(
                ctx,
                msg,
                &format!("🔊 <@{}> n'est plus **muté**.", user_id),
            )
            .await;
            send_mod_log(ctx, guild_id, mod_log_embed("🔊 Unmute", GREEN, msg.author.id, user_id, "—")).await;
        }
        Err(e) => {
            error_embed(ctx, msg, &format!("❌ Impossible de démuter: {}", e)).await;
        }
    }
    Ok(())
}

#[command]
#[required_permissions(MANAGE_MESSAGES)]
#[description("Avertir un membre")]
#[usage("@utilisateur <raison>")]
async fn warn(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = get_pool(ctx).await;
    let (user_ids, reason) = parse_users_and_rest(&mut args);

    if user_ids.is_empty() {
        error_embed(ctx, msg, "❌ Usage: `!warn @user1 [@user2 ...] <raison>`").await;
        return Ok(());
    }
    if reason.is_empty() {
        error_embed(ctx, msg, "❌ Raison manquante.").await;
        return Ok(());
    }

    let now = Utc::now().timestamp();
    let mut lines = Vec::new();

    for user_id in &user_ids {
        let _ = sqlx::query(
            "INSERT INTO warnings (guild_id, user_id, moderator_id, reason, created_at) VALUES (?, ?, ?, ?, ?)",
        )
        .bind(guild_id.get().to_string())
        .bind(user_id.get().to_string())
        .bind(msg.author.id.get().to_string())
        .bind(&reason)
        .bind(now)
        .execute(&pool)
        .await;

        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM warnings WHERE guild_id = ? AND user_id = ?",
        )
        .bind(guild_id.get().to_string())
        .bind(user_id.get().to_string())
        .fetch_one(&pool)
        .await
        .unwrap_or((0,));

        lines.push(format!("⚠️ <@{}> (total: {})", user_id, count.0));
        send_mod_log(ctx, guild_id, mod_log_embed("⚠️ Warn", 0xF39C12, msg.author.id, *user_id, &reason)).await;
    }

    send_embed(ctx, msg, 0xF39C12, &format!("{}\n**Raison :** {}", lines.join("\n"), reason)).await;
    Ok(())
}

#[command]
#[required_permissions(MANAGE_MESSAGES)]
#[description("Voir les avertissements d'un membre")]
#[usage("@utilisateur")]
async fn warnings(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = get_pool(ctx).await;

    let raw = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!warnings @utilisateur`").await;
            return Ok(());
        }
    };
    let user_id = match parse_user_id(&raw) {
        Some(id) => id,
        None => {
            error_embed(ctx, msg, "❌ Utilisateur introuvable.").await;
            return Ok(());
        }
    };

    let warns: Vec<Warning> = sqlx::query_as(
        "SELECT * FROM warnings WHERE guild_id = ? AND user_id = ? ORDER BY created_at DESC",
    )
    .bind(guild_id.get().to_string())
    .bind(user_id.get().to_string())
    .fetch_all(&pool)
    .await?;

    if warns.is_empty() {
        success_embed(ctx, msg, &format!("✅ <@{}> n'a aucun avertissement.", user_id)).await;
        return Ok(());
    }

    let list = warns
        .iter()
        .enumerate()
        .map(|(i, w)| {
            format!(
                "`#{}` — {} — par <@{}>",
                i + 1,
                w.reason,
                w.moderator_id
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let _ = msg
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .color(0xF39C12)
                    .title(format!("Avertissements de <@{}>", user_id))
                    .description(list),
            ),
        )
        .await;
    Ok(())
}

#[command]
#[required_permissions(ADMINISTRATOR)]
#[description("Effacer tous les avertissements d'un membre")]
#[usage("@utilisateur")]
async fn clearwarns(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = get_pool(ctx).await;

    let raw = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!clearwarns @utilisateur`").await;
            return Ok(());
        }
    };
    let user_id = match parse_user_id(&raw) {
        Some(id) => id,
        None => {
            error_embed(ctx, msg, "❌ Utilisateur introuvable.").await;
            return Ok(());
        }
    };

    let result = sqlx::query(
        "DELETE FROM warnings WHERE guild_id = ? AND user_id = ?",
    )
    .bind(guild_id.get().to_string())
    .bind(user_id.get().to_string())
    .execute(&pool)
    .await?;

    success_embed(
        ctx,
        msg,
        &format!(
            "🗑️ {} avertissement(s) supprimé(s) pour <@{}>.",
            result.rows_affected(),
            user_id
        ),
    )
    .await;
    Ok(())
}

#[command]
#[required_permissions(MANAGE_MESSAGES)]
#[description("Supprimer des messages")]
#[usage("<nombre> (1-100)")]
async fn clear(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let amount: u8 = match args.single::<u8>() {
        Ok(n) if n >= 1 && n <= 100 => n,
        _ => {
            error_embed(ctx, msg, "❌ Usage: `!clear <nombre>` (entre 1 et 100)").await;
            return Ok(());
        }
    };

    msg.delete(&ctx.http).await?;

    let messages = msg
        .channel_id
        .messages(&ctx.http, GetMessages::new().limit(amount))
        .await?;

    if messages.is_empty() {
        return Ok(());
    }

    let ids: Vec<MessageId> = messages.iter().map(|m| m.id).collect();

    if ids.len() == 1 {
        msg.channel_id.delete_message(&ctx.http, ids[0]).await?;
    } else {
        msg.channel_id.delete_messages(&ctx.http, &ids).await?;
    }

    let confirmation = msg
        .channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .color(GREEN)
                    .description(format!("🗑️ {} message(s) supprimé(s).", ids.len())),
            ),
        )
        .await?;

    tokio::time::sleep(std::time::Duration::from_secs(3)).await;
    let _ = confirmation.delete(&ctx.http).await;

    Ok(())
}
