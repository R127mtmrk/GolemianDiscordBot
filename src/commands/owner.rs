use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::model::id::{GuildId, UserId};
use serenity::model::prelude::*;
use serenity::prelude::*;

const BLUE: u32 = 0x5865F2;
const GREEN: u32 = 0x2ECC71;
const RED: u32 = 0xE74C3C;

// Permissions par défaut demandées dans le lien d'invitation (Administrator).
// Pratique parce que le bot fait de la modération, des tickets, des salons vocaux, etc.
const INVITE_PERMISSIONS: u64 = 0x8;

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

async fn bot_owner_id(ctx: &Context) -> Option<UserId> {
    let info = ctx.http.get_current_application_info().await.ok()?;
    info.owner.as_ref().map(|u| u.id)
}

#[group]
#[commands(invite, servers, leave)]
pub struct Owner;

#[command]
#[description("Afficher le lien d'invitation OAuth2 du bot")]
async fn invite(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let bot_id = ctx.cache.current_user().id;
    let url = format!(
        "https://discord.com/api/oauth2/authorize?client_id={}&permissions={}&scope=bot%20applications.commands",
        bot_id, INVITE_PERMISSIONS
    );

    let embed = CreateEmbed::new()
        .color(BLUE)
        .title("🔗 Inviter Golemian Bot")
        .description(format!(
            "Clique sur le lien ci-dessous pour ajouter le bot à un serveur.\n\
             Tu dois posséder la permission **Gérer le serveur** sur la cible.\n\n\
             [Lien d'invitation]({})",
            url
        ))
        .field("URL brute", format!("`{}`", url), false);

    msg.channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await?;
    Ok(())
}

#[command]
#[description("Lister les serveurs où le bot est présent (propriétaire du bot uniquement)")]
async fn servers(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let owner = bot_owner_id(ctx).await;
    if owner != Some(msg.author.id) {
        error_embed(ctx, msg, "❌ Commande réservée au propriétaire du bot.").await;
        return Ok(());
    }

    let guild_ids: Vec<GuildId> = ctx.cache.guilds();
    if guild_ids.is_empty() {
        error_embed(ctx, msg, "Le bot n'est sur aucun serveur.").await;
        return Ok(());
    }

    let mut entries: Vec<(String, u64, u64)> = Vec::with_capacity(guild_ids.len());
    for gid in &guild_ids {
        if let Some(g) = ctx.cache.guild(*gid) {
            entries.push((g.name.clone(), gid.get(), g.member_count));
        } else {
            entries.push(("(inconnu — pas en cache)".to_string(), gid.get(), 0));
        }
    }
    entries.sort_by(|a, b| b.2.cmp(&a.2));

    let total = entries.len();
    let mut description = String::new();
    for (i, (name, id, members)) in entries.iter().enumerate() {
        let line = format!("**{}.** `{}` — {} ({} membres)\n", i + 1, id, name, members);
        // Embed description limit ~4096 chars
        if description.len() + line.len() > 3900 {
            description.push_str(&format!("…et {} de plus.", total - i));
            break;
        }
        description.push_str(&line);
    }

    let embed = CreateEmbed::new()
        .color(BLUE)
        .title(format!("🌐 Serveurs ({})", total))
        .description(description);

    msg.channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await?;
    Ok(())
}

#[command]
#[description("Faire quitter le bot d'un serveur par son ID (propriétaire du bot uniquement)")]
#[usage("<guild_id>")]
async fn leave(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let owner = bot_owner_id(ctx).await;
    if owner != Some(msg.author.id) {
        error_embed(ctx, msg, "❌ Commande réservée au propriétaire du bot.").await;
        return Ok(());
    }

    let id_str = match args.single::<String>() {
        Ok(s) => s,
        Err(_) => {
            error_embed(ctx, msg, "❌ Usage: `!leave <guild_id>`").await;
            return Ok(());
        }
    };

    let guild_id = match id_str.parse::<u64>() {
        Ok(id) => GuildId::new(id),
        Err(_) => {
            error_embed(ctx, msg, "❌ ID de serveur invalide.").await;
            return Ok(());
        }
    };

    let name = ctx.cache.guild(guild_id).map(|g| g.name.clone());

    match ctx.http.leave_guild(guild_id).await {
        Ok(_) => {
            let label = name.unwrap_or_else(|| format!("`{}`", guild_id));
            success_embed(ctx, msg, &format!("✅ Bot retiré du serveur **{}**.", label)).await;
        }
        Err(e) => {
            error_embed(ctx, msg, &format!("❌ Impossible de quitter le serveur : {}", e)).await;
        }
    }
    Ok(())
}
