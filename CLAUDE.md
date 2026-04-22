# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo run          # Lancer le bot en local (nécessite un fichier .env)
cargo build        # Compiler en mode debug
cargo build --release  # Compiler pour la production
cargo check        # Vérifier les erreurs sans compiler
```

## Variables d'environnement

Créer un fichier `.env` à la racine :
```
DISCORD_TOKEN=ton_token_ici
DATABASE_URL=sqlite:bot.db   # optionnel, défaut: sqlite:bot.db
```

## Architecture

Le bot utilise **Serenity** (framework Discord) avec le `StandardFramework` (préfixe `!`) et **SQLx** avec SQLite pour la persistance.

### Flux de démarrage (`src/main.rs`)
1. Charge `.env` via `dotenvy`
2. Crée le pool SQLite et initialise les tables (`src/database.rs`)
3. Enregistre les 3 groupes de commandes
4. Démarre un `tokio::spawn` qui vérifie les giveaways expirés toutes les 30 secondes
5. Écoute les réactions 🎉 pour enregistrer/retirer les participants aux giveaways

### Modules de commandes (`src/commands/`)
- `moderation.rs` — `!ban`, `!kick`, `!mute`, `!unmute`, `!warn`, `!warnings`, `!clearwarns`, `!clear`
- `giveaway.rs` — `!gcreate`, `!gend`, `!greroll`, `!glist` + logique de fin automatique
- `poll.rs` — `!poll` (Oui/Non ou choix multiples jusqu'à 9 options)

### Base de données (`src/database.rs`)
Trois tables SQLite créées au démarrage : `warnings`, `giveaways`, `giveaway_entries`.  
Le pool est partagé via `TypeMapKey` (`DatabaseKey`) dans le `Context` Serenity.

### Déploiement (Railway)
- `Dockerfile` à la racine utilise `rust:latest` + `libsqlite3-dev`
- Volume Railway monté sur `/data`, base de données à `/data/bot.db` via `DATABASE_URL`
- `nixpacks.toml` présent mais ignoré car le Dockerfile prend la priorité

### Gateway Intents requis
`GUILD_MESSAGES`, `MESSAGE_CONTENT`, `GUILD_MESSAGE_REACTIONS`
