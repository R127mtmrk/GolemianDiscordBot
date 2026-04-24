use serenity::builder::{CreateEmbed, CreateEmbedFooter, CreateMessage};
use serenity::framework::standard::{macros::command, macros::group, Args, CommandResult};
use serenity::model::prelude::*;
use serenity::prelude::*;

#[group]
#[commands(aide)]
#[only_in(guilds)]
pub struct Help;

#[command]
#[aliases("help")]
#[description("Affiche la liste des commandes")]
async fn aide(ctx: &Context, msg: &Message, _args: Args) -> CommandResult {
    let embed = CreateEmbed::new()
        .color(0x5865F2)
        .title("📖 Aide — Golemian Bot")
        .field(
            "🛡️ Modération",
            "`!ban @user [raison]` — Bannir\n\
             `!unban <id> [raison]` — Débannir\n\
             `!kick @user [raison]` — Expulser\n\
             `!mute @user <durée> [raison]` — Timeout (`10m`, `2h`, `1d`)\n\
             `!unmute @user` — Retirer le timeout\n\
             `!warn @user <raison>` — Avertir\n\
             `!warnings @user` — Voir les avertissements\n\
             `!clearwarns @user` — Effacer les avertissements\n\
             `!clear <1-100>` — Supprimer des messages",
            false,
        )
        .field(
            "🔧 Utilitaires",
            "`!slowmode <secondes>` — Mode lent (`0` pour désactiver)\n\
             `!lock [#salon]` — Verrouiller un salon\n\
             `!unlock [#salon]` — Déverrouiller un salon\n\
             `!userinfo [@user]` — Infos d'un membre\n\
             `!setmodlog #salon` — Définir le salon des logs de modération",
            false,
        )
        .field(
            "🗂️ Structure de serveur",
            "`!sbackup` — Exporter rôles, catégories, salons et permissions en JSON\n\
             `!srestore` — Recréer une structure depuis un fichier backup.json",
            false,
        )
        .field(
            "🎉 Giveaways",
            "`!gcreate <durée> <gagnants> <prix>` — Créer un giveaway\n\
             `!gend <message_id>` — Terminer un giveaway\n\
             `!greroll <message_id>` — Nouveau tirage\n\
             `!glist` — Voir les giveaways actifs",
            false,
        )
        .field(
            "📊 Sondages",
            "`!poll <question>` — Sondage Oui/Non\n\
             `!poll <question> | <choix1> | <choix2>` — Choix multiples (max 9)",
            false,
        )
        .footer(CreateEmbedFooter::new("Préfixe : !"));

    msg.channel_id
        .send_message(&ctx.http, CreateMessage::new().embed(embed))
        .await?;

    msg.delete(&ctx.http).await.ok();
    Ok(())
}
