# Golemian Discord Bot

Bot de modération et d'animation pour serveurs Discord. Préfixe : `!`

---

## Modération

### `!ban @utilisateur [raison]`
Bannit définitivement un membre du serveur.
- **Permission requise :** Bannir des membres
- **Exemple :** `!ban @Jean comportement toxique`

### `!unban <user_id> [raison]`
Débannit un membre. L'ID s'obtient en activant le mode développeur (Paramètres → Avancé) puis clic droit sur l'utilisateur dans la liste des bans → **Copier l'identifiant**.
- **Permission requise :** Bannir des membres
- **Exemple :** `!unban 123456789012345678`

### `!kick @utilisateur [raison]`
Expulse un membre du serveur (il peut revenir).
- **Permission requise :** Expulser des membres
- **Exemple :** `!kick @Jean spam`

### `!mute @utilisateur <durée> [raison]`
Met un membre en timeout. Durées : `s` (secondes), `m` (minutes), `h` (heures), `d` (jours).
- **Permission requise :** Modérer des membres
- **Exemples :** `!mute @Jean 10m` — `!mute @Jean 2h insultes`

### `!unmute @utilisateur`
Retire le timeout d'un membre.
- **Permission requise :** Modérer des membres
- **Exemple :** `!unmute @Jean`

### `!warn @utilisateur <raison>`
Avertit un membre et enregistre l'avertissement.
- **Permission requise :** Gérer les messages
- **Exemple :** `!warn @Jean non-respect des règles`

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

## Sondages

### `!poll <question>`
Crée un sondage Oui / Non.
- **Exemple :** `!poll Voulez-vous un événement ce week-end ?`

### `!poll <question> | <choix1> | <choix2> | ...`
Crée un sondage à choix multiples (9 choix maximum). Sépare les options avec `|`.
- **Exemple :** `!poll Quel jeu jouer ? | Minecraft | Valorant | League of Legends`
