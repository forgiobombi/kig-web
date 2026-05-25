# kig-web
Web interface for games, events and stats for the KIG Network.

## Quickstart (UI demo, no Mongo needed)

1. Install Rust (stable)
2. Run the web server:

```bash
cargo run
```

3. Open a demo page (fake data with well-known Minecraft player names):

- `http://127.0.0.1:3233/demo/CAI`
- `http://127.0.0.1:3233/demo/TIMV`
- `http://127.0.0.1:3233/demo/BED`

This route renders a synthetic gamelog entirely in-process, so you can iterate on UI/phone layout without needing a
database.

## Running with real data (MongoDB)

By default the app uses:

- `KIG_MONGO_URI` (default `mongodb://localhost:27017`)
- `KIG_MONGO_DB` (default `kig`)

Example:

```bash
KIG_MONGO_URI="mongodb://localhost:27017" KIG_MONGO_DB="kig" cargo run
```

Gamelog page:

- `/game/{mode}/{id}`

Where:

- `mode` is case-insensitive (examples: `CAI`, `TIMV`, `BP`, `GRAV`, `BED`, `Turf2026`)
- `id` is the base62-encoded game id stored in Mongo

## Dev/testing commands

```bash
cargo test
```

## Config

- `KIG_HOST` (default `127.0.0.1`)
- `KIG_PORT` (default `3233`)
