<script lang="ts">
	import { Chess } from 'chess.js';
	import type { MultiPvLine } from '$lib/utils/engine';

	let { lines, fen, orientation, onPlayMove }: {
		lines: MultiPvLine[];
		fen: string;
		orientation: 'white' | 'black';
		onPlayMove?: (uci: string) => void;
	} = $props();

	const rankColors = ['#22c55e', '#eab308', '#3b82f6', '#86efac', '#f87171'];

	function uciToSan(uci: string, fen: string): string {
		try {
			const chess = new Chess(fen);
			const from = uci.slice(0, 2);
			const to = uci.slice(2, 4);
			const promotion = uci.length > 4 ? uci[4] : undefined;
			const move = chess.move({ from, to, promotion });
			return move ? move.san : uci;
		} catch {
			return uci;
		}
	}

	function formatScore(line: MultiPvLine): string {
		if (line.mate !== null) {
			return line.mate > 0 ? `M${line.mate}` : `-M${Math.abs(line.mate)}`;
		}
		const score = line.score / 100;
		return score > 0 ? `+${score.toFixed(1)}` : score.toFixed(1);
	}
</script>

<div class="best-moves">
	<div class="header">
		<span class="title">Best Moves</span>
		<span class="depth">{lines.length > 0 ? `d${lines[0].depth}` : ''}</span>
	</div>
	{#each lines as line (line.rank)}
		<button class="line" style="--rank-color: {rankColors[line.rank - 1] ?? '#9ca3af'}" onclick={() => onPlayMove?.(line.bestMove)}>
			<span class="rank">{line.rank}</span>
			<span class="move">{uciToSan(line.bestMove, fen)}</span>
			<span class="score" class:positive={line.score > 0} class:negative={line.score < 0}>
				{formatScore(line)}
			</span>
		</button>
	{/each}
</div>

<style>
	.best-moves {
		padding: 0.75rem 1rem;
		border-bottom: 1px solid var(--color-border);
	}

	.header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
	}

	.title {
		font-size: 0.8rem;
		font-weight: 600;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: var(--color-muted);
	}

	.depth {
		font-size: 0.7rem;
		color: var(--color-muted);
	}

	.line {
		display: grid;
		grid-template-columns: 1.2rem 1fr auto;
		gap: 0.4rem;
		align-items: center;
		padding: 0.2rem 0;
		font-size: 0.85rem;
		width: 100%;
		background: none;
		border: none;
		color: inherit;
		cursor: pointer;
		border-radius: 4px;
		text-align: left;
	}

	.line:hover {
		background: var(--color-border);
	}

	.rank {
		width: 1.2rem;
		height: 1.2rem;
		display: flex;
		align-items: center;
		justify-content: center;
		border-radius: 3px;
		background: var(--rank-color);
		color: white;
		font-size: 0.7rem;
		font-weight: 700;
	}

	.move {
		font-weight: 600;
	}

	.score {
		font-size: 0.8rem;
		font-variant-numeric: tabular-nums;
	}

	.score.positive {
		color: #22c55e;
	}

	.score.negative {
		color: #ef4444;
	}

	@media (max-width: 768px) {
		.line {
			padding: 0.5rem 0.25rem;
			min-height: 40px;
		}
	}
</style>
