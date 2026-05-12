# Golemian Discord Bot

Bot de modération et d'animation pour serveurs Discord. Préfixe : `!`

---

## Modération

> Les commandes `!ban`, `!unban`, `!kick`, `!mute`, `!unmute` et `!warn` acceptent plusieurs utilisateurs mentionnés à la suite. La raison (ou la durée pour `!mute`) vient après les mentions.

### `!ban @utilisateur [@user2 ...] [raison]`
Bannit définitivement un ou plusieurs membres du serveur.
- **Permission requise :** Bannir des membres
- **Exemples :** `!ban @Jean comportement toxique` — `!ban @Jean @Paul raid`

### `!unban <user_id> [<user_id2> ...] [raison]`
Débannit un ou plusieurs membres. L'ID s'obtient en activant le mode développeur (Paramètres → Avancé) puis clic droit sur l'utilisateur dans la liste des bans → **Copier l'identifiant**.
- **Permission requise :** Bannir des membres
- **Exemple :** `!unban 123456789012345678`

### `!kick @utilisateur [@user2 ...] [raison]`
Expulse un ou plusieurs membres du serveur (ils peuvent revenir).
- **Permission requise :** Expulser des membres
- **Exemples :** `!kick @Jean spam` — `!kick @Jean @Paul flood`

### `!mute @utilisateur [@user2 ...] <durée> [raison]`
Met un ou plusieurs membres en timeout. Durées : `s` (secondes), `m` (minutes), `h` (heures), `d` (jours).
- **Permission requise :** Modérer des membres
- **Exemples :** `!mute @Jean 10m` — `!mute @Jean @Paul 2h insultes`

### `!unmute @utilisateur [@user2 ...]`
Retire le timeout d'un ou plusieurs membres.
- **Permission requise :** Modérer des membres
- **Exemple :** `!unmute @Jean @Paul`

### `!warn @utilisateur [@user2 ...] <raison>`
Avertit un ou plusieurs membres et enregistre l'avertissement pour chacun.
- **Permission requise :** Gérer les messages
- **Exemples :** `!warn @Jean non-respect des règles` — `!warn @Jean @Paul flood`

### `!warnings @utilisateur`
Affiche la liste des avertissements d'un membre.
- **Permission requise :** Gérer les messages
- **Exemple :** `!warnings @Jean`

### `!clearwarns @utilisateur`
Supprime tous les avertissements d'un membre.
- **Permission requise :** Administrateur
- **Exemple :** `!clearwarns @Jean`

### `!clear <nombre>`
Supprime entre 1 et 100 messages dans le salon actuel.
- **Permission requise :** Gérer les messages
- **Exemple :** `!clear 20`

---

## Giveaways

### `!gcreate <durée> <gagnants> <prix>`
Lance un giveaway. Les membres participent en réagissant avec 🎉.
- **Permission requise :** Gérer le serveur
- **Exemple :** `!gcreate 24h 1 Abonnement Nitro`
- **Exemple :** `!gcreate 30m 3 Rôle exclusif`

### `!gend <message_id>`
Termine immédiatement un giveaway et tire au sort les gagnants.
- **Permission requise :** Gérer le serveur
- **Exemple :** `!gend 1234567890123456789`

### `!greroll <message_id>`
Effectue un nouveau tirage au sort pour un giveaway terminé.
- **Permission requise :** Gérer le serveur
- **Exemple :** `!greroll 1234567890123456789`

### `!glist`
Affiche tous les giveaways en cours sur le serveur.
- **Exemple :** `!glist`

> Pour obtenir l'ID d'un message : clic droit sur le message → **Copier l'identifiant**

---

## Utilitaires

### `!slowmode <secondes>`
Active le mode lent sur le salon actuel. `0` pour désactiver. Maximum 21600 (6h).
- **Permission requise :** Gérer les salons
- **Exemple :** `!slowmode 30` — `!slowmode 0`

### `!lock [#salon]`
Verrouille un salon en bloquant les messages pour @everyone. Sans argument, verrouille le salon actuel.
- **Permission requise :** Gérer les salons
- **Exemple :** `!lock` — `!lock #général`

### `!unlock [#salon]`
Déverrouille un salon.
- **Permission requise :** Gérer les salons
- **Exemple :** `!unlock` — `!unlock #général`

### `!userinfo [@utilisateur]`
Affiche les informations d'un membre : date de création du compte, date d'arrivée, rôles, nombre d'avertissements. Sans argument, affiche les infos de l'auteur.
- **Exemple :** `!userinfo @Jean`

### `!setmodlog #salon`
Définit le salon où sont envoyés les logs de toutes les actions de modération (ban, kick, mute, warn, etc.).
- **Permission requise :** Gérer le serveur
- **Exemple :** `!setmodlog #logs-modération`

---

## Structure de serveur

### `!sbackup`
Exporte la structure complète du serveur (rôles, catégories, salons) dans un fichier `backup.json`.
- **Permission requise :** Gérer le serveur

### `!srestore`
Recrée la structure d'un serveur depuis un fichier `backup.json` (à joindre en pièce jointe).
- **Permission requise :** Administrateur
- **Exemple :** Taper `!srestore` avec le fichier `backup.json` en pièce jointe

---

## Sondages

### `!poll <question>`
Crée un sondage Oui / Non.
- **Exemple :** `!poll Voulez-vous un événement ce week-end ?`

### `!poll <question> | <choix1> | <choix2> | ...`
Crée un sondage à choix multiples (9 choix maximum). Sépare les options avec `|`.
- **Exemple :** `!poll Quel jeu jouer ? | Minecraft | Valorant | League of Legends`

---

## Tickets

Système de tickets privés avec bouton. Les membres ouvrent un ticket en cliquant sur un bouton dans le panneau ; un salon privé est créé automatiquement dans la catégorie configurée, avec accès pour le membre et l'éventuel rôle support.

### `!setticket #catégorie [@rôle_support] [#salon_logs]`
Configure le système de tickets. La catégorie sert à regrouper les tickets, le rôle support a accès à tous les tickets, et le salon de logs reçoit les ouvertures/fermetures.
- **Permission requise :** Gérer le serveur
- **Exemple :** `!setticket #📩-tickets @Support #ticket-logs`

### `!ticketpanel [#salon]`
Envoie le panneau avec le bouton **Ouvrir un ticket** dans le salon indiqué (ou le salon actuel par défaut).
- **Permission requise :** Gérer le serveur
- **Exemple :** `!ticketpanel #support`

### `!delticket`
Désactive le système de tickets sur le serveur (la configuration est supprimée, les tickets ouverts restent).
- **Permission requise :** Gérer le serveur

### `!tclose`
Ferme le ticket actuel. Utilisable par le créateur du ticket ou tout membre ayant **Gérer les salons**. Le salon est supprimé après 5 secondes.

### `!tadd @utilisateur`
Ajoute un membre au ticket actuel.
- **Permission requise :** Gérer les salons
- **Exemple :** `!tadd @Marie`

### `!tremove @utilisateur`
Retire un membre du ticket actuel (sauf le créateur).
- **Permission requise :** Gérer les salons
- **Exemple :** `!tremove @Marie`

---

## Salons vocaux temporaires

Définit un salon vocal "hub" : dès qu'un membre le rejoint, un salon vocal personnel est créé, et il y est déplacé. Le salon est automatiquement supprimé quand il devient vide.

### `!settempvc #salon-vocal`
Définit le salon vocal hub.
- **Permission requise :** Gérer les salons
- **Exemple :** `!settempvc #➕-créer-vocal`

### `!deltempvc`
Désactive le système de salons vocaux temporaires.
- **Permission requise :** Gérer les salons

---

## Bot

> Discord interdit aux bots de rejoindre un serveur via un lien d'invitation. Pour ajouter le bot, un administrateur du serveur cible doit ouvrir l'URL OAuth2 ci-dessous et l'autoriser.

### `!invite`
Affiche le lien d'invitation OAuth2 du bot (avec la permission Administrateur demandée par défaut).
- **Exemple :** `!invite`

### `!servers`
Liste tous les serveurs où le bot est présent (nom, ID, nombre de membres).
- **Permission requise :** propriétaire du bot (owner de l'application Discord)

### `!leave <guild_id>`
Force le bot à quitter le serveur dont l'ID est donné. L'ID se récupère via `!servers`.
- **Permission requise :** propriétaire du bot
- **Exemple :** `!leave 123456789012345678`

---

## Aide

### `!help` (alias `!aide`)
Affiche dans Discord la liste complète des commandes regroupées par catégorie.
