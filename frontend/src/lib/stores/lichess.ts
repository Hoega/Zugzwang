export interface LichessMove {
	uci: string;
	san: string;
	white: number;
	draws: number;
	black: number;
	averageRating: number;
}

export interface LichessGame {
	id: string;
	white: { name: string; rating: number };
	black: { name: string; rating: number };
	year: number;
	winner: string;
}

export interface LichessExplorerResponse {
	opening: { eco: string; name: string } | null;
	white: number;
	draws: number;
	black: number;
	moves: LichessMove[];
	topGames: LichessGame[];
}

export type DatabaseType = 'masters' | 'lichess';

const cache = new Map<string, LichessExplorerResponse>();
let abortController: AbortController | null = null;
let debounceTimer: ReturnType<typeof setTimeout> | null = null;

function cacheKey(fen: string, db: DatabaseType): string {
	return `${db}:${fen}`;
}

export async function fetchExplorer(
	fen: string,
	db: DatabaseType = 'masters'
): Promise<LichessExplorerResponse> {
	const key = cacheKey(fen, db);
	const cached = cache.get(key);
	if (cached) return cached;

	if (abortController) {
		abortController.abort();
	}
	abortController = new AbortController();

	const encodedFen = encodeURIComponent(fen);
	let url: string;
	if (db === 'masters') {
		url = `/api/explorer/masters?fen=${encodedFen}`;
	} else {
		url = `/api/explorer/lichess?fen=${encodedFen}&ratings=2200,2500&speeds=blitz,rapid,classical`;
	}

	const response = await fetch(url, {
		signal: abortController.signal
	});
	if (!response.ok) {
		throw new Error(`Lichess API error: ${response.status}`);
	}

	const data: LichessExplorerResponse = await response.json();
	cache.set(key, data);
	return data;
}

export function fetchExplorerDebounced(
	fen: string,
	db: DatabaseType,
	callback: (data: LichessExplorerResponse) => void,
	onError?: (err: Error) => void
): void {
	if (debounceTimer) {
		clearTimeout(debounceTimer);
	}

	// Return cached immediately without debounce
	const key = cacheKey(fen, db);
	const cached = cache.get(key);
	if (cached) {
		callback(cached);
		return;
	}

	debounceTimer = setTimeout(() => {
		fetchExplorer(fen, db)
			.then(callback)
			.catch((err) => {
				if (err.name !== 'AbortError') {
					onError?.(err);
				}
			});
	}, 200);
}
