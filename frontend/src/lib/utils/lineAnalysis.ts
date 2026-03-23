import { Chess } from 'chess.js';
import { fetchExplorerQuiet, type DatabaseType, type LichessMove } from '$lib/stores/lichess';

export interface LineMoveStep {
	san: string;
	uci: string;
	fen: string;
}

export interface CommonLine {
	moves: LineMoveStep[];
	totalGames: number;
	whiteWins: number;
	draws: number;
	blackWins: number;
}

export interface TrickyLine {
	moves: LineMoveStep[];
	trapMove: LineMoveStep;
	popularMove: string;
	popularWinRate: number;
	trapWinRate: number;
	totalGames: number;
	description: string;
}

export interface AnalysisProgress {
	fetched: number;
	total: number;
}

const MAX_DEPTH = 6;
const TOP_MOVES = 3;
const MIN_GAMES = 50;
const TRAP_WIN_RATE_GAP = 0.08;

function totalGames(m: LichessMove): number {
	return m.white + m.draws + m.black;
}

function winRate(m: LichessMove, forWhite: boolean): number {
	const total = totalGames(m);
	if (total === 0) return 0;
	return forWhite ? (m.white + m.draws * 0.5) / total : (m.black + m.draws * 0.5) / total;
}

function applyUciMove(fen: string, uci: string): { san: string; fen: string } | null {
	const chess = new Chess(fen);
	const from = uci.slice(0, 2);
	const to = uci.slice(2, 4);
	const promotion = uci.length > 4 ? uci[4] : undefined;
	try {
		const move = chess.move({ from, to, promotion });
		if (!move) return null;
		return { san: move.san, fen: chess.fen() };
	} catch {
		return null;
	}
}

/**
 * Find the most common continuations from a position by recursively
 * following the most popular moves in the explorer database.
 */
export async function findCommonLines(
	fen: string,
	db: DatabaseType,
	signal: AbortSignal,
	onProgress?: (p: AnalysisProgress) => void
): Promise<CommonLine[]> {
	const lines: CommonLine[] = [];
	let fetched = 0;
	let total = 1;

	async function explore(currentFen: string, path: LineMoveStep[], depth: number) {
		if (signal.aborted) return;
		if (depth >= MAX_DEPTH) {
			if (path.length > 0) {
				// Get stats from the last move's explorer data
				lines.push({
					moves: [...path],
					totalGames: 0,
					whiteWins: 0,
					draws: 0,
					blackWins: 0
				});
			}
			return;
		}

		let data;
		try {
			data = await fetchExplorerQuiet(currentFen, db, signal);
			fetched++;
			onProgress?.({ fetched, total });
		} catch {
			return;
		}

		const topMoves = data.moves
			.filter((m) => totalGames(m) >= MIN_GAMES)
			.slice(0, TOP_MOVES);

		if (topMoves.length === 0) {
			if (path.length > 0) {
				const lastExplorerTotal = data.white + data.draws + data.black;
				lines.push({
					moves: [...path],
					totalGames: lastExplorerTotal,
					whiteWins: data.white,
					draws: data.draws,
					blackWins: data.black
				});
			}
			return;
		}

		// Only follow the main line (most popular) for building complete lines
		// but record all top moves at depth 0 as separate lines
		const movesToFollow = depth === 0 ? topMoves : [topMoves[0]];
		total += movesToFollow.length;

		for (const move of movesToFollow) {
			if (signal.aborted) return;
			const result = applyUciMove(currentFen, move.uci);
			if (!result) continue;

			const step: LineMoveStep = { san: result.san, uci: move.uci, fen: result.fen };
			path.push(step);
			await explore(result.fen, path, depth + 1);
			path.pop();

			// Small delay to avoid hammering the API
			if (!signal.aborted) {
				await new Promise((r) => setTimeout(r, 80));
			}
		}
	}

	await explore(fen, [], 0);

	// Sort by total games descending, fill in stats for lines that ended at max depth
	// Re-fetch leaf stats for lines that hit max depth
	for (const line of lines) {
		if (line.totalGames === 0 && line.moves.length > 0) {
			try {
				const leafData = await fetchExplorerQuiet(
					line.moves[line.moves.length - 1].fen,
					db,
					signal
				);
				line.totalGames = leafData.white + leafData.draws + leafData.black;
				line.whiteWins = leafData.white;
				line.draws = leafData.draws;
				line.blackWins = leafData.black;
			} catch {
				// ignore
			}
		}
	}

	lines.sort((a, b) => b.totalGames - a.totalGames);
	return lines;
}

/**
 * Find tricky/trap positions from a given FEN by looking for moves where
 * the most popular choice significantly underperforms an alternative.
 */
export async function findTrickyLines(
	fen: string,
	db: DatabaseType,
	signal: AbortSignal,
	onProgress?: (p: AnalysisProgress) => void
): Promise<TrickyLine[]> {
	const traps: TrickyLine[] = [];
	let fetched = 0;
	let total = 1;

	async function explore(currentFen: string, path: LineMoveStep[], depth: number) {
		if (signal.aborted || depth >= MAX_DEPTH) return;

		let data;
		try {
			data = await fetchExplorerQuiet(currentFen, db, signal);
			fetched++;
			onProgress?.({ fetched, total });
		} catch {
			return;
		}

		const validMoves = data.moves.filter((m) => totalGames(m) >= MIN_GAMES);
		if (validMoves.length < 2) return;

		// Determine whose turn it is
		const isWhiteTurn = currentFen.split(' ')[1] === 'w';

		// Find the most popular move and the best-performing move
		const sorted = [...validMoves].sort((a, b) => totalGames(b) - totalGames(a));
		const mostPopular = sorted[0];
		const mostPopularWr = winRate(mostPopular, isWhiteTurn);

		// Check each alternative for better win rate
		for (const alt of sorted.slice(1)) {
			const altWr = winRate(alt, isWhiteTurn);
			if (altWr - mostPopularWr >= TRAP_WIN_RATE_GAP && totalGames(alt) >= MIN_GAMES) {
				const result = applyUciMove(currentFen, alt.uci);
				if (!result) continue;

				const trapStep: LineMoveStep = { san: result.san, uci: alt.uci, fen: result.fen };
				const popResult = applyUciMove(currentFen, mostPopular.uci);
				const popSan = popResult?.san ?? mostPopular.san;

				const side = isWhiteTurn ? 'White' : 'Black';
				traps.push({
					moves: [...path],
					trapMove: trapStep,
					popularMove: popSan,
					popularWinRate: mostPopularWr,
					trapWinRate: altWr,
					totalGames: totalGames(alt),
					description: `${side} can play ${result.san} (${(altWr * 100).toFixed(0)}%) instead of the popular ${popSan} (${(mostPopularWr * 100).toFixed(0)}%)`
				});
			}
		}

		// Also check: is the most popular move a trap? (low win rate for the side playing it)
		// If the best alternative is much better, the popular move is a "trap to avoid"
		const bestWr = Math.max(...validMoves.map((m) => winRate(m, isWhiteTurn)));
		if (bestWr - mostPopularWr >= TRAP_WIN_RATE_GAP) {
			const bestMove = validMoves.find((m) => winRate(m, isWhiteTurn) === bestWr);
			if (bestMove && bestMove.uci !== mostPopular.uci) {
				// Already captured above
			}
		}

		// Recurse into top moves
		const movesToFollow = sorted.slice(0, 2);
		total += movesToFollow.length;

		for (const move of movesToFollow) {
			if (signal.aborted) return;
			const result = applyUciMove(currentFen, move.uci);
			if (!result) continue;

			const step: LineMoveStep = { san: result.san, uci: move.uci, fen: result.fen };
			path.push(step);
			await explore(result.fen, path, depth + 1);
			path.pop();

			if (!signal.aborted) {
				await new Promise((r) => setTimeout(r, 80));
			}
		}
	}

	await explore(fen, [], 0);

	// Sort by win rate gap descending (biggest traps first)
	traps.sort((a, b) => (b.trapWinRate - b.popularWinRate) - (a.trapWinRate - a.popularWinRate));
	return traps;
}
