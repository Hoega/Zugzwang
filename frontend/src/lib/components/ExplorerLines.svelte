<script lang="ts">
	import { type DatabaseType } from '$lib/stores/lichess';
	import {
		findCommonLines,
		findTrickyLines,
		type CommonLine,
		type TrickyLine,
		type AnalysisProgress
	} from '$lib/utils/lineAnalysis';

	let {
		fen,
		onNavigate
	}: {
		fen: string;
		onNavigate?: (fen: string) => void;
	} = $props();

	let tab = $state<'popular' | 'traps'>('popular');
	let db = $state<DatabaseType>('masters');
	let commonLines = $state<CommonLine[]>([]);
	let trickyLines = $state<TrickyLine[]>([]);
	let loading = $state(false);
	let progress = $state<AnalysisProgress>({ fetched: 0, total: 0 });
	let abortController = $state<AbortController | null>(null);
	let analyzedFen = $state('');
	let analyzedDb = $state<DatabaseType>('masters');
	let analyzedTab = $state<'popular' | 'traps'>('popular');

	function needsRefresh(): boolean {
		return fen !== analyzedFen || db !== analyzedDb || tab !== analyzedTab;
	}

	async function analyze() {
		if (abortController) {
			abortController.abort();
		}
		abortController = new AbortController();
		loading = true;
		progress = { fetched: 0, total: 1 };

		const currentFen = fen;
		const currentDb = db;
		const currentTab = tab;

		try {
			if (currentTab === 'popular') {
				commonLines = await findCommonLines(currentFen, currentDb, abortController.signal, (p) => {
					progress = p;
				});
			} else {
				trickyLines = await findTrickyLines(currentFen, currentDb, abortController.signal, (p) => {
					progress = p;
				});
			}
			analyzedFen = currentFen;
			analyzedDb = currentDb;
			analyzedTab = currentTab;
		} catch (e: unknown) {
			if (e instanceof Error && e.name !== 'AbortError') {
				console.error('Analysis failed:', e);
			}
		}
		loading = false;
	}

	function formatMoves(moves: { san: string }[]): string {
		let result = '';
		for (let i = 0; i < moves.length; i++) {
			const moveNum = Math.floor(i / 2) + 1;
			if (i % 2 === 0) {
				result += `${moveNum}.`;
			}
			result += moves[i].san + ' ';
		}
		return result.trim();
	}

	function formatGames(n: number): string {
		if (n >= 1000000) return `${(n / 1000000).toFixed(1)}M`;
		if (n >= 1000) return `${(n / 1000).toFixed(1)}k`;
		return String(n);
	}

	function winPct(line: CommonLine): string {
		if (line.totalGames === 0) return '';
		const w = ((line.whiteWins / line.totalGames) * 100).toFixed(0);
		const d = ((line.draws / line.totalGames) * 100).toFixed(0);
		const b = ((line.blackWins / line.totalGames) * 100).toFixed(0);
		return `${w}/${d}/${b}`;
	}
</script>

<div class="explorer-lines">
	<div class="el-header">
		<div class="el-tabs">
			<button class:active={tab === 'popular'} onclick={() => { tab = 'popular'; }}>Popular</button>
			<button class:active={tab === 'traps'} onclick={() => { tab = 'traps'; }}>Traps</button>
		</div>
		<select bind:value={db}>
			<option value="masters">Masters</option>
			<option value="lichess">Lichess 2200+</option>
		</select>
	</div>

	<div class="el-actions">
		<button class="analyze-btn" onclick={analyze} disabled={loading}>
			{#if loading}
				Analyzing... ({progress.fetched}/{progress.total})
			{:else if needsRefresh()}
				Analyze
			{:else}
				Re-analyze
			{/if}
		</button>
	</div>

	<div class="el-content">
		{#if tab === 'popular'}
			{#if commonLines.length === 0 && !loading}
				<p class="el-empty">{analyzedFen ? 'No common lines found' : 'Click Analyze to find popular continuations'}</p>
			{:else}
				{#each commonLines as line, i}
					<button
						class="el-line"
						onclick={() => {
							if (line.moves.length > 0) {
								onNavigate?.(line.moves[line.moves.length - 1].fen);
							}
						}}
					>
						<div class="el-line-main">
							<span class="el-line-num">{i + 1}.</span>
							<span class="el-line-moves">{formatMoves(line.moves)}</span>
						</div>
						<div class="el-line-stats">
							<span class="el-games">{formatGames(line.totalGames)} games</span>
							{#if line.totalGames > 0}
								<div class="el-bar">
									<div class="el-bar-w" style="width: {(line.whiteWins / line.totalGames) * 100}%"></div>
									<div class="el-bar-d" style="width: {(line.draws / line.totalGames) * 100}%"></div>
									<div class="el-bar-b" style="width: {(line.blackWins / line.totalGames) * 100}%"></div>
								</div>
								<span class="el-pct">{winPct(line)}</span>
							{/if}
						</div>
					</button>
				{/each}
			{/if}
		{:else}
			{#if trickyLines.length === 0 && !loading}
				<p class="el-empty">{analyzedFen ? 'No traps found at this depth' : 'Click Analyze to find tricky lines'}</p>
			{:else}
				{#each trickyLines as trap, i}
					<button
						class="el-trap"
						onclick={() => {
							onNavigate?.(trap.trapMove.fen);
						}}
					>
						<div class="el-trap-header">
							<span class="el-trap-num">{i + 1}.</span>
							<span class="el-trap-desc">{trap.description}</span>
						</div>
						{#if trap.moves.length > 0}
							<div class="el-trap-context">
								<span class="el-context-label">After</span>
								<span class="el-context-moves">{formatMoves(trap.moves)}</span>
							</div>
						{/if}
						<div class="el-trap-comparison">
							<div class="el-trap-move good">
								<span class="el-move-san">{trap.trapMove.san}</span>
								<span class="el-move-pct">{(trap.trapWinRate * 100).toFixed(0)}%</span>
							</div>
							<span class="el-vs">vs</span>
							<div class="el-trap-move bad">
								<span class="el-move-san">{trap.popularMove}</span>
								<span class="el-move-pct">{(trap.popularWinRate * 100).toFixed(0)}%</span>
							</div>
						</div>
					</button>
				{/each}
			{/if}
		{/if}
	</div>
</div>

<style>
	.explorer-lines {
		border-bottom: 1px solid var(--color-border);
	}

	.el-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.5rem 0.75rem;
		border-bottom: 1px solid var(--color-border);
		gap: 0.5rem;
	}

	.el-tabs {
		display: flex;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		overflow: hidden;
	}

	.el-tabs button {
		padding: 0.2rem 0.5rem;
		border: none;
		background: transparent;
		cursor: pointer;
		font-size: 0.75rem;
		color: var(--color-text);
	}

	.el-tabs button + button {
		border-left: 1px solid var(--color-border);
	}

	.el-tabs button.active {
		background: var(--color-primary);
		color: white;
	}

	select {
		padding: 0.2rem 0.3rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-surface);
		font-size: 0.7rem;
		color: var(--color-text);
	}

	.el-actions {
		padding: 0.4rem 0.75rem;
	}

	.analyze-btn {
		width: 100%;
		padding: 0.35rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-surface);
		cursor: pointer;
		font-size: 0.8rem;
		color: var(--color-text);
	}

	.analyze-btn:hover:not(:disabled) {
		background: var(--color-border);
	}

	.analyze-btn:disabled {
		opacity: 0.6;
		cursor: default;
	}

	.el-content {
		max-height: 300px;
		overflow-y: auto;
	}

	.el-empty {
		padding: 0.75rem;
		font-size: 0.8rem;
		color: var(--color-muted);
		text-align: center;
		margin: 0;
	}

	/* Common lines */
	.el-line {
		display: block;
		width: 100%;
		text-align: left;
		padding: 0.4rem 0.75rem;
		border: none;
		border-top: 1px solid var(--color-border);
		background: transparent;
		cursor: pointer;
		color: var(--color-text);
	}

	.el-line:first-child {
		border-top: none;
	}

	.el-line:hover {
		background: var(--color-border);
	}

	.el-line-main {
		display: flex;
		gap: 0.3rem;
		margin-bottom: 0.2rem;
	}

	.el-line-num {
		font-size: 0.7rem;
		color: var(--color-muted);
		flex-shrink: 0;
	}

	.el-line-moves {
		font-size: 0.8rem;
		font-weight: 500;
		word-break: break-word;
	}

	.el-line-stats {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.el-games {
		font-size: 0.7rem;
		color: var(--color-muted);
		flex-shrink: 0;
	}

	.el-bar {
		display: flex;
		height: 6px;
		border-radius: 3px;
		overflow: hidden;
		flex: 1;
		min-width: 40px;
	}

	.el-bar-w {
		background: #f0f0f0;
	}

	.el-bar-d {
		background: #999;
	}

	.el-bar-b {
		background: #333;
	}

	.el-pct {
		font-size: 0.6rem;
		color: var(--color-muted);
		flex-shrink: 0;
	}

	/* Tricky lines */
	.el-trap {
		display: block;
		width: 100%;
		text-align: left;
		padding: 0.5rem 0.75rem;
		border: none;
		border-top: 1px solid var(--color-border);
		background: transparent;
		cursor: pointer;
		color: var(--color-text);
	}

	.el-trap:first-child {
		border-top: none;
	}

	.el-trap:hover {
		background: var(--color-border);
	}

	.el-trap-header {
		display: flex;
		gap: 0.3rem;
		margin-bottom: 0.3rem;
	}

	.el-trap-num {
		font-size: 0.7rem;
		color: var(--color-muted);
		flex-shrink: 0;
	}

	.el-trap-desc {
		font-size: 0.75rem;
		line-height: 1.3;
	}

	.el-trap-context {
		margin-bottom: 0.3rem;
	}

	.el-context-label {
		font-size: 0.65rem;
		color: var(--color-muted);
		text-transform: uppercase;
		margin-right: 0.3rem;
	}

	.el-context-moves {
		font-size: 0.75rem;
		color: var(--color-muted);
	}

	.el-trap-comparison {
		display: flex;
		align-items: center;
		gap: 0.4rem;
	}

	.el-trap-move {
		display: flex;
		align-items: center;
		gap: 0.25rem;
		padding: 0.15rem 0.4rem;
		border-radius: 4px;
		font-size: 0.75rem;
	}

	.el-trap-move.good {
		background: #dcfce7;
		color: #16a34a;
	}

	.el-trap-move.bad {
		background: #fee2e2;
		color: #dc2626;
	}

	.el-move-san {
		font-weight: 600;
	}

	.el-move-pct {
		font-size: 0.7rem;
	}

	.el-vs {
		font-size: 0.65rem;
		color: var(--color-muted);
	}

	@media (max-width: 768px) {
		.el-tabs button {
			padding: 0.4rem 0.75rem;
			min-height: 36px;
		}

		.analyze-btn {
			min-height: 44px;
		}

		.el-line,
		.el-trap {
			padding: 0.6rem 0.75rem;
		}
	}
</style>
