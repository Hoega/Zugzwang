import type {
	Repertoire,
	CreateRepertoire,
	MoveTreeNode,
	MoveNode,
	CreateMove,
	DrillBatch,
	ReviewSubmission,
	ReviewState,
	StatsOverview,
	RepertoireStats,
	HeatmapEntry,
	TranspositionGroup,
	BundleExport,
	AuthResponse,
	LoginRequest
} from '$lib/types';

async function request<T>(url: string, options?: RequestInit): Promise<T> {
	const res = await fetch(url, {
		headers: { 'Content-Type': 'application/json' },
		...options
	});
	if (!res.ok) {
		if (res.status === 401 && !url.includes('/api/auth/')) {
			window.location.href = '/login';
			throw new Error('Unauthorized');
		}
		const body = await res.json().catch(() => ({ error: res.statusText }));
		throw new Error(body.error || res.statusText);
	}
	return res.json();
}

export const api = {
	// Repertoires
	listRepertoires: () => request<Repertoire[]>('/api/repertoires'),

	createRepertoire: (data: CreateRepertoire) =>
		request<Repertoire>('/api/repertoires', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	getRepertoire: (id: string) => request<Repertoire>(`/api/repertoires/${id}`),

	updateRepertoire: (id: string, name: string) =>
		request<Repertoire>(`/api/repertoires/${id}`, {
			method: 'PUT',
			body: JSON.stringify({ name })
		}),

	deleteRepertoire: (id: string) =>
		request<{ deleted: boolean }>(`/api/repertoires/${id}`, { method: 'DELETE' }),

	// Moves
	getMoveTree: (repertoireId: string) =>
		request<MoveTreeNode[]>(`/api/repertoires/${repertoireId}/moves`),

	addMove: (repertoireId: string, data: CreateMove) =>
		request<MoveNode>(`/api/repertoires/${repertoireId}/moves`, {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	updateMove: (repertoireId: string, moveId: string, data: { comment?: string; nag?: string; variant_name?: string }) =>
		request<MoveNode>(`/api/repertoires/${repertoireId}/moves/${moveId}`, {
			method: 'PUT',
			body: JSON.stringify(data)
		}),

	deleteMove: (repertoireId: string, moveId: string) =>
		request<{ deleted: boolean }>(`/api/repertoires/${repertoireId}/moves/${moveId}`, {
			method: 'DELETE'
		}),

	// Variants
	getVariants: (repertoireId: string) =>
		request<string[]>(`/api/repertoires/${repertoireId}/variants`),

	// Analysis
	getTranspositions: (repertoireId: string) =>
		request<TranspositionGroup[]>(`/api/repertoires/${repertoireId}/transpositions`),

	getCoverageGaps: (repertoireId: string) =>
		request<string[]>(`/api/repertoires/${repertoireId}/gaps`),

	// Training
	getNextBatch: (repertoireId: string, variant?: string, fromMoveId?: string, mode?: 'due' | 'all') => {
		const params = new URLSearchParams();
		if (variant) params.set('variant', variant);
		if (fromMoveId) params.set('from_move_id', fromMoveId);
		if (mode && mode !== 'due') params.set('mode', mode);
		const qs = params.toString();
		return request<DrillBatch>(`/api/training/${repertoireId}/next${qs ? `?${qs}` : ''}`);
	},

	submitReview: (data: ReviewSubmission) =>
		request<ReviewState>('/api/training/review', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	// PGN
	importPgn: (repertoireId: string, pgn: string) =>
		request<{ imported_moves: number }>(`/api/pgn/${repertoireId}/import`, {
			method: 'POST',
			body: JSON.stringify({ pgn })
		}),

	exportPgn: (repertoireId: string) =>
		request<{ pgn: string }>(`/api/pgn/${repertoireId}/export`),

	// Stats
	getOverview: () => request<StatsOverview>('/api/stats/overview'),

	getRepertoireStats: (id: string) => request<RepertoireStats>(`/api/stats/${id}`),

	getHeatmap: (id: string) => request<HeatmapEntry[]>(`/api/stats/${id}/heatmap`),

	// Bundle export/import
	exportBundle: () => request<BundleExport>('/api/bundle/export'),

	importBundle: (data: BundleExport) =>
		request<{ imported_repertoires: number; imported_moves: number }>('/api/bundle/import', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	// Auth
	login: (data: LoginRequest) =>
		request<AuthResponse>('/api/auth/login', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	register: (data: LoginRequest) =>
		request<AuthResponse>('/api/auth/register', {
			method: 'POST',
			body: JSON.stringify(data)
		}),

	logout: () =>
		request<{ ok: boolean }>('/api/auth/logout', { method: 'POST' }),

	getMe: () => request<AuthResponse>('/api/auth/me')
};
