# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

All commands are available via [Taskfile](https://taskfile.dev/) (`Taskfile.yml` at repo root, run `task --list` to see all):

```bash
task start              # Start everything in parallel (db + backend + frontend)
task dev                # Start everything sequentially (db + backend + frontend)
task db                 # Start PostgreSQL only
task db:stop            # Stop PostgreSQL
task db:reset           # Destroy and recreate database
task backend:run        # Run backend (starts db first, auto-migrates on startup)
task backend:test       # Run backend unit tests
task backend:build      # Build backend
task frontend:run       # Run frontend dev server
task frontend:build     # Build frontend for production
task deploy:vps         # Deploy to production VPS (pull, rebuild, restart)
task deploy:restart     # Restart production containers without rebuilding
task deploy:quick       # Git add all, commit, and deploy (optional message: task deploy:quick -- "msg")
```

Run a single backend test: `cd backend && cargo test test_name`

Type-check frontend: `cd frontend && npm run check`

Frontend requires Node >= 25 (`nvm use 25`). Backend runs on `:3000`, frontend on `:5173` with vite proxy forwarding `/api` to the backend.

## Deployment / Production

Production is live at **https://zugzwang-theory.com/** and runs on a **Hostinger VPS** at `srv1335537.hstgr.cloud`.

- **SSH**: `ssh -i ~/.ssh/vps_frouxel root@srv1335537.hstgr.cloud`
- **Nginx** reverse-proxies to the backend on `127.0.0.1:3000` and serves the static frontend build from `/var/www/zugzwang`
- **docker-compose.prod.yml**: runs PostgreSQL + backend containers; frontend is built in a throwaway container and copied to `/var/www/zugzwang`
- **Deploy flow** (`task deploy:vps`): SSH in → `git pull` → rebuild backend + frontend Docker images → copy frontend static files → restart containers
- Production DB password is set via `POSTGRES_PASSWORD` env var (see `.env.production.example`)

## Architecture

Monorepo with a Rust/Axum backend (`backend/`) and SvelteKit 5 frontend (`frontend/`). PostgreSQL 16 via docker-compose.

### Backend

- **Axum 0.8** REST API with `PgPool` as shared state
- **sqlx 0.8** with auto-migrations from `backend/migrations/` on startup
- **Route modules** (`routes/`): `auth`, `repertoires`, `moves`, `training`, `pgn`, `stats`, `bundle`, `explorer` — each exports a `router()` merged in `routes/mod.rs`
- **Handler pattern**: all handlers return `Result<Json<T>, AppError>` where `AppError` (in `error.rs`) implements `IntoResponse` with proper HTTP status codes
- **Authentication** (`auth.rs`): Argon2id password hashing, session tokens (32-byte random, base64, 30-day expiry) stored in DB, `AuthUser` extractor reads `session` cookie via `tower-cookies`. All routes except `/api/auth/*` and `/api/explorer/*` require `AuthUser`. Use `verify_ownership(pool, repertoire_id, user_id)` helper before accessing any repertoire.
- **Models** (`models/`): `User`, `Repertoire`, `MoveNode`, `MoveTreeNode`, `ReviewState` — derive `sqlx::FromRow` and `Serialize`
- **Services** (`services/`):
  - `validation.rs` — **shakmaty is the single source of truth for FEN**. `validate_move(fen, uci)` parses position, validates the move, returns canonical resulting FEN + SAN. The frontend must use the backend-returned FEN for subsequent API calls.
  - `tree.rs` — recursive CTE queries for move trees (adjacency list with `parent_id`), `build_tree()` converts flat rows to nested `MoveTreeNode`
  - `sm2.rs` — SM-2 spaced repetition algorithm, quality grading from response time
  - `pgn.rs` — PGN import via `pgn-reader` Visitor pattern, PGN export via recursive tree walk

### Database Schema

6 tables: `users` → `sessions` (auth), `users` → `repertoires` → `moves` (self-referencing adjacency list via `parent_id`) → `review_state` (1:1 with user-color moves) + `review_log` (every drill attempt). Cascading deletes throughout. All repertoires are scoped to a user via `user_id`.

### Frontend

- **Mobile-first is important** — this app is frequently used on phones/tablets. All new UI must be touch-friendly (min 44px tap targets for primary actions, 36px for secondary). Use `@media (max-width: 768px)` breakpoint. Avoid hover-only interactions; ensure controls are visible/tappable without hover.
- **SvelteKit 5 with runes** (`$state`, `$derived`, `$effect`, `$props`) — do NOT use legacy `export let` or `$:` syntax
- **chessground** for board rendering (initialized in `onMount`, updated via `$effect`)
- **chess.js** only for client-side legal move hints (`toDests()` in `utils/chess.ts`) — never as FEN authority
- **Stores** (`stores/`): `api.ts` (typed fetch wrapper for all endpoints, 401 → redirect to `/login`), `auth.svelte.ts` (reactive auth state), `game.ts`, `repertoire.ts`, `drill.ts`, `lichess.ts` (Lichess explorer API with caching/debounce)
- **Utils** (`utils/`): `chess.ts` (legal move dests via chess.js), `engine.ts` (`Engine` class wrapping Stockfish WASM worker with multi-PV support), `shapes.ts` (chessground arrow rendering for engine lines), `eco.ts` (ECO opening lookup from FEN), `lineAnalysis.ts` (common lines & trap detection via Lichess Explorer — `findCommonLines`, `findTrickyLines`)
- **Data**: `data/eco.json` — ECO opening classification lookup
- **Pages**: login (`/login`), home (`/`), builder (`/repertoire/[id]`), drill (`/train/[id]`), dashboard (`/dashboard`), explorer (`/explorer`), openings browser (`/openings`, `/openings/explorer`)
- **Auth guard**: `+layout.svelte` checks `auth.check()` on mount, redirects to `/login` if unauthenticated (skipped for `/login` itself)
- **Stockfish**: WASM engine in `static/` (stockfish.js, stockfish.wasm, stockfish-worker.js) — worker loaded via `new Worker('/stockfish-worker.js#/stockfish.wasm')`. First load compiles WASM (~15-20s). Engine init is non-blocking with `engineReady` state flag.

### External APIs

- **Lichess Explorer** (`stores/lichess.ts`): fetches opening stats from `explorer.lichess.ovh` (masters + lichess databases). Results are cached in-memory with request debouncing and abort-on-supersede. `fetchExplorerQuiet()` is a non-aborting variant for batch analysis (used by `lineAnalysis.ts`). Backend proxies requests via `routes/explorer.rs` to avoid CORS.

### Key Invariant

The backend (shakmaty) is the canonical FEN source. When a move is added, the backend validates it, computes the resulting FEN, and returns it. The frontend must sync its board state to this backend-returned FEN, not chess.js's computed FEN.

## API Endpoints

| Path | Methods | Purpose |
|------|---------|---------|
| `/api/auth/register` | POST | Register (username, email, password) → set session cookie |
| `/api/auth/login` | POST | Login (username, password) → set session cookie |
| `/api/auth/logout` | POST | Delete session, clear cookie |
| `/api/auth/me` | GET | Current user info (or 401) |
| `/api/repertoires` | GET, POST | List/create repertoires |
| `/api/repertoires/:id` | GET, PUT, DELETE | Single repertoire CRUD |
| `/api/repertoires/:id/moves` | GET, POST | Get move tree / add move |
| `/api/repertoires/:id/moves/:mid` | PUT, DELETE | Update annotation / delete subtree |
| `/api/training/:id/next` | GET | Next drill batch (due moves with full lines) |
| `/api/training/review` | POST | Submit drill result (triggers SM-2 update) |
| `/api/pgn/:id/import` | POST | Import PGN into repertoire |
| `/api/pgn/:id/export` | GET | Export repertoire as PGN |
| `/api/stats/overview` | GET | Global stats |
| `/api/stats/:id` | GET | Per-repertoire stats |
| `/api/stats/activity` | GET | Daily review counts (last 15 weeks) + current streak |
| `/api/stats/:id/heatmap` | GET | Accuracy by destination square |
| `/api/stats/:id/weakest` | GET | Top 10 weakest moves (lowest accuracy, min 3 attempts) |
| `/api/repertoires/:id/variants` | GET | List named variants |
| `/api/repertoires/:id/transpositions` | GET | Find transposition groups by FEN |
| `/api/repertoires/:id/gaps` | GET | Coverage gap analysis |

## SvelteKit Notes

- Use `import MoveTree from './MoveTree.svelte'` for recursive components (not deprecated `<svelte:self>`)
- All types are in `$lib/types/index.ts` — keep frontend types in sync with backend response shapes

## shakmaty 0.28 Notes

- `San::from_move()` and `Position::play()` take `Move` by value, not reference — clone if needed twice
- Use `UciMove` (not deprecated `Uci` alias)
- `Fen::from_position()` takes `&pos` reference
