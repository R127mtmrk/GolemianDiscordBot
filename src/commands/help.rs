use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

const BLUE: u32 = 0x5865F2;
const RED: u32 = 0xE74C3C;

struct Category {
    aliases: &'static [&'static str],
    title: &'static str,
    commands: &'static str,
}

const CATEGORIES: &[Category] = &[
    Category {
        aliases: &["moderation", "mod", "modération", "moderation"],
        title: "🛡️ Modération",
        commands: "`!ban @user [@user2 ...] [raison]` — Bannir (mentions multiples)\n\
                   `!unban <id> [raison]` — Débannir\n\
                   `!kick @user [@user2 ...] [raison]` — Expulser (mentions multiples)\n\
                   `!mute @user [@user2 ...] <durée> [raison]` — Timeout (`10m`, `2h`, `1d`)\n\
                   `!unmute @user [@user2 ...]` — Retirer le timeout\n\
                   `!warn @user [@user2 ...] <raison>` — Avertir (mentions multiples)\n\
                   `!warnings @user` — Voir les avertissements\n\
                   `!clearwarns @user` — Effacer les avertissements\n\
                   `!clear <1-100>` — Supprimer des messages",
    },
    Category {
        aliases: &["utility", "util", "utilitaires", "utilitaire"],
        title: "🔧 Utilitaires",
        commands: "`!slowmode <secondes>` — Mode lent (`0` pour désactiver)\n\
                   `!lock [#salon]` — Verrouiller un salon\n\
                   `!unlock [#salon]` — Déverrouiller un salon\n\
                   `!userinfo [@user]` — Infos d'un membre\n\
                   `!setmodlog #salon` — Définir le salon des logs de modération",
    },
    Category {
        aliases: &["server", "serveur", "structure", "backup"],
        title: "🗂️ Structure de serveur",
        commands: "`!sbackup` — Exporter rôles, catégories, salons et permissions en JSON\n\
                   `!srestore` — Recréer une structure depuis un fichier backup.json",
    },
    Category {
        aliases: &["giveaway", "giveaways", "gw", "concours"],
        title: "🎉 Giveaways",
        commands: "`!gcreate <durée> <gagnants> <prix>` — Créer un giveaway\n\
                   `!gend <message_id>` — Terminer un giveaway\n\
                   `!greroll <message_id>` — Nouveau tirage\n\
                   `!glist` — Voir les giveaways actifs",
    },
    Category {
        aliases: &["poll", "polls", "sondage", "sondages"],
        title: "📊 Sondages",
        commands: "`!poll <question>` — Sondage Oui/Non\n\
                   `!poll <question> | <choix1> | <choix2>` — Choix multiples (max 9)",
    },
    Category {
        aliases: &["ticket", "tickets"],
        title: "🎫 Tickets",
        commands: "`!setticket #catégorie [@rôle_support] [#salon_logs]` — Configurer\n\
                   `!ticketpanel [#salon]` — Envoyer le panneau à bouton\n\
                   `!delticket` — Désactiver le système\n\
                   `!tclose` — Fermer le ticket actuel\n\
                   `!tadd @user` — Ajouter un membre au ticket\n\
                   `!tremove @user` — Retirer un membre du ticket",
    },
    Category {
        aliases: &["tempvc", "vocal", "vocaux", "vc"],
        title: "🔊 Salons vocaux temporaires",
        commands: "`!settempvc #salon-vocal` — Définir le salon hub (rejoindre = créer un salon perso)\n\
                   `!deltempvc` — Désactiver le système",
    },
    Category {
        aliases: &["bot", "invite", "owner"],
        title: "🤖 Bot",
        commands: "`!invite` — Obtenir le lien d'invitation OAuth2 du bot\n\
                   `!servers` — Lister les serveurs où le bot est présent *(owner)*\n\
                   `!leave <guild_id>` — Faire quitter le bot d'un serveur *(owner)*",
    },
];

fn find_category(query: &str) -> Option<&'static Category> {
    let q = query.trim().to_lowercase();
    CATEGORIES
        .iter()
        .find(|c| c.aliases.iter().any(|a| a.to_lowercase() == q))
}

#[group]
#[commands(aide)]
#[only_in(guilds)]
pub struct Help;

#[command]
#[aliases("help")]
#[description("Affiche la liste des commandes (optionnel : une catégorie)")]
#[usage("[catégorie]")]
async fn aide(ctx: &Context, msg: &Message, mut args: Args) -> CommandResult {
    let embed = match args.single::<String>() {
        Ok(query) => match find_category(&query) {
            Some(cat) => CreateEmbed::new()
                .color(BLUE)
                .title(format!("📖 Aide — {}", cat.title))
                .description(cat.commands)
                .footer(CreateEmbedFooter::new("Préfixe : !  •  !help pour voir toutes les catégories")),
            None => {
                let list: String = CATEGORIES
                    .iter()
                    .map(|c| format!("• `{}`", c.aliases[0]))
                    .collect::<Vec<_>>()
                    .join("\n");
                CreateEmbed::new()
                    .color(RED)
                    .title("❌ Catégorie inconnue")
                    .description(format!(
                        "La catégorie `{}` n'existe pas.\n\n**Catégories disponibles :**\n{}",
                        query, list
                    ))
            }
        },
        Err(_) => {
            let mut embed = CreateEmbed::new()
                .color(BLUE)
                .title("📖 Aide — Golemian Bot")
                .description("Tape `!help <catégorie>` pour voir le détail d'une seule catégorie.")
                .footer(CreateEmbedFooter::new("Préfixe : !"));
            for cat in CATEGORIES {
                embed = embed.field(cat.title, cat.commands, false);
            }
            embed
        }
    };

    msg.channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await?;

    msg.delete(&ctx.http).await.ok();
    Ok(())
}
