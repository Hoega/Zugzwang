<script lang="ts">
	import { onMount } from 'svelte';
	import { api } from '$lib/stores/api';
	import type { StatsOverview, Repertoire, RepertoireStats, WeakMove, ActivityResponse } from '$lib/types';

	let overview = $state<StatsOverview | null>(null);
	let repertoires = $state<Repertoire[]>([]);
	let repStats = $state<Map<string, RepertoireStats>>(new Map());
	let weakMoves = $state<Map<string, WeakMove[]>>(new Map());
	let expandedRepId = $state<string | null>(null);
	let loadingWeak = $state(false);
	let activity = $state<ActivityResponse | null>(null);
	let loading = $state(true);

	// Build the 15-week grid from activity data
	let activityGrid = $derived.by(() => {
		if (!activity) return [];
		const countMap = new Map(activity.days.map((d) => [d.date, d.count]));
		const today = new Date();
		const dayOfWeek = today.getDay(); // 0=Sun
		// Start from the Sunday 14 full weeks before this week
		const startDate = new Date(today);
		startDate.setDate(today.getDate() - dayOfWeek - 14 * 7);

		const weeks: { date: string; count: number }[][] = [];
		let current = new Date(startDate);
		while (current <= today) {
			const week: { date: string; count: number }[] = [];
			for (let d = 0; d < 7; d++) {
				if (current > today) {
					week.push({ date: '', count: -1 }); // future
				} else {
					const key = current.toISOString().slice(0, 10);
					week.push({ date: key, count: countMap.get(key) ?? 0 });
				}
				current.setDate(current.getDate() + 1);
			}
			weeks.push(week);
		}
		return weeks;
	});

	onMount(async () => {
		try {
			[overview, repertoires, activity] = await Promise.all([
				api.getOverview(),
				api.listRepertoires(),
				api.getActivity()
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

	async function toggleWeakest(repId: string) {
		if (expandedRepId === repId) {
			expandedRepId = null;
			return;
		}
		expandedRepId = repId;
		if (!weakMoves.has(repId)) {
			loadingWeak = true;
			try {
				const moves = await api.getWeakestMoves(repId);
				weakMoves.set(repId, moves);
				weakMoves = weakMoves;
			} catch (e) {
				console.error('Failed to load weakest moves:', e);
			}
			loadingWeak = false;
		}
	}

	function formatLine(wm: WeakMove): string {
		return wm.line
			.map((m, i) => {
				const prefix = m.is_white_move ? `${m.move_number}.` : i === 0 ? `${m.move_number}...` : '';
				return prefix + m.san_move;
			})
			.join(' ');
	}

	function activityLevel(count: number): string {
		if (count <= 0) return 'level-0';
		if (count < 5) return 'level-1';
		if (count < 15) return 'level-2';
		if (count < 30) return 'level-3';
		return 'level-4';
	}

	function accuracyClass(accuracy: number): string {
		if (accuracy < 0.4) return 'acc-bad';
		if (accuracy < 0.7) return 'acc-mid';
		return 'acc-ok';
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

		{#if activity}
			<div class="activity-section">
				<div class="activity-header">
					<h2>Activity</h2>
					{#if activity.current_streak > 0}
						<span class="streak-badge">{activity.current_streak} day streak</span>
					{/if}
				</div>
				<div class="activity-grid">
					<div class="day-labels">
						<span></span>
						<span>Mon</span>
						<span></span>
						<span>Wed</span>
						<span></span>
						<span>Fri</span>
						<span></span>
					</div>
					{#each activityGrid as week}
						<div class="activity-week">
							{#each week as day}
								{#if day.count === -1}
									<div class="activity-cell future"></div>
								{:else}
									<div
										class="activity-cell {activityLevel(day.count)}"
										title="{day.date}: {day.count} reviews"
									></div>
								{/if}
							{/each}
						</div>
					{/each}
				</div>
			</div>
		{/if}

		<h2>Repertoire Progress</h2>
		<div class="rep-list">
			{#each repertoires as rep}
				{@const stats = repStats.get(rep.id)}
				{@const weak = weakMoves.get(rep.id)}
				<div class="rep-card" class:expanded={expandedRepId === rep.id}>
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
							<button
								class="weakest-btn"
								class:active={expandedRepId === rep.id}
								onclick={() => toggleWeakest(rep.id)}
							>
								Weakest
							</button>
						</div>
					</div>

					{#if expandedRepId === rep.id}
						<div class="weak-section">
							{#if loadingWeak && !weak}
								<p class="weak-loading">Loading...</p>
							{:else if weak && weak.length > 0}
								<div class="weak-list">
									{#each weak as wm}
										<div class="weak-item">
											<div class="weak-line">
												<span class="line-moves">{formatLine(wm)}</span>
												<span class="weak-target">{wm.move_node.san_move}</span>
											</div>
											<div class="weak-stats">
												<span class="acc-badge {accuracyClass(wm.accuracy)}">
													{(wm.accuracy * 100).toFixed(0)}%
												</span>
												<span class="attempt-count">{wm.total_attempts} attempts</span>
												<a class="drill-link" href="/train/{rep.id}?from_move_id={wm.move_node.id}">
													Drill
												</a>
											</div>
										</div>
									{/each}
								</div>
							{:else}
								<p class="weak-empty">No weak moves found (need at least 3 attempts per move)</p>
							{/if}
						</div>
					{/if}
				</div>
			{/each}
		</div>
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

	.rep-card {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		overflow: hidden;
	}

	.rep-row {
		display: flex;
		align-items: center;
		gap: 1rem;
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

	.weakest-btn {
		padding: 0.3rem 0.6rem;
		border: 1px solid var(--color-border);
		border-radius: 4px;
		background: var(--color-surface);
		cursor: pointer;
		font-size: 0.8rem;
	}

	.weakest-btn.active {
		background: var(--color-border);
	}

	/* Weak moves section */
	.weak-section {
		border-top: 1px solid var(--color-border);
		padding: 0.75rem 1rem;
	}

	.weak-loading,
	.weak-empty {
		font-size: 0.85rem;
		color: var(--color-muted);
		margin: 0;
	}

	.weak-list {
		display: grid;
		gap: 0.5rem;
	}

	.weak-item {
		display: flex;
		align-items: center;
		justify-content: space-between;
		gap: 0.75rem;
		padding: 0.4rem 0;
	}

	.weak-item + .weak-item {
		border-top: 1px solid var(--color-border);
		padding-top: 0.5rem;
	}

	.weak-line {
		flex: 1;
		min-width: 0;
	}

	.line-moves {
		font-size: 0.8rem;
		color: var(--color-muted);
		word-break: break-word;
	}

	.weak-target {
		font-weight: 700;
		font-size: 0.85rem;
		margin-left: 0.25rem;
	}

	.weak-stats {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		flex-shrink: 0;
	}

	.acc-badge {
		font-size: 0.75rem;
		font-weight: 700;
		padding: 0.15rem 0.4rem;
		border-radius: 4px;
	}

	.acc-bad {
		background: #fee2e2;
		color: #dc2626;
	}

	.acc-mid {
		background: #fef3c7;
		color: #d97706;
	}

	.acc-ok {
		background: #dcfce7;
		color: #16a34a;
	}

	.attempt-count {
		font-size: 0.75rem;
		color: var(--color-muted);
	}

	.drill-link {
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--color-accent, #4a90d9);
		text-decoration: none;
	}

	.drill-link:hover {
		text-decoration: underline;
	}

	/* Activity grid */
	.activity-section {
		margin-bottom: 1rem;
	}

	.activity-header {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.activity-header h2 {
		margin: 2rem 0 0.75rem;
	}

	.streak-badge {
		font-size: 0.8rem;
		font-weight: 600;
		background: var(--color-accent, #4a90d9);
		color: #fff;
		padding: 0.2rem 0.6rem;
		border-radius: 12px;
		margin-top: 1rem;
	}

	.activity-grid {
		display: flex;
		gap: 3px;
		overflow-x: auto;
		padding-bottom: 0.25rem;
	}

	.day-labels {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.day-labels span {
		height: 13px;
		font-size: 0.6rem;
		color: var(--color-muted);
		display: flex;
		align-items: center;
		line-height: 1;
	}

	.activity-week {
		display: flex;
		flex-direction: column;
		gap: 3px;
	}

	.activity-cell {
		width: 13px;
		height: 13px;
		border-radius: 2px;
	}

	.activity-cell.future {
		visibility: hidden;
	}

	.activity-cell.level-0 {
		background: var(--color-border, #ebedf0);
	}

	.activity-cell.level-1 {
		background: #9be9a8;
	}

	.activity-cell.level-2 {
		background: #40c463;
	}

	.activity-cell.level-3 {
		background: #30a14e;
	}

	.activity-cell.level-4 {
		background: #216e39;
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
		.weakest-btn {
			min-height: 44px;
			padding: 0.5rem 1rem;
		}

		.weak-item {
			flex-direction: column;
			align-items: flex-start;
		}

		.weak-stats {
			width: 100%;
		}

		.drill-link {
			min-height: 44px;
			display: flex;
			align-items: center;
			padding: 0 0.5rem;
		}
	}
</style>
