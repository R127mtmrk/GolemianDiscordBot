use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::model::channel::{PermissionOverwrite, PermissionOverwriteType};
use serenity::model::id::{ChannelId, RoleId, UserId};
use serenity::model::permissions::Permissions;
use serenity::model::prelude::*;
use serenity::prelude::*;

use crate::database::DatabaseKey;
use crate::ticket::{close_ticket, open_button};

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

fn parse_role_mention(s: &str) -> Option<RoleId> {
    let s = s.trim();
    let id_str = if s.starts_with("<@&") && s.ends_with('>') {
        &s[3..s.len() - 1]
    } else {
        s
    };
    id_str.parse::<u64>().ok().map(RoleId::new)
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
#[commands(setticket, ticketpanel, delticket, tclose, tadd, tremove)]
#[only_in(guilds)]
pub struct Ticket;

#[command]
#[required_permissions(MANAGE_GUILD)]
#[description("Configurer le système de tickets")]
#[usage("#catégorie [@rôle_support] [#salon_logs]")]
async fn setticket(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().unwrap().clone()
    };

    let cat_raw = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(
                ctx,
                msg,
                "❌ Usage: `!setticket #catégorie [@rôle_support] [#salon_logs]`",
            )
            .await;
            return Ok(());
        }
    };
    let category_id = match parse_channel_mention(&cat_raw) {
        Some(id) => id,
        None => {
            error_embed(ctx, msg, "❌ Catégorie invalide.").await;
            return Ok(());
        }
    };

    let mut support_role: Option<RoleId> = None;
    let mut log_channel: Option<ChannelId> = None;

    while let Ok(raw) = args.single::<String>() {
        if let Some(r) = parse_role_mention(&raw) {
            support_role = Some(r);
        } else if let Some(c) = parse_channel_mention(&raw) {
            log_channel = Some(c);
        }
    }

    sqlx::query(
        "INSERT INTO ticket_config (guild_id, category_id, support_role_id, log_channel_id, ticket_counter)
         VALUES (?, ?, ?, ?, COALESCE((SELECT ticket_counter FROM ticket_config WHERE guild_id = ?), 0))
         ON CONFLICT(guild_id) DO UPDATE SET
            category_id = excluded.category_id,
            support_role_id = excluded.support_role_id,
            log_channel_id = excluded.log_channel_id",
    )
    .bind(guild_id.get().to_string())
    .bind(category_id.get().to_string())
    .bind(support_role.map(|r| r.get().to_string()))
    .bind(log_channel.map(|c| c.get().to_string()))
    .bind(guild_id.get().to_string())
    .execute(&pool)
    .await?;

    let mut text = format!("✅ Système de tickets configuré.\n• Catégorie : <#{}>", category_id);
    if let Some(r) = support_role {
        text.push_str(&format!("\n• Rôle support : <@&{}>", r));
    }
    if let Some(c) = log_channel {
        text.push_str(&format!("\n• Salon logs : <#{}>", c));
    }
    success_embed(ctx, msg, &text).await;
    Ok(())
}

#[command]
#[required_permissions(MANAGE_GUILD)]
#[description("Envoyer le panneau de création de ticket dans un salon")]
#[usage("[#salon]")]
async fn ticketpanel(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().unwrap().clone()
    };

    let exists: Option<(String,)> = sqlx::query_as(
        "SELECT category_id FROM ticket_config WHERE guild_id = ?",
    )
    .bind(guild_id.get().to_string())
    .fetch_optional(&pool)
    .await?;

    if exists.is_none() {
        error_embed(
            ctx,
            msg,
            "❌ Configure d'abord le système avec `!setticket #catégorie`.",
        )
        .await;
        return Ok(());
    }

    let target_channel = if let Ok(raw) = args.single::<String>() {
        parse_channel_mention(&raw).unwrap_or(msg.channel_id)
    } else {
        msg.channel_id
    };

    let embed = CreateEmbed::new()
        .color(BLUE)
        .title("🎫 Support — Ouvrir un ticket")
        .description(
            "Besoin d'aide ? Clique sur le bouton ci-dessous pour ouvrir un ticket privé avec l'équipe de support.\n\n\
             Un seul ticket à la fois par utilisateur.",
        );

    target_channel
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(embed).components(vec![open_button()]),
        )
        .await?;

    if target_channel != msg.channel_id {
        success_embed(ctx, msg, &format!("✅ Panneau envoyé dans <#{}>.", target_channel)).await;
    }
    let _ = msg.delete(&ctx.http).await;
    Ok(())
}

#[command]
#[required_permissions(MANAGE_GUILD)]
#[description("Désactiver le système de tickets")]
async fn delticket(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().unwrap().clone()
    };

    sqlx::query("DELETE FROM ticket_config WHERE guild_id = ?")
        .bind(guild_id.get().to_string())
        .execute(&pool)
        .await?;

    success_embed(ctx, msg, "✅ Système de tickets **désactivé**.").await;
    Ok(())
}

#[command]
#[description("Fermer le ticket actuel")]
async fn tclose(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().unwrap().clone()
    };

    let ticket: Option<(String, i64)> = sqlx::query_as(
        "SELECT user_id, ticket_number FROM tickets WHERE channel_id = ?",
    )
    .bind(msg.channel_id.get().to_string())
    .fetch_optional(&pool)
    .await?;

    let (owner_id_str, number) = match ticket {
        Some(t) => t,
        None => {
            error_embed(ctx, msg, "❌ Cette commande doit être utilisée dans un ticket.").await;
            return Ok(());
        }
    };

    // Le propriétaire ou un membre avec MANAGE_CHANNELS peut fermer
    let is_owner = owner_id_str == msg.author.id.get().to_string();
    let has_perm = match msg.member(&ctx.http).await {
        Ok(m) => m
            .permissions(&ctx.cache)
            .map(|p| p.manage_channels())
            .unwrap_or(false),
        Err(_) => false,
    };

    if !is_owner && !has_perm {
        error_embed(ctx, msg, "❌ Tu n'as pas la permission de fermer ce ticket.").await;
        return Ok(());
    }

    msg.channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .color(RED)
                    .description(format!(
                        "🔒 Ticket fermé par <@{}>. Suppression dans 5 secondes...",
                        msg.author.id
                    )),
            ),
        )
        .await?;

    close_ticket(
        ctx,
        &pool,
        guild_id,
        msg.channel_id,
        &owner_id_str,
        number,
        msg.author.id,
    )
    .await;
    Ok(())
}

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("Ajouter un utilisateur au ticket actuel")]
#[usage("@utilisateur")]
async fn tadd(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().unwrap().clone()
    };

    let exists: Option<(String,)> =
        sqlx::query_as("SELECT user_id FROM tickets WHERE channel_id = ?")
            .bind(msg.channel_id.get().to_string())
            .fetch_optional(&pool)
            .await?;

    if exists.is_none() {
        error_embed(ctx, msg, "❌ Cette commande doit être utilisée dans un ticket.").await;
        return Ok(());
    }

    let raw = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!tadd @utilisateur`").await;
            return Ok(());
        }
    };
    let user_id = match parse_user_id(&raw) {
        Some(id) => id,
        None => {
            error_embed(ctx, msg, "❌ Utilisateur invalide.").await;
            return Ok(());
        }
    };

    let overwrite = PermissionOverwrite {
        allow: Permissions::VIEW_CHANNEL
            | Permissions::SEND_MESSAGES
            | Permissions::READ_MESSAGE_HISTORY
            | Permissions::ATTACH_FILES
            | Permissions::EMBED_LINKS,
        deny: Permissions::empty(),
        kind: PermissionOverwriteType::Member(user_id),
    };

    match msg.channel_id.create_permission(&ctx.http, overwrite).await {
        Ok(_) => success_embed(ctx, msg, &format!("✅ <@{}> a été ajouté au ticket.", user_id)).await,
        Err(e) => error_embed(ctx, msg, &format!("❌ Erreur: {}", e)).await,
    }
    Ok(())
}

#[command]
#[required_permissions(MANAGE_CHANNELS)]
#[description("Retirer un utilisateur du ticket actuel")]
#[usage("@utilisateur")]
async fn tremove(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let pool = {
        let data = ctx.data.read().await;
        data.get::<DatabaseKey>().unwrap().clone()
    };

    let owner: Option<(String,)> =
        sqlx::query_as("SELECT user_id FROM tickets WHERE channel_id = ?")
            .bind(msg.channel_id.get().to_string())
            .fetch_optional(&pool)
            .await?;

    let owner_id = match owner {
        Some((o,)) => o,
        None => {
            error_embed(ctx, msg, "❌ Cette commande doit être utilisée dans un ticket.").await;
            return Ok(());
        }
    };

    let raw = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!tremove @utilisateur`").await;
            return Ok(());
        }
    };
    let user_id = match parse_user_id(&raw) {
        Some(id) => id,
        None => {
            error_embed(ctx, msg, "❌ Utilisateur invalide.").await;
            return Ok(());
        }
    };

    if user_id.get().to_string() == owner_id {
        error_embed(ctx, msg, "❌ Tu ne peux pas retirer le créateur du ticket.").await;
        return Ok(());
    }

    match msg
        .channel_id
        .delete_permission(&ctx.http, PermissionOverwriteType::Member(user_id))
        .await
    {
        Ok(_) => success_embed(ctx, msg, &format!("✅ <@{}> retiré du ticket.", user_id)).await,
        Err(e) => error_embed(ctx, msg, &format!("❌ Erreur: {}", e)).await,
    }
    Ok(())
}
