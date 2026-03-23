export interface Repertoire {
	id: string;
	name: string;
	color: 'white' | 'black';
	created_at: string;
	updated_at: string;
	move_count?: number;
	line_count?: number;
	first_moves?: string | null;
}

export interface CreateRepertoire {
	name: string;
	color: 'white' | 'black';
}

export interface MoveNode {
	id: string;
	repertoire_id: string;
	parent_id: string | null;
	fen: string;
	uci_move: string;
	san_move: string;
	move_number: number;
	is_white_move: boolean;
	comment: string | null;
	nag: string | null;
	variant_name: string | null;
	sort_order: number;
	created_at: string;
}

export interface MoveTreeNode extends MoveNode {
	children: MoveTreeNode[];
}

export interface CreateMove {
	parent_id: string | null;
	fen: string;
	uci_move: string;
}

export interface ReviewState {
	id: string;
	move_id: string;
	repetition_count: number;
	easiness_factor: number;
	interval_days: number;
	next_review: string;
	last_reviewed: string | null;
	created_at: string;
}

export interface ReviewSubmission {
	move_id: string;
	response_time_ms: number;
	played_uci: string;
	was_correct: boolean;
}

export interface DrillPosition {
	move_node: MoveNode;
	line: MoveNode[];
	review_state: ReviewState;
}

export interface DrillBatch {
	positions: DrillPosition[];
	total_due: number;
}

export interface StatsOverview {
	total_moves: number;
	due_today: number;
	mastered: number;
	accuracy_7d: number;
}

export interface RepertoireStats {
	repertoire_id: string;
	move_count: number;
	due_count: number;
	mastery_percentage: number;
}

export interface HeatmapEntry {
	square: string;
	total: number;
	correct: number;
	accuracy: number;
}

export interface ActivityDay {
	date: string;
	count: number;
}

export interface ActivityResponse {
	days: ActivityDay[];
	current_streak: number;
}

export interface WeakMove {
	move_node: MoveNode;
	line: MoveNode[];
	accuracy: number;
	total_attempts: number;
}

export interface TranspositionGroup {
	fen: string;
	move_ids: string[];
}

export interface BundleMove {
	parent_index: number | null;
	fen: string;
	uci_move: string;
	san_move: string;
	move_number: number;
	is_white_move: boolean;
	comment: string | null;
	nag: string | null;
	variant_name: string | null;
	sort_order: number;
}

export interface BundleRepertoire {
	name: string;
	color: string;
	moves: BundleMove[];
}

export interface BundleExport {
	version: number;
	exported_at: string;
	repertoires: BundleRepertoire[];
}

export interface AuthUser {
	id: string;
	username: string;
}

export interface AuthResponse {
	user: AuthUser;
}

export interface LoginRequest {
	username: string;
	password: string;
}

export interface RegisterRequest {
	username: string;
	email: string;
	password: string;
}

export interface ChangePasswordRequest {
	current_password: string;
	new_password: string;
}
