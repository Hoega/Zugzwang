# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

All commands are available via [Task](https://taskfile.dev/) (`task --list`):

```bash
task start              # Start everything in parallel (db + backend + frontend)
task db                 # Start PostgreSQL only
task db:stop            # Stop PostgreSQL
task db:reset           # Destroy and recreate database
task backend:run        # Run backend (starts db first, auto-migrates on startup)
task backend:test       # Run backend unit tests
task backend:build      # Build backend
task frontend:run       # Run frontend dev server
task frontend:build     # Build frontend for production
```

Run a single backend test: `cd backend && cargo test test_name`

Type-check frontend: `cd frontend && npm run check`

Frontend requires Node >= 25 (`nvm use 25`). Backend runs on `:3000`, frontend on `:5173` with vite proxy forwarding `/api` to the backend.

## Architecture

Monorepo with a Rust/Axum backend (`backend/`) and SvelteKit 5 frontend (`frontend/`). PostgreSQL 16 via docker-compose.

### Backend

- **Axum 0.8** REST API with `PgPool` as shared state
- **sqlx 0.8** with auto-migrations from `backend/migrations/` on startup
- **Route modules** (`routes/`): `repertoires`, `moves`, `training`, `pgn`, `stats` — each exports a `router() -> Router<PgPool>` merged in `routes/mod.rs`
- **Handler pattern**: all handlers return `Result<Json<T>, AppError>` where `AppError` (in `error.rs`) implements `IntoResponse` with proper HTTP status codes
- **Models** (`models/`): `Repertoire`, `MoveNode`, `MoveTreeNode`, `ReviewState` — derive `sqlx::FromRow` and `Serialize`
- **Services** (`services/`):
  - `validation.rs` — **shakmaty is the single source of truth for FEN**. `validate_move(fen, uci)` parses position, validates the move, returns canonical resulting FEN + SAN. The frontend must use the backend-returned FEN for subsequent API calls.
  - `tree.rs` — recursive CTE queries for move trees (adjacency list with `parent_id`), `build_tree()` converts flat rows to nested `MoveTreeNode`
  - `sm2.rs` — SM-2 spaced repetition algorithm, quality grading from response time
  - `pgn.rs` — PGN import via `pgn-reader` Visitor pattern, PGN export via recursive tree walk

### Database Schema

4 tables: `repertoires` → `moves` (self-referencing adjacency list via `parent_id`) → `review_state` (1:1 with user-color moves) + `review_log` (every drill attempt). Cascading deletes throughout.

### Frontend

- **SvelteKit 5 with runes** (`$state`, `$derived`, `$effect`, `$props`) — do NOT use legacy `export let` or `$:` syntax
- **chessground** for board rendering (initialized in `onMount`, updated via `$effect`)
- **chess.js** only for client-side legal move hints (`toDests()` in `utils/chess.ts`) — never as FEN authority
- **Stores** (`stores/`): `api.ts` (typed fetch wrapper for all endpoints), `game.ts`, `repertoire.ts`, `drill.ts`
- **Pages**: home (`/`), builder (`/repertoire/[id]`), drill (`/train/[id]`), dashboard (`/dashboard`)
- **Stockfish**: WASM engine in `static/` (stockfish.js, stockfish.wasm, stockfish-worker.js) — first load compiles WASM (~15-20s)

### Key Invariant

The backend (shakmaty) is the canonical FEN source. When a move is added, the backend validates it, computes the resulting FEN, and returns it. The frontend must sync its board state to this backend-returned FEN, not chess.js's computed FEN.

## API Endpoints

| Path | Methods | Purpose |
|------|---------|---------|
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
| `/api/stats/:id/heatmap` | GET | Accuracy by destination square |

## shakmaty 0.28 Notes

- `San::from_move()` and `Position::play()` take `Move` by value, not reference — clone if needed twice
- Use `UciMove` (not deprecated `Uci` alias)
- `Fen::from_position()` takes `&pos` reference
