use crate::models::review::ReviewState;

pub struct Sm2Result {
    pub easiness_factor: f64,
    pub interval_days: i32,
    pub repetition_count: i32,
}

pub fn calculate(state: &ReviewState, quality: i32) -> Sm2Result {
    let q = quality.clamp(0, 5) as f64;

    let new_ef = (state.easiness_factor + (0.1 - (5.0 - q) * (0.08 + (5.0 - q) * 0.02)))
        .max(1.3);

    if quality >= 3 {
        let new_n = state.repetition_count + 1;
        let interval = match new_n {
            1 => 1,
            2 => 6,
            _ => (state.interval_days as f64 * new_ef).round() as i32,
        };
        Sm2Result {
            easiness_factor: new_ef,
            interval_days: interval.max(1),
            repetition_count: new_n,
        }
    } else {
        Sm2Result {
            easiness_factor: new_ef,
            interval_days: 1,
            repetition_count: 0,
        }
    }
}

pub fn grade_from_response(correct: bool, response_time_ms: i64) -> i32 {
    if !correct {
        if response_time_ms > 10000 {
            0
        } else {
            1
        }
    } else if response_time_ms < 5000 {
        5
    } else if response_time_ms < 15000 {
        4
    } else {
        3
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;

    fn default_state() -> ReviewState {
        ReviewState {
            id: Uuid::new_v4(),
            move_id: Uuid::new_v4(),
            repetition_count: 0,
            easiness_factor: 2.5,
            interval_days: 0,
            next_review: Utc::now(),
            last_reviewed: None,
            created_at: Utc::now(),
        }
    }

    #[test]
    fn test_perfect_score_first_review() {
        let state = default_state();
        let result = calculate(&state, 5);
        assert_eq!(result.repetition_count, 1);
        assert_eq!(result.interval_days, 1);
        assert!((result.easiness_factor - 2.6).abs() < 0.001);
    }

    #[test]
    fn test_perfect_score_second_review() {
        let mut state = default_state();
        state.repetition_count = 1;
        state.interval_days = 1;
        state.easiness_factor = 2.6;
        let result = calculate(&state, 5);
        assert_eq!(result.repetition_count, 2);
        assert_eq!(result.interval_days, 6);
    }

    #[test]
    fn test_perfect_score_third_review() {
        let mut state = default_state();
        state.repetition_count = 2;
        state.interval_days = 6;
        state.easiness_factor = 2.6;
        let result = calculate(&state, 5);
        assert_eq!(result.repetition_count, 3);
        // 6 * 2.7 = 16.2 -> 16
        assert_eq!(result.interval_days, 16);
    }

    #[test]
    fn test_failure_resets() {
        let mut state = default_state();
        state.repetition_count = 5;
        state.interval_days = 30;
        state.easiness_factor = 2.5;
        let result = calculate(&state, 1);
        assert_eq!(result.repetition_count, 0);
        assert_eq!(result.interval_days, 1);
    }

    #[test]
    fn test_ef_floor() {
        let mut state = default_state();
        state.easiness_factor = 1.3;
        let result = calculate(&state, 0);
        assert!((result.easiness_factor - 1.3).abs() < 0.001);
    }

    #[test]
    fn test_grade_from_response() {
        assert_eq!(grade_from_response(true, 2000), 5);
        assert_eq!(grade_from_response(true, 8000), 4);
        assert_eq!(grade_from_response(true, 20000), 3);
        assert_eq!(grade_from_response(false, 5000), 1);
        assert_eq!(grade_from_response(false, 15000), 0);
    }
}
