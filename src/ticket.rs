use crate::database::DatabaseKey;
use serenity::all::{
    ButtonStyle, ComponentInteraction, CreateActionRow, CreateButton, CreateChannel, CreateEmbed,
    CreateInteractionResponse, CreateInteractionResponseMessage, CreateMessage, Interaction,
    PermissionOverwrite, PermissionOverwriteType, Permissions,
};
use serenity::model::channel::ChannelType;
use serenity::model::id::{ChannelId, RoleId, UserId};
use serenity::prelude::Context;
use sqlx::SqlitePool;

const BLUE: u32 = 0x5865F2;
const GREEN: u32 = 0x2ECC71;
const RED: u32 = 0xE74C3C;

pub fn open_button() -> CreateActionRow {
    CreateActionRow::Buttons(vec![CreateButton::new("ticket_open")
        .label("Ouvrir un ticket")
        .style(ButtonStyle::Primary)
        .emoji('🎫')])
}

pub fn close_button() -> CreateActionRow {
    CreateActionRow::Buttons(vec![CreateButton::new("ticket_close")
        .label("Fermer le ticket")
        .style(ButtonStyle::Danger)
        .emoji('🔒')])
}

pub async fn handle_interaction(ctx: &Context, interaction: Interaction) {
    let component = match interaction {
        Interaction::Component(c) => c,
        _ => return,
    };

    match component.data.custom_id.as_str() {
        "ticket_open" => handle_open(ctx, &component).await,
        "ticket_close" => handle_close(ctx, &component).await,
        _ => {}
    }
}

async fn ephemeral_reply(ctx: &Context, comp: &ComponentInteraction, color: u32, text: &str) {
    let _ = comp
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new()
                    .ephemeral(true)
                    .embed(CreateEmbed::new().color(color).description(text)),
            ),
        )
        .await;
}

async fn handle_open(ctx: &Context, comp: &ComponentInteraction) {
    let guild_id = match comp.guild_id {
        Some(id) => id,
        None => return,
    };
    let user_id = comp.user.id;

    let pool = {
        let data = ctx.data.read().await;
        match data.get::<DatabaseKey>() {
            Some(p) => p.clone(),
            None => return,
        }
    };

    let config: Option<(String, Option<String>, Option<String>, i64)> = sqlx::query_as(
        "SELECT category_id, support_role_id, log_channel_id, ticket_counter
         FROM ticket_config WHERE guild_id = ?",
    )
    .bind(guild_id.get().to_string())
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    let (category_id_str, support_role_id, log_channel_id, counter) = match config {
        Some(c) => c,
        None => {
            ephemeral_reply(
                ctx,
                comp,
                RED,
                "❌ Le système de tickets n'est pas configuré.",
            )
            .await;
            return;
        }
    };

    // Vérifier si l'utilisateur a déjà un ticket ouvert
    let existing: Option<(String,)> = sqlx::query_as(
        "SELECT channel_id FROM tickets WHERE guild_id = ? AND user_id = ?",
    )
    .bind(guild_id.get().to_string())
    .bind(user_id.get().to_string())
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    if let Some((existing_channel,)) = existing {
        ephemeral_reply(
            ctx,
            comp,
            RED,
            &format!(
                "❌ Tu as déjà un ticket ouvert : <#{}>.",
                existing_channel
            ),
        )
        .await;
        return;
    }

    let category_id = match category_id_str.parse::<u64>().map(ChannelId::new) {
        Ok(id) => id,
        Err(_) => return,
    };

    let next_number = counter + 1;

    // Permissions
    let bot_id = ctx.cache.current_user().id;
    let mut overwrites = vec![
        PermissionOverwrite {
            allow: Permissions::empty(),
            deny: Permissions::VIEW_CHANNEL,
            kind: PermissionOverwriteType::Role(RoleId::new(guild_id.get())),
        },
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::READ_MESSAGE_HISTORY
                | Permissions::ATTACH_FILES
                | Permissions::EMBED_LINKS,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(user_id),
        },
        PermissionOverwrite {
            allow: Permissions::VIEW_CHANNEL
                | Permissions::SEND_MESSAGES
                | Permissions::READ_MESSAGE_HISTORY
                | Permissions::MANAGE_CHANNELS
                | Permissions::MANAGE_MESSAGES,
            deny: Permissions::empty(),
            kind: PermissionOverwriteType::Member(bot_id),
        },
    ];

    if let Some(role_str) = support_role_id.as_ref() {
        if let Ok(role_id) = role_str.parse::<u64>().map(RoleId::new) {
            overwrites.push(PermissionOverwrite {
                allow: Permissions::VIEW_CHANNEL
                    | Permissions::SEND_MESSAGES
                    | Permissions::READ_MESSAGE_HISTORY
                    | Permissions::ATTACH_FILES
                    | Permissions::EMBED_LINKS
                    | Permissions::MANAGE_MESSAGES,
                deny: Permissions::empty(),
                kind: PermissionOverwriteType::Role(role_id),
            });
        }
    }

    let channel_name = format!("ticket-{:04}", next_number);
    let builder = CreateChannel::new(&channel_name)
        .kind(ChannelType::Text)
        .category(category_id)
        .topic(format!("Ticket de {} (#{:04})", comp.user.name, next_number))
        .permissions(overwrites);

    let new_channel = match guild_id.create_channel(&ctx.http, builder).await {
        Ok(c) => c,
        Err(e) => {
            ephemeral_reply(
                ctx,
                comp,
                RED,
                &format!("❌ Impossible de créer le ticket : {}", e),
            )
            .await;
            return;
        }
    };

    let now = chrono::Utc::now().timestamp();
    let _ = sqlx::query(
        "INSERT INTO tickets (channel_id, guild_id, user_id, ticket_number, created_at)
         VALUES (?, ?, ?, ?, ?)",
    )
    .bind(new_channel.id.get().to_string())
    .bind(guild_id.get().to_string())
    .bind(user_id.get().to_string())
    .bind(next_number)
    .bind(now)
    .execute(&pool)
    .await;

    let _ = sqlx::query("UPDATE ticket_config SET ticket_counter = ? WHERE guild_id = ?")
        .bind(next_number)
        .bind(guild_id.get().to_string())
        .execute(&pool)
        .await;

    // Message de bienvenue dans le ticket
    let mut content = format!("<@{}>", user_id);
    if let Some(role_str) = support_role_id.as_ref() {
        if let Ok(role_id) = role_str.parse::<u64>() {
            content.push_str(&format!(" <@&{}>", role_id));
        }
    }

    let welcome = CreateEmbed::new()
        .color(BLUE)
        .title(format!("🎫 Ticket #{:04}", next_number))
        .description(format!(
            "Bonjour <@{}>, l'équipe de support va te répondre dès que possible.\nDécris ton problème en détail.\n\nClique sur le bouton ci-dessous pour fermer ce ticket.",
            user_id
        ));

    let _ = new_channel
        .id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .content(content)
                .embed(welcome)
                .components(vec![close_button()]),
        )
        .await;

    // Log
    if let Some(log_str) = log_channel_id.as_ref() {
        if let Ok(log_id) = log_str.parse::<u64>().map(ChannelId::new) {
            let _ = log_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new().embed(
                        CreateEmbed::new()
                            .color(GREEN)
                            .title("🎫 Ticket ouvert")
                            .field("Numéro", format!("#{:04}", next_number), true)
                            .field("Utilisateur", format!("<@{}>", user_id), true)
                            .field("Salon", format!("<#{}>", new_channel.id), true)
                            .timestamp(serenity::model::Timestamp::now()),
                    ),
                )
                .await;
        }
    }

    ephemeral_reply(
        ctx,
        comp,
        GREEN,
        &format!("✅ Ticket créé : <#{}>", new_channel.id),
    )
    .await;
}

async fn handle_close(ctx: &Context, comp: &ComponentInteraction) {
    let guild_id = match comp.guild_id {
        Some(id) => id,
        None => return,
    };
    let channel_id = comp.channel_id;

    let pool = {
        let data = ctx.data.read().await;
        match data.get::<DatabaseKey>() {
            Some(p) => p.clone(),
            None => return,
        }
    };

    let ticket: Option<(String, i64)> = sqlx::query_as(
        "SELECT user_id, ticket_number FROM tickets WHERE channel_id = ?",
    )
    .bind(channel_id.get().to_string())
    .fetch_optional(&pool)
    .await
    .ok()
    .flatten();

    let (owner_id_str, number) = match ticket {
        Some(t) => t,
        None => {
            ephemeral_reply(ctx, comp, RED, "❌ Ce salon n'est pas un ticket.").await;
            return;
        }
    };

    let _ = comp
        .create_response(
            &ctx.http,
            CreateInteractionResponse::Message(
                CreateInteractionResponseMessage::new().embed(
                    CreateEmbed::new()
                        .color(RED)
                        .description(format!(
                            "🔒 Ticket fermé par <@{}>. Suppression dans 5 secondes...",
                            comp.user.id
                        )),
                ),
            ),
        )
        .await;

    close_ticket(ctx, &pool, guild_id, channel_id, &owner_id_str, number, comp.user.id).await;
}

pub async fn close_ticket(
    ctx: &Context,
    pool: &SqlitePool,
    guild_id: serenity::model::id::GuildId,
    channel_id: ChannelId,
    owner_id_str: &str,
    number: i64,
    closer_id: UserId,
) {
    // Log avant suppression
    let log_channel_id: Option<(Option<String>,)> = sqlx::query_as(
        "SELECT log_channel_id FROM ticket_config WHERE guild_id = ?",
    )
    .bind(guild_id.get().to_string())
    .fetch_optional(pool)
    .await
    .ok()
    .flatten();

    if let Some((Some(log_str),)) = log_channel_id {
        if let Ok(log_id) = log_str.parse::<u64>().map(ChannelId::new) {
            let _ = log_id
                .send_message(
                    &ctx.http,
                    CreateMessage::new().embed(
                        CreateEmbed::new()
                            .color(RED)
                            .title("🔒 Ticket fermé")
                            .field("Numéro", format!("#{:04}", number), true)
                            .field("Utilisateur", format!("<@{}>", owner_id_str), true)
                            .field("Fermé par", format!("<@{}>", closer_id), true)
                            .timestamp(serenity::model::Timestamp::now()),
                    ),
                )
                .await;
        }
    }

    let _ = sqlx::query("DELETE FROM tickets WHERE channel_id = ?")
        .bind(channel_id.get().to_string())
        .execute(pool)
        .await;

    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
    let _ = channel_id.delete(&ctx.http).await;
}
