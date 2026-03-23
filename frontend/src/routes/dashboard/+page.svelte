<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/stores/api';
	import type { StatsOverview, Repertoire, RepertoireStats, HeatmapEntry } from '$lib/types';

	let overview = $state<StatsOverview | null>(null);
	let repertoires = $state<Repertoire[]>([]);
	let repStats = $state<Map<string, RepertoireStats>>(new Map());
	let heatmapData = $state<HeatmapEntry[]>([]);
	let selectedRepId = $state<string | null>(null);
	let loading = $state(true);

	const files = ['a', 'b', 'c', 'd', 'e', 'f', 'g', 'h'];
	const ranks = ['8', '7', '6', '5', '4', '3', '2', '1'];

	onMount(async () => {
		try {
			[overview, repertoires] = await Promise.all([
				api.getOverview(),
				api.listRepertoires()
			]);

			for (const rep of repertoires) {
				const stats = await api.getRepertoireStats(rep.id);
				repStats.set(rep.id, stats);
			}
			repStats = repStats; // trigger reactivity
		} catch (e) {
			console.error('Failed to load stats:', e);
		}
		loading = false;
	});

	async function loadHeatmap(repId: string) {
		selectedRepId = repId;
		try {
			heatmapData = await api.getHeatmap(repId);
		} catch (e) {
			console.error('Failed to load heatmap:', e);
		}
	}

	function getSquareAccuracy(file: string, rank: string): number | null {
		const square = file + rank;
		const entry = heatmapData.find((e) => e.square === square);
		return entry ? entry.accuracy : null;
	}

	function accuracyColor(accuracy: number | null): string {
		if (accuracy === null) return '#f0f0f0';
		const r = Math.round(255 * (1 - accuracy));
		const g = Math.round(255 * accuracy);
		return `rgb(${r}, ${g}, 80)`;
	}
</script>

<div class="dashboard">
	<h1>Dashboard</h1>

	{#if loading}
		<p>Loading...</p>
	{:else if overview}
		<div class="summary-cards">
			<div class="card">
				<span class="card-value">{overview.total_moves}</span>
				<span class="card-label">Total Moves</span>
			</div>
			<div class="card">
				<span class="card-value">{overview.due_today}</span>
				<span class="card-label">Due Today</span>
			</div>
			<div class="card">
				<span class="card-value">{overview.mastered}</span>
				<span class="card-label">Mastered</span>
			</div>
			<div class="card">
				<span class="card-value">{(overview.accuracy_7d * 100).toFixed(0)}%</span>
				<span class="card-label">7-Day Accuracy</span>
			</div>
		</div>

		<h2>Repertoire Progress</h2>
		<div class="rep-list">
			{#each repertoires as rep}
				{@const stats = repStats.get(rep.id)}
				<div class="rep-row">
					<div class="rep-info">
						<span class="rep-name">{rep.name}</span>
						<span class="rep-meta">
							{stats?.move_count ?? 0} moves &middot;
							{stats?.due_count ?? 0} due
						</span>
					</div>
					<div class="progress-bar-wrap">
						<div
							class="progress-bar"
							style="width: {((stats?.mastery_percentage ?? 0) * 100).toFixed(0)}%"
						></div>
						<span class="progress-label">
							{((stats?.mastery_percentage ?? 0) * 100).toFixed(0)}%
						</span>
					</div>
					<div class="rep-actions">
						<a class="drill-btn" href="/train/{rep.id}">
							Drill{stats?.due_count ? ` (${stats.due_count})` : ''}
						</a>
						<button class="heatmap-btn" onclick={() => loadHeatmap(rep.id)}>
							Heatmap
						</button>
					</div>
				</div>
			{/each}
		</div>

		{#if selectedRepId && heatmapData.length > 0}
			<h2>Accuracy Heatmap</h2>
			<div class="heatmap">
				{#each ranks as rank}
					<div class="heatmap-row">
						<span class="rank-label">{rank}</span>
						{#each files as file}
							{@const acc = getSquareAccuracy(file, rank)}
							<div
								class="heatmap-cell"
								style="background: {accuracyColor(acc)}"
								title="{file}{rank}: {acc !== null ? (acc * 100).toFixed(0) + '%' : 'no data'}"
							>
								{#if acc !== null}
									{(acc * 100).toFixed(0)}
								{/if}
							</div>
						{/each}
					</div>
				{/each}
				<div class="heatmap-row file-labels">
					<span class="rank-label"></span>
					{#each files as file}
						<span class="file-label">{file}</span>
					{/each}
				</div>
			</div>
		{/if}
	{/if}
</div>

<style>
	.dashboard {
		max-width: 900px;
		margin: 0 auto;
	}

	h2 {
		margin: 2rem 0 1rem;
	}

	.summary-cards {
		display: grid;
		grid-template-columns: repeat(4, 1fr);
		gap: 1rem;
		margin-bottom: 1rem;
	}

	.card {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 1.25rem;
		text-align: center;
	}

	.card-value {
		display: block;
		font-size: 2rem;
		font-weight: 700;
	}

	.card-label {
		font-size: 0.8rem;
		color: var(--color-muted);
		text-transform: uppercase;
	}

	.rep-list {
		display: grid;
		gap: 0.5rem;
	}

	.rep-row {
		display: flex;
		align-items: center;
		gap: 1rem;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 0.75rem 1rem;
	}

	.rep-info {
		flex: 1;
	}

	.rep-name {
		font-weight: 600;
		display: block;
	}

	.rep-meta {
		font-size: 0.8rem;
		color: var(--color-muted);
	}

	.progress-bar-wrap {
		width: 200px;
		height: 20px;
		background: #eee;
		border-radius: 10px;
		position: relative;
		overflow: hidden;
	}

	.progress-bar {
		height: 100%;
		background: var(--color-success);
		border-radius: 10px;
		transition: width 0.3s;
	}

	.progress-label {
		position: absolute;
		inset: 0;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.7rem;
		font-weight: 600;
	}

	.rep-actions {
		display: flex;
		gap: 0.5rem;
	}

	.drill-btn {
		padding: 0.3rem 0.6rem;
		border: 1px solid var(--color-accent, #4a90d9);
		border-radius: 4px;
		background: var(--color-accent, #4a90d9);
		color: #fff;
		cursor: pointer;
		font-size: 0.8rem;
		text-decoration: none;
		display: flex;
		align-items: center;
		font-weight: 600;
	}

	.drill-btn:hover {
		opacity: 0.9;
	}

	.heatmap-btn {
		padding: 0.3rem 0.6rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-surface);
		cursor: pointer;
		font-size: 0.8rem;
	}

	.heatmap {
		display: inline-block;
		margin-top: 0.5rem;
	}

	.heatmap-row {
		display: flex;
		gap: 2px;
	}

	.rank-label {
		width: 20px;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.75rem;
		color: var(--color-muted);
	}

	.heatmap-cell {
		width: 50px;
		height: 50px;
		display: flex;
		align-items: center;
		justify-content: center;
		font-size: 0.7rem;
		font-weight: 600;
		border-radius: 2px;
	}

	.file-labels {
		margin-top: 2px;
	}

	.file-label {
		width: 50px;
		text-align: center;
		font-size: 0.75rem;
		color: var(--color-muted);
	}

	@media (max-width: 768px) {
		.summary-cards {
			grid-template-columns: repeat(2, 1fr);
		}

		.rep-row {
			flex-wrap: wrap;
		}

		.progress-bar-wrap {
			width: 100%;
		}

		.drill-btn,
		.heatmap-btn {
			min-height: 44px;
			padding: 0.5rem 1rem;
		}

		.heatmap-cell {
			width: 40px;
			height: 40px;
		}

		.file-label {
			width: 40px;
		}
	}
</style>
