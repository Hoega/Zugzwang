use crate::error::AppError;
use crate::models::chess_move::MoveTreeNode;
use crate::services::validation;
use pgn_reader::{BufferedReader, Nag, RawComment, SanPlus, Skip, Visitor};
use shakmaty::{CastlingMode, Chess, Position};
use sqlx::PgPool;
use uuid::Uuid;

const STARTING_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

struct PendingMove {
    parent_id: Option<Uuid>,
    fen_before: String,
    uci: String,
    is_white_move: bool,
    nag: Option<String>,
    comment: Option<String>,
    /// Placeholder UUID assigned during parsing so children can reference this move as parent
    placeholder_id: Uuid,
}

struct PgnImporter {
    moves: Vec<PendingMove>,
    /// Stack for variation handling: (position, parent_placeholder, fen_before)
    stack: Vec<(Chess, Option<Uuid>, String)>,
    current_pos: Chess,
    current_parent_placeholder: Option<Uuid>,
    current_fen: String,
}

impl PgnImporter {
    fn new() -> Self {
        Self {
            moves: Vec::new(),
            stack: Vec::new(),
            current_pos: Chess::default(),
            current_parent_placeholder: None,
            current_fen: STARTING_FEN.to_string(),
        }
    }
}

impl Visitor for PgnImporter {
    type Result = Vec<PendingMove>;

    fn begin_game(&mut self) {
        self.current_pos = Chess::default();
        self.current_parent_placeholder = None;
        self.current_fen = STARTING_FEN.to_string();
        self.moves.clear();
        self.stack.clear();
    }

    fn san(&mut self, san_plus: SanPlus) {
        let m = match san_plus.san.to_move(&self.current_pos) {
            Ok(m) => m,
            Err(_) => return,
        };

        let uci = shakmaty::uci::UciMove::from_standard(m.clone()).to_string();
        let is_white_move = self.current_pos.turn().is_white();
        let fen_before = self.current_fen.clone();

        let new_pos = match self.current_pos.clone().play(m) {
            Ok(p) => p,
            Err(_) => return,
        };

        let resulting_fen =
            shakmaty::fen::Fen::from_position(&new_pos, shakmaty::EnPassantMode::Legal)
                .to_string();

        let placeholder_id = Uuid::new_v4();

        self.moves.push(PendingMove {
            parent_id: self.current_parent_placeholder,
            fen_before,
            uci,
            is_white_move,
            nag: None,
            comment: None,
            placeholder_id,
        });

        self.current_parent_placeholder = Some(placeholder_id);
        self.current_pos = new_pos;
        self.current_fen = resulting_fen;
    }

    fn nag(&mut self, nag: Nag) {
        if let Some(last_move) = self.moves.last_mut() {
            let symbol = match nag.0 {
                1 => "!",
                2 => "?",
                3 => "!!",
                4 => "??",
                5 => "!?",
                6 => "?!",
                _ => return,
            };
            last_move.nag = Some(symbol.to_string());
        }
    }

    fn comment(&mut self, comment: RawComment<'_>) {
        if let Some(last_move) = self.moves.last_mut() {
            let text = String::from_utf8_lossy(comment.as_bytes()).trim().to_string();
            if !text.is_empty() {
                last_move.comment = Some(text);
            }
        }
    }

    fn begin_variation(&mut self) -> Skip {
        // Save current state
        self.stack.push((
            self.current_pos.clone(),
            self.current_parent_placeholder,
            self.current_fen.clone(),
        ));
        // Rewind: variation starts from position before the last move
        if let Some(last_move) = self.moves.last() {
            self.current_parent_placeholder = last_move.parent_id;
            self.current_fen = last_move.fen_before.clone();
            if let Ok(fen) = last_move.fen_before.parse::<shakmaty::fen::Fen>() {
                if let Ok(pos) = fen.into_setup().position::<Chess>(CastlingMode::Standard) {
                    self.current_pos = pos;
                }
            }
        }
        Skip(false)
    }

    fn end_variation(&mut self) {
        if let Some((pos, parent_placeholder, fen)) = self.stack.pop() {
            self.current_pos = pos;
            self.current_parent_placeholder = parent_placeholder;
            self.current_fen = fen;
        }
    }

    fn end_game(&mut self) -> Self::Result {
        std::mem::take(&mut self.moves)
    }
}

pub async fn import_pgn(pool: &PgPool, repertoire_id: Uuid, pgn: &str) -> Result<i64, AppError> {
    let pgn = pgn.trim_start_matches('\u{FEFF}');
    let mut reader = BufferedReader::new_cursor(pgn.as_bytes());
    let mut importer = PgnImporter::new();

    let moves = match reader.read_game(&mut importer) {
        Ok(Some(moves)) => moves,
        Ok(None) => return Err(AppError::InvalidPgn("No game found in PGN".into())),
        Err(e) => return Err(AppError::InvalidPgn(format!("PGN parse error: {e}"))),
    };

    if moves.is_empty() {
        return Ok(0);
    }

    let rep_color: String = sqlx::query_scalar("SELECT color FROM repertoires WHERE id = $1")
        .bind(repertoire_id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Repertoire not found".into()))?;

    let mut tx = pool.begin().await?;
    let mut count: i64 = 0;

    // Maps placeholder UUIDs from parsing to real UUIDs in the database
    let mut placeholder_to_real: std::collections::HashMap<Uuid, Uuid> =
        std::collections::HashMap::new();

    for pending in &moves {
        let result = validation::validate_move(&pending.fen_before, &pending.uci)?;

        // Resolve actual parent_id from placeholder
        let actual_parent_id = pending
            .parent_id
            .map(|pid| *placeholder_to_real.get(&pid).unwrap_or(&pid));

        // Check for existing duplicate
        let existing: Option<(Uuid,)> = sqlx::query_as(
            "SELECT id FROM moves WHERE repertoire_id = $1 AND parent_id IS NOT DISTINCT FROM $2 AND uci_move = $3",
        )
        .bind(repertoire_id)
        .bind(actual_parent_id)
        .bind(&pending.uci)
        .fetch_optional(&mut *tx)
        .await?;

        if let Some((existing_id,)) = existing {
            placeholder_to_real.insert(pending.placeholder_id, existing_id);
            continue;
        }

        let max_sort: Option<i32> = sqlx::query_scalar(
            "SELECT MAX(sort_order) FROM moves WHERE repertoire_id = $1 AND parent_id IS NOT DISTINCT FROM $2",
        )
        .bind(repertoire_id)
        .bind(actual_parent_id)
        .fetch_one(&mut *tx)
        .await?;
        let sort_order = max_sort.unwrap_or(-1) + 1;

        let move_id = Uuid::new_v4();
        placeholder_to_real.insert(pending.placeholder_id, move_id);

        sqlx::query(
            r#"INSERT INTO moves (id, repertoire_id, parent_id, fen, uci_move, san_move,
               move_number, is_white_move, sort_order, nag, comment, created_at)
               VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, NOW())"#,
        )
        .bind(move_id)
        .bind(repertoire_id)
        .bind(actual_parent_id)
        .bind(&result.resulting_fen)
        .bind(&pending.uci)
        .bind(&result.san_move)
        .bind(result.move_number)
        .bind(result.is_white_move)
        .bind(sort_order)
        .bind(&pending.nag)
        .bind(&pending.comment)
        .execute(&mut *tx)
        .await?;

        let is_user_move = (rep_color == "white" && pending.is_white_move)
            || (rep_color == "black" && !pending.is_white_move);

        if is_user_move {
            sqlx::query(
                r#"INSERT INTO review_state (id, move_id, repetition_count, easiness_factor,
                   interval_days, next_review, created_at)
                   VALUES ($1, $2, 0, 2.5, 0, NOW(), NOW())"#,
            )
            .bind(Uuid::new_v4())
            .bind(move_id)
            .execute(&mut *tx)
            .await?;
        }

        count += 1;
    }

    tx.commit().await?;
    Ok(count)
}

fn nag_to_pgn(nag: &str) -> &str {
    match nag {
        "!" => "$1",
        "?" => "$2",
        "!!" => "$3",
        "??" => "$4",
        "!?" => "$5",
        "?!" => "$6",
        other => other,
    }
}

pub fn export_pgn(tree: &[MoveTreeNode]) -> String {
    let mut output = String::new();
    output.push_str("[Event \"Repertoire\"]\n[Site \"Zugzwang\"]\n[Result \"*\"]\n\n");

    fn write_moves(node: &MoveTreeNode, output: &mut String, is_main_line: bool) {
        if node.node.is_white_move {
            output.push_str(&format!("{}. ", node.node.move_number));
        } else if !is_main_line {
            output.push_str(&format!("{}... ", node.node.move_number));
        }

        output.push_str(&node.node.san_move);

        if let Some(ref nag) = node.node.nag {
            output.push_str(&format!(" {}", nag_to_pgn(nag)));
        }

        if let Some(ref comment) = node.node.comment {
            output.push_str(&format!(" {{ {} }}", comment));
        }

        output.push(' ');

        if !node.children.is_empty() {
            write_moves(&node.children[0], output, true);

            for variation in &node.children[1..] {
                output.push_str("( ");
                write_moves(variation, output, false);
                output.push_str(") ");
            }
        }
    }

    for root_node in tree {
        write_moves(root_node, &mut output, true);
    }

    output.push_str("*\n");
    output.trim().to_string()
}
