CREATE TABLE repertoires (
    id UUID PRIMARY KEY,
    name TEXT NOT NULL,
    color TEXT NOT NULL CHECK (color IN ('white', 'black')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE moves (
    id UUID PRIMARY KEY,
    repertoire_id UUID NOT NULL REFERENCES repertoires(id) ON DELETE CASCADE,
    parent_id UUID REFERENCES moves(id) ON DELETE CASCADE,
    fen TEXT NOT NULL,
    uci_move TEXT NOT NULL,
    san_move TEXT NOT NULL,
    move_number INTEGER NOT NULL,
    is_white_move BOOLEAN NOT NULL,
    comment TEXT,
    nag TEXT,
    sort_order INTEGER NOT NULL DEFAULT 0,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_moves_parent_id ON moves(parent_id);
CREATE INDEX idx_moves_repertoire_id ON moves(repertoire_id);
CREATE INDEX idx_moves_fen ON moves(fen);

CREATE TABLE review_state (
    id UUID PRIMARY KEY,
    move_id UUID NOT NULL UNIQUE REFERENCES moves(id) ON DELETE CASCADE,
    repetition_count INTEGER NOT NULL DEFAULT 0,
    easiness_factor DOUBLE PRECISION NOT NULL DEFAULT 2.5,
    interval_days INTEGER NOT NULL DEFAULT 0,
    next_review TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_reviewed TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE review_log (
    id UUID PRIMARY KEY,
    move_id UUID NOT NULL REFERENCES moves(id) ON DELETE CASCADE,
    quality INTEGER NOT NULL CHECK (quality >= 0 AND quality <= 5),
    response_time_ms BIGINT NOT NULL,
    played_uci TEXT NOT NULL,
    was_correct BOOLEAN NOT NULL,
    reviewed_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_review_state_next_review ON review_state(next_review);
CREATE INDEX idx_review_log_move_id ON review_log(move_id);
CREATE INDEX idx_review_log_reviewed_at ON review_log(reviewed_at);
