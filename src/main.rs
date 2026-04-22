mod commands;
mod database;

use std::sync::Arc;

use serenity::async_trait;
use serenity::framework::standard::{Configuration, StandardFramework};
use serenity::model::channel::Reaction;
use serenity::model::gateway::Ready;
use serenity::prelude::*;
use tracing::{error, info};

use commands::giveaway::{check_giveaways, GIVEAWAY_GROUP};
use commands::help::HELP_GROUP;
use commands::moderation::MODERATION_GROUP;
use commands::poll::POLL_GROUP;
use database::DatabaseKey;

struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, ready: Ready) {
        info!("{} est connecté et prêt!", ready.user.name);

        let pool = {
            let data = ctx.data.read().await;
            data.get::<DatabaseKey>().unwrap().clone()
        };
        let http = Arc::clone(&ctx.http);

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(30)).await;
                if let Err(e) = check_giveaways(&http, &pool).await {
                    error!("Erreur tâche giveaway: {}", e);
                }
            }
        });
    }

    async fn reaction_add(&self, ctx: Context, reaction: Reaction) {
        if !reaction.emoji.unicode_eq("🎉") {
            return;
        }
        let uid = match reaction.user_id {
            Some(id) => id,
            None => return,
        };
        if uid == ctx.cache.current_user().id {
            return;
        }

        let pool = {
            let data = ctx.data.read().await;
            data.get::<DatabaseKey>().unwrap().clone()
        };
        let mid = reaction.message_id.get().to_string();

        if let Ok(ga) = sqlx::query_as::<_, database::Giveaway>(
            "SELECT * FROM giveaways WHERE message_id = ? AND ended = 0",
        )
        .bind(&mid)
        .fetch_one(&pool)
        .await
        {
            let _ = sqlx::query(
                "INSERT OR IGNORE INTO giveaway_entries (giveaway_id, user_id) VALUES (?, ?)",
            )
            .bind(ga.id)
            .bind(uid.get().to_string())
            .execute(&pool)
            .await;
        }
    }

    async fn reaction_remove(&self, ctx: Context, reaction: Reaction) {
        if !reaction.emoji.unicode_eq("🎉") {
            return;
        }
        let uid = match reaction.user_id {
            Some(id) => id,
            None => return,
        };

        let pool = {
            let data = ctx.data.read().await;
            data.get::<DatabaseKey>().unwrap().clone()
        };
        let mid = reaction.message_id.get().to_string();

        if let Ok(ga) = sqlx::query_as::<_, database::Giveaway>(
            "SELECT * FROM giveaways WHERE message_id = ? AND ended = 0",
        )
        .bind(&mid)
        .fetch_one(&pool)
        .await
        {
            let _ = sqlx::query(
                "DELETE FROM giveaway_entries WHERE giveaway_id = ? AND user_id = ?",
            )
            .bind(ga.id)
            .bind(uid.get().to_string())
            .execute(&pool)
            .await;
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::fmt()
        .with_env_filter("GolemianDiscordBot=info,serenity=warn")
        .init();

    let token = std::env::var("DISCORD_TOKEN").expect("DISCORD_TOKEN manquant dans .env");

    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite:bot.db".to_string());
    let pool = database::create_pool(&db_url).await?;

    let framework = StandardFramework::new()
        .group(&MODERATION_GROUP)
        .group(&GIVEAWAY_GROUP)
        .group(&POLL_GROUP)
        .group(&HELP_GROUP);

    framework.configure(Configuration::new().prefix("!").allow_dm(false));

    let intents = GatewayIntents::GUILD_MESSAGES
        | GatewayIntents::MESSAGE_CONTENT
        | GatewayIntents::GUILD_MESSAGE_REACTIONS;

    let mut client = Client::builder(&token, intents)
        .event_handler(Handler)
        .framework(framework)
        .await?;

    {
        let mut data = client.data.write().await;
        data.insert::<DatabaseKey>(pool);
    }

    client.start().await?;
    Ok(())
}
