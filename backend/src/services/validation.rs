use crate::error::AppError;
use shakmaty::{fen::Fen, san::SanPlus, uci::UciMove, CastlingMode, Chess, Position};

pub struct ValidationResult {
    pub resulting_fen: String,
    pub san_move: String,
    pub move_number: i32,
    pub is_white_move: bool,
}

pub fn validate_move(fen: &str, uci_move: &str) -> Result<ValidationResult, AppError> {
    let fen: Fen = fen
        .parse()
        .map_err(|e| AppError::InvalidMove(format!("Invalid FEN: {e}")))?;

    let setup = fen.into_setup();
    let mut pos: Chess = setup
        .position(CastlingMode::Standard)
        .map_err(|e| AppError::InvalidMove(format!("Invalid position: {e}")))?;

    let uci: UciMove = uci_move
        .parse()
        .map_err(|e| AppError::InvalidMove(format!("Invalid UCI: {e}")))?;

    let m = uci
        .to_move(&pos)
        .map_err(|e| AppError::InvalidMove(format!("Illegal move: {e}")))?;

    let is_white_move = pos.turn().is_white();
    let move_number = pos.fullmoves().get() as i32;

    let san_plus = SanPlus::from_move_and_play_unchecked(&mut pos, m);

    let resulting_fen = shakmaty::fen::Fen::from_position(&pos, shakmaty::EnPassantMode::Legal)
        .to_string();

    Ok(ValidationResult {
        resulting_fen,
        san_move: san_plus.to_string(),
        move_number,
        is_white_move,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_e2e4() {
        let result = validate_move(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "e2e4",
        )
        .unwrap();
        assert_eq!(result.san_move, "e4");
        assert!(result.is_white_move);
        assert_eq!(result.move_number, 1);
        assert!(result.resulting_fen.contains("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR"));
    }

    #[test]
    fn test_validate_illegal_move() {
        let result = validate_move(
            "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
            "e2e5",
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_validate_castling() {
        let result = validate_move(
            "r1bqk2r/pppp1ppp/2n2n2/2b1p3/2B1P3/5N2/PPPP1PPP/RNBQK2R w KQkq - 4 4",
            "e1g1",
        )
        .unwrap();
        assert_eq!(result.san_move, "O-O");
    }

    #[test]
    fn test_validate_promotion() {
        let result = validate_move(
            "8/P7/8/8/8/8/8/4K2k w - - 0 1",
            "a7a8q",
        )
        .unwrap();
        assert!(result.san_move.starts_with("a8=Q"));
    }

    #[test]
    fn test_validate_check_suffix() {
        // Rf8+ (rook delivers check to king on e8)
        let result = validate_move(
            "4k3/8/8/8/8/8/8/4KR2 w - - 0 1",
            "f1f8",
        )
        .unwrap();
        assert_eq!(result.san_move, "Rf8+");
    }

    #[test]
    fn test_validate_checkmate_suffix() {
        // Scholar's mate: Qxf7#
        let result = validate_move(
            "r1bqkb1r/pppp1ppp/2n2n2/4p2Q/2B1P3/8/PPPP1PPP/RNB1K1NR w KQkq - 4 4",
            "h5f7",
        )
        .unwrap();
        assert_eq!(result.san_move, "Qxf7#");
    }
}
