use serenity::builder::{CreateEmbed, CreateMessage};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

const BLUE: u32 = 0x3498DB;
const RED: u32 = 0xE74C3C;

const LETTER_EMOJIS: &[&str] = &[
    "🇦", "🇧", "🇨", "🇩", "🇪", "🇫", "🇬", "🇭", "🇮",
];

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
#[commands(poll)]
#[only_in(guilds)]
pub struct Poll;

#[command]
#[description("Créer un sondage avec réactions")]
#[usage("<question> | <choix1> | <choix2> ...  (ou juste une question pour Oui/Non)")]
async fn poll(ctx: &Context, msg: &Message, args: Args) -> CommandResult {
    let full = args.rest().trim().to_string();
    if full.is_empty() {
        error_embed(ctx, msg, "❌ Usage: `!poll <question> | <choix1> | <choix2> ...`").await;
        return Ok(());
    }

    let parts: Vec<&str> = full.split('|').map(|s| s.trim()).collect();
    let question = parts[0];

    if question.is_empty() {
        error_embed(ctx, msg, "❌ La question ne peut pas être vide.").await;
        return Ok(());
    }

    if parts.len() == 1 {
        // Sondage Oui/Non
        let poll_msg = msg
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .color(BLUE)
                        .title("📊 Sondage")
                        .description(format!("**{}**\n\n✅ Oui\n❌ Non", question))
                        .footer(serenity::builder::CreateEmbedFooter::new(format!(
                            "Sondage créé par {}",
                            msg.author.name
                        ))),
                ),
            )
            .await?;

        poll_msg
            .react(&ctx.http, ReactionType::Unicode("✅".to_string()))
            .await?;
        poll_msg
            .react(&ctx.http, ReactionType::Unicode("❌".to_string()))
            .await?;
    } else {
        // Sondage à choix multiples
        let choices: Vec<&str> = parts[1..].to_vec();

        if choices.len() > LETTER_EMOJIS.len() {
            error_embed(
                ctx,
                msg,
                &format!("❌ Maximum {} choix autorisés.", LETTER_EMOJIS.len()),
            )
            .await;
            return Ok(());
        }

        let description = choices
            .iter()
            .enumerate()
            .map(|(i, c)| format!("{} {}", LETTER_EMOJIS[i], c))
            .collect::<Vec<_>>()
            .join("\n");

        let poll_msg = msg
            .channel_id
            .send_message(
                &ctx.http,
                CreateMessage::new().embed(
                    CreateEmbed::new()
                        .color(BLUE)
                        .title("📊 Sondage")
                        .description(format!("**{}**\n\n{}", question, description))
                        .footer(serenity::builder::CreateEmbedFooter::new(format!(
                            "Sondage créé par {}",
                            msg.author.name
                        ))),
                ),
            )
            .await?;

        for emoji in &LETTER_EMOJIS[..choices.len()] {
            poll_msg
                .react(&ctx.http, ReactionType::Unicode(emoji.to_string()))
                .await?;
        }
    }

    msg.delete(&ctx.http).await.ok();
    Ok(())
}
