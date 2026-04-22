use chrono::Utc;
use rand::seq::SliceRandom;
use serenity::builder::{CreateEmbed, CreateMessage, EditMessage};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::http::Http;
use serenity::model::id::{ChannelId, MessageId};
use serenity::model::prelude::*;
use serenity::prelude::*;
use sqlx::SqlitePool;
use std::sync::Arc;

use crate::database::{DatabaseKey, Giveaway as DbGiveaway};

const GOLD: u32 = 0xFFD700;
const RED: u32 = 0xE74C3C;
const GREEN: u32 = 0x2ECC71;
const GIVEAWAY_EMOJI: &str = "🎉";

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

pub async fn end_giveaway_by_id(
    http: &Arc<Http>,
    pool: &SqlitePool,
    giveaway: &DbGiveaway,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    sqlx::query("UPDATE giveaways SET ended = 1 WHERE id = ?")
        .bind(giveaway.id)
        .execute(pool)
        .await?;

    let entries: Vec<(String,)> =
        sqlx::query_as("SELECT user_id FROM giveaway_entries WHERE giveaway_id = ?")
            .bind(giveaway.id)
            .fetch_all(pool)
            .await?;

    let channel_id = ChannelId::new(giveaway.channel_id.parse::<u64>()?);
    let message_id = MessageId::new(giveaway.message_id.parse::<u64>()?);

    if entries.is_empty() {
        channel_id
            .send_message(
                http,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .color(RED)
                        .title(format!("🎉 Giveaway terminé — {}", giveaway.prize))
                        .description("Personne n'a participé à ce giveaway."),
                ),
            )
            .await?;
        return Ok(());
    }

    let winner_count = giveaway.winner_count as usize;
    let winners: Vec<String> = {
        let mut rng = rand::thread_rng();
        entries
            .choose_multiple(&mut rng, winner_count.min(entries.len()))
            .map(|(uid,)| format!("<@{}>", uid))
            .collect()
    };

    let winners_str = winners.join(", ");

    let _ = channel_id
        .edit_message(
            http,
            message_id,
            EditMessage::new().embed(
                CreateEmbed::new()
                    .color(GREEN)
                    .title("🎉 GIVEAWAY TERMINÉ 🎉")
                    .description(format!(
                        "**Prix :** {}\n**Gagnant(s) :** {}\n\nGiveaway terminé.",
                        giveaway.prize, winners_str
                    )),
            ),
        )
        .await;

    channel_id
        .send_message(
            http,
            CreateMessage::new()
                .content(format!(
                    "🎊 Félicitations {} ! Vous avez gagné **{}** !",
                    winners_str, giveaway.prize
                ))
                .embed(
                    CreateEmbed::new()
                        .color(GOLD)
                        .title(format!("🎉 Résultat du giveaway — {}", giveaway.prize))
                        .description(format!("Gagnant(s) : {}", winners_str)),
                ),
        )
        .await?;

    Ok(())
}

pub async fn check_giveaways(
    http: &Arc<Http>,
    pool: &SqlitePool,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let now = Utc::now().timestamp();
    let expired: Vec<DbGiveaway> =
        sqlx::query_as("SELECT * FROM giveaways WHERE ended = 0 AND end_time <= ?")
            .bind(now)
            .fetch_all(pool)
            .await?;

    for ga in &expired {
        if let Err(e) = end_giveaway_by_id(http, pool, ga).await {
            tracing::error!("Erreur fin giveaway {}: {}", ga.id, e);
        }
    }
    Ok(())
}

#[group]
#[commands(gcreate, gend, greroll, glist)]
#[only_in(guilds)]
pub struct Giveaway;

#[command]
#[required_permissions(MANAGE_GUILD)]
#[description("Créer un giveaway")]
#[usage("<durée> <nb_gagnants> <prix...>  — ex: !gcreate 1h 2 Un Nitro")]
async fn gcreate(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = get_pool(ctx).await;

    let dur_str = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!gcreate <durée> <gagnants> <prix>`").await;
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

    let winner_count = match args.single::<i64>() {
        Ok(n) if n >= 1 => n,
        _ => {
            error_embed(ctx, msg, "❌ Nombre de gagnants invalide (minimum 1).").await;
            return Ok(());
        }
    };

    let prize = args.rest().trim().to_string();
    if prize.is_empty() {
        error_embed(ctx, msg, "❌ Prix manquant.").await;
        return Ok(());
    }

    let end_time = (Utc::now() + duration).timestamp();

    let embed = CreateEmbed::new()
        .color(GOLD)
        .title("🎉 GIVEAWAY 🎉")
        .description(format!(
            "Réagis avec {} pour participer !\n\n**Prix :** {}\n**Gagnant(s) :** {}\n**Fin :** <t:{}:R>",
            GIVEAWAY_EMOJI, prize, winner_count, end_time
        ))
        .footer(serenity::builder::CreateEmbedFooter::new(format!(
            "Fin le"
        )))
        .timestamp(serenity::model::Timestamp::from_unix_timestamp(end_time).unwrap());

    let ga_msg = msg
        .channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await?;

    ga_msg
        .react(
            &ctx.http,
            ReactionType::Unicode(GIVEAWAY_EMOJI.to_string()),
        )
        .await?;

    sqlx::query(
        "INSERT INTO giveaways (guild_id, channel_id, message_id, prize, end_time, winner_count, created_by) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(guild_id.get().to_string())
    .bind(msg.channel_id.get().to_string())
    .bind(ga_msg.id.get().to_string())
    .bind(&prize)
    .bind(end_time)
    .bind(winner_count)
    .bind(msg.author.id.get().to_string())
    .execute(&pool)
    .await?;

    msg.delete(&ctx.http).await.ok();
    Ok(())
}

#[command]
#[required_permissions(MANAGE_GUILD)]
#[description("Terminer immédiatement un giveaway")]
#[usage("<message_id>")]
async fn gend(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = get_pool(ctx).await;

    let raw = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!gend <message_id>`").await;
            return Ok(());
        }
    };

    let giveaway: Option<DbGiveaway> = sqlx::query_as(
        "SELECT * FROM giveaways WHERE guild_id = ? AND message_id = ? AND ended = 0",
    )
    .bind(guild_id.get().to_string())
    .bind(&raw)
    .fetch_optional(&pool)
    .await?;

    match giveaway {
        None => {
            error_embed(ctx, msg, "❌ Giveaway introuvable ou déjà terminé.").await;
        }
        Some(ga) => {
            let http = Arc::clone(&ctx.http);
            if let Err(e) = end_giveaway_by_id(&http, &pool, &ga).await {
                error_embed(ctx, msg, &format!("❌ Erreur: {}", e)).await;
            }
        }
    }
    Ok(())
}

#[command]
#[required_permissions(MANAGE_GUILD)]
#[description("Retirer au sort un nouveau gagnant")]
#[usage("<message_id>")]
async fn greroll(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = get_pool(ctx).await;

    let raw = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!greroll <message_id>`").await;
            return Ok(());
        }
    };

    let giveaway: Option<DbGiveaway> = sqlx::query_as(
        "SELECT * FROM giveaways WHERE guild_id = ? AND message_id = ?",
    )
    .bind(guild_id.get().to_string())
    .bind(&raw)
    .fetch_optional(&pool)
    .await?;

    let ga = match giveaway {
        None => {
            error_embed(ctx, msg, "❌ Giveaway introuvable.").await;
            return Ok(());
        }
        Some(g) => g,
    };

    let entries: Vec<(String,)> =
        sqlx::query_as("SELECT user_id FROM giveaway_entries WHERE giveaway_id = ?")
            .bind(ga.id)
            .fetch_all(&pool)
            .await?;

    if entries.is_empty() {
        error_embed(ctx, msg, "❌ Aucun participant pour ce giveaway.").await;
        return Ok(());
    }

    let winner_uid = {
        let mut rng = rand::thread_rng();
        entries.choose(&mut rng).unwrap().0.clone()
    };

    msg.channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new()
                .content(format!(
                    "🎊 Nouveau tirage ! Félicitations <@{}> ! Tu as gagné **{}** !",
                    winner_uid, ga.prize
                ))
                .embed(
                    CreateEmbed::new()
                        .color(GOLD)
                        .title("🔄 Reroll du giveaway")
                        .description(format!(
                            "Nouveau gagnant : <@{}>\nPrix : **{}**",
                            winner_uid, ga.prize
                        )),
                ),
        )
        .await?;

    Ok(())
}

#[command]
#[description("Voir les giveaways actifs")]
async fn glist(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let guild_id = msg.guild_id.unwrap();
    let pool = get_pool(ctx).await;

    let giveaways: Vec<DbGiveaway> =
        sqlx::query_as("SELECT * FROM giveaways WHERE guild_id = ? AND ended = 0 ORDER BY end_time ASC")
            .bind(guild_id.get().to_string())
            .fetch_all(&pool)
            .await?;

    if giveaways.is_empty() {
        msg.channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .color(0x95A5A6)
                        .description("Aucun giveaway actif en ce moment."),
                ),
            )
            .await?;
        return Ok(());
    }

    let list = giveaways
        .iter()
        .map(|g| {
            format!(
                "🎉 **{}** — {} gagnant(s) — fin <t:{}:R>\n└ Message ID: `{}`",
                g.prize, g.winner_count, g.end_time, g.message_id
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    msg.channel_id
        .send_message(
            &ctx.http,
            CreateMessage::new().embed(
                CreateEmbed::new()
                    .color(GOLD)
                    .title("🎉 Giveaways actifs")
                    .description(list),
            ),
        )
        .await?;

    Ok(())
}
