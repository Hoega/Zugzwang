<script lang="ts">
	import { onMount } from 'svelte';
	import { page } from '$app/stores';
	import { Chess } from 'chess.js';
	import Board from '$lib/components/Board.svelte';
	import EvalBar from '$lib/components/EvalBar.svelte';
	import { api } from '$lib/stores/api';
	import { Engine } from '$lib/utils/engine';
	import { toDests, turnColor, STARTING_FEN } from '$lib/utils/chess';
	import type { DrillPosition } from '$lib/types';
	import type { Config } from 'chessground/config';
	import type { DrawShape } from 'chessground/draw';
	import type { Key } from 'chessground/types';

	let positions = $state<DrillPosition[]>([]);
	let totalDue = $state(0);
	let currentIndex = $state(0);
	let currentFen = $state(STARTING_FEN);
	let moveIndex = $state(0);
	let startTime = $state(0);
	let feedback = $state<'correct' | 'incorrect' | null>(null);
	let sessionCorrect = $state(0);
	let sessionIncorrect = $state(0);
	let phase = $state<'loading' | 'selecting' | 'drilling' | 'waiting' | 'done'>('loading');
	let repertoireColor = $state<'white' | 'black'>('white');
	let repertoireName = $state('');
	let variants = $state<string[]>([]);
	let selectedVariant = $state<string | undefined>(undefined);
	let fromMoveId = $state<string | undefined>(undefined);
	let drillMode = $state<'due' | 'all'>('due');
	let animating = $state(false);
	let hadIncorrectAttempt = $state(false);
	let hintSquare = $state<Key | null>(null);
	let continueFromPosition = $state(true);
	let previousLine = $state<{ id: string; fen: string }[]>([]);
	let evalScore = $state<number | null>(null);
	let evalMate = $state<number | null>(null);
	let engine: Engine | null = null;
	let engineReady = $state(false);

	// Exploration mode state
	let exploring = $state(false);
	let exploreHistory = $state<string[]>([]);
	let exploreIndex = $state(0);

	let repertoireId = $derived($page.params.id);
	let currentPosition = $derived(positions[currentIndex] ?? null);

	let lineName = $derived.by(() => {
		const pos = currentPosition;
		if (!pos) return null;
		for (const m of pos.line) {
			if (m.variant_name) return m.variant_name;
		}
		return null;
	});

	// During exploration, which line move index is currently viewed (-1 if viewing a free move)
	let exploreLineIndex = $derived.by(() => {
		if (!exploring || !currentPosition) return -1;
		const pos = currentPosition;
		const fen = exploreHistory[exploreIndex];
		// Check if this FEN matches a line position (starting FEN = index -1, first move FEN = index 0, etc.)
		if (fen === STARTING_FEN) return -1;
		for (let i = 0; i < pos.line.length; i++) {
			if (pos.line[i].fen === fen) return i;
		}
		return -1; // free move, not in the line
	});

	let pgnRows = $derived.by(() => {
		const pos = currentPosition;
		if (!pos) return [];
		const rows: { num: number; white?: { san: string; isPlayed: boolean; isTarget: boolean; isActive: boolean }; black?: { san: string; isPlayed: boolean; isTarget: boolean; isActive: boolean } }[] = [];
		for (let i = 0; i < pos.line.length; i++) {
			const move = pos.line[i];
			const isActive = exploring ? exploreLineIndex === i : i === moveIndex - 1;
			const entry = {
				san: move.san_move,
				isPlayed: i < moveIndex,
				isTarget: move.id === pos.move_node.id,
				isActive
			};
			if (move.is_white_move) {
				rows.push({ num: move.move_number, white: entry });
			} else {
				const last = rows[rows.length - 1];
				if (last && last.num === move.move_number && !last.black) {
					last.black = entry;
				} else {
					rows.push({ num: move.move_number, black: entry });
				}
			}
		}
		return rows;
	});

	let displayFen = $derived(exploring ? exploreHistory[exploreIndex] : currentFen);

	let boardConfig = $derived.by((): Config => {
		const fen = displayFen;
		const chess = new Chess(fen);
		const color = turnColor(fen);
		const isUserTurn = color === repertoireColor;
		const canMove = exploring
			? true  // any legal move allowed during exploration
			: phase === 'drilling' && isUserTurn && !animating;

		return {
			fen,
			orientation: repertoireColor,
			turnColor: color,
			movable: {
				color: canMove ? color : undefined,
				free: false,
				dests: canMove ? toDests(chess) : new Map()
			}
		};
	});

	let hintShapes = $derived<DrawShape[]>(
		hintSquare ? [{ orig: hintSquare, brush: 'blue' }] : []
	);

	function showHint() {
		const pos = currentPosition;
		if (!pos || phase !== 'drilling') return;
		const uci = pos.move_node.uci_move;
		hintSquare = uci.slice(0, 2) as Key;
	}

	onMount(async () => {
		engine = new Engine();
		engine.init().then(() => { engineReady = true; }).catch(() => {});

		try {
			const rep = await api.getRepertoire(repertoireId);
			repertoireColor = rep.color as 'white' | 'black';
			repertoireName = rep.name;

			// Check for mode in URL params
			const urlMode = $page.url.searchParams.get('mode');
			if (urlMode === 'all') drillMode = 'all';

			// Check for from_move_id in URL params
			const urlFromMoveId = $page.url.searchParams.get('from_move_id');
			if (urlFromMoveId) {
				fromMoveId = urlFromMoveId;
				await loadBatch();
				return;
			}

			variants = await api.getVariants(repertoireId);
			phase = 'selecting';
		} catch (e) {
			console.error('Failed to start drill:', e);
		}

		return () => {
			engine?.destroy();
		};
	});

	$effect(() => {
		const isExploring = exploring;
		const eIdx = exploreIndex;
		const eHist = exploreHistory;
		const cFen = currentFen;
		const isAnimating = animating;
		const fen = isExploring ? eHist[eIdx] : cFen;
		if (!engineReady || !engine || !fen || isAnimating) return;
		engine.evaluate(fen, (result) => {
			evalScore = result.score;
			evalMate = result.mate;
		});
	});

	async function startWithVariant(variant?: string) {
		selectedVariant = variant;
		await loadBatch();
	}

	async function loadBatch() {
		phase = 'loading';
		try {
			const batch = await api.getNextBatch(repertoireId, selectedVariant, fromMoveId, drillMode);
			positions = batch.positions;
			totalDue = batch.total_due;

			if (positions.length === 0) {
				phase = 'done';
				return;
			}

			currentIndex = 0;
			sessionCorrect = 0;
			sessionIncorrect = 0;
			previousLine = [];
			startDrillPosition();
		} catch (e) {
			console.error('Failed to load batch:', e);
			phase = 'done';
		}
	}

	function startDrillPosition() {
		const pos = positions[currentIndex];
		if (!pos) {
			phase = 'done';
			return;
		}

		feedback = null;
		hadIncorrectAttempt = false;
		hintSquare = null;

		// Find common prefix with previous line to skip shared moves
		let skipTo = 0;
		if (continueFromPosition && previousLine.length > 0) {
			const line = pos.line;
			while (skipTo < previousLine.length && skipTo < line.length && previousLine[skipTo].id === line[skipTo].id) {
				skipTo++;
			}
		}

		if (skipTo > 0) {
			// Continue from the end of the shared prefix
			currentFen = previousLine[skipTo - 1].fen;
			moveIndex = skipTo;
		} else {
			currentFen = STARTING_FEN;
			moveIndex = 0;
		}

		// Save current line for next comparison
		previousLine = pos.line.map((m) => ({ id: m.id, fen: m.fen }));

		phase = 'drilling';
		playOpponentMoves();
	}

	async function playOpponentMoves() {
		const pos = positions[currentIndex];
		if (!pos) return;
		const line = pos.line;

		animating = true;
		while (moveIndex < line.length) {
			const move = line[moveIndex];
			const isTargetMove = move.id === pos.move_node.id;

			if (isTargetMove) {
				animating = false;
				startTime = Date.now();
				return;
			}

			await delay(500);
			currentFen = move.fen;
			moveIndex++;
		}
		animating = false;
	}

	function delay(ms: number): Promise<void> {
		return new Promise((resolve) => setTimeout(resolve, ms));
	}

	function enterExploration() {
		if (!currentPosition || animating) return;
		const pos = currentPosition;
		// Build history from starting FEN through played moves
		const history = [STARTING_FEN];
		for (let i = 0; i < moveIndex; i++) {
			history.push(pos.line[i].fen);
		}
		exploreHistory = history;
		exploreIndex = history.length - 2; // go back one step
		if (exploreIndex < 0) exploreIndex = 0;
		exploring = true;
	}

	function exitExploration() {
		exploring = false;
		exploreHistory = [];
		exploreIndex = 0;
	}

	function handleKeydown(e: KeyboardEvent) {
		if (phase !== 'drilling' && phase !== 'waiting') return;
		if (animating) return;

		if (e.key === 'ArrowLeft') {
			e.preventDefault();
			if (!exploring) {
				if (moveIndex > 0) enterExploration();
			} else if (exploreIndex > 0) {
				exploreIndex--;
			}
		} else if (e.key === 'ArrowRight') {
			e.preventDefault();
			if (exploring && exploreIndex < exploreHistory.length - 1) {
				exploreIndex++;
				// If we've returned to the drill position, exit exploration
				if (exploreHistory[exploreIndex] === currentFen) {
					exitExploration();
				}
			}
		} else if (e.key === 'Escape') {
			if (exploring) {
				e.preventDefault();
				exitExploration();
			}
		}
	}

	function handleExploreMove(orig: string, dest: string) {
		const fen = exploreHistory[exploreIndex];
		const chess = new Chess(fen);
		const move = chess.move({ from: orig, to: dest, promotion: 'q' });
		if (!move) return;
		// Truncate history after current position and add new FEN
		exploreHistory = [...exploreHistory.slice(0, exploreIndex + 1), chess.fen()];
		exploreIndex = exploreHistory.length - 1;
	}

	async function handleMove(orig: string, dest: string) {
		if (exploring) {
			handleExploreMove(orig, dest);
			return;
		}
		const pos = positions[currentIndex];
		if (!pos || phase !== 'drilling') return;
		hintSquare = null;

		const targetMove = pos.move_node;
		const previousFen = currentFen;
		const chess = new Chess(currentFen);
		const move = chess.move({ from: orig, to: dest, promotion: 'q' });
		if (!move) return;

		const playedUci = orig + dest + (move.promotion || '');
		const responseTime = Date.now() - startTime;
		const isCorrect = playedUci === targetMove.uci_move;

		if (isCorrect) {
			feedback = 'correct';
			if (!hadIncorrectAttempt) sessionCorrect++;
			currentFen = targetMove.fen;
			moveIndex++;

			try {
				await api.submitReview({
					move_id: targetMove.id,
					response_time_ms: responseTime,
					played_uci: playedUci,
					was_correct: !hadIncorrectAttempt
				});
			} catch (e) {
				console.error('Failed to submit review:', e);
			}

			phase = 'waiting';
			await delay(1000);

			currentIndex++;
			if (currentIndex < positions.length) {
				startDrillPosition();
			} else {
				phase = 'done';
			}
		} else {
			feedback = 'incorrect';
			if (!hadIncorrectAttempt) {
				hadIncorrectAttempt = true;
				sessionIncorrect++;
			}
			phase = 'waiting';
			await delay(800);
			currentFen = previousFen;
			feedback = null;
			phase = 'drilling';
			startTime = Date.now();
		}
	}
</script>

<svelte:window onkeydown={handleKeydown} />

<div class="drill-page">
	<div class="drill-header">
		<h2>Drill Mode</h2>
		{#if phase === 'drilling' || phase === 'waiting'}
			<span class="progress">
				{currentIndex + 1} / {positions.length}
				({totalDue} {drillMode === 'all' ? 'total moves' : 'total due'})
			</span>
		{/if}
	</div>

	{#if phase === 'selecting'}
		<div class="variant-selector">
			<div class="mode-toggle">
				<button class="mode-btn" class:active={drillMode === 'due'} onclick={() => drillMode = 'due'}>Due Only</button>
				<button class="mode-btn" class:active={drillMode === 'all'} onclick={() => drillMode = 'all'}>Drill All</button>
			</div>
			<h3>Select variant to drill</h3>
			<div class="variant-options">
				<button class="variant-btn" onclick={() => startWithVariant()}>All Variants</button>
				{#each variants as v}
					<button class="variant-btn" onclick={() => startWithVariant(v)}>{v}</button>
				{/each}
			</div>
		</div>
	{/if}

	<div class="drill-layout" class:hidden={phase === 'selecting'}>
		<div class="pgn-panel">
			<h3>{repertoireName}{lineName ? ` — ${lineName}` : ''}</h3>
			<div class="pgn-rows">
				{#each pgnRows as row}
					<div class="pgn-row">
						<span class="move-number">{row.num}.</span>
						<span
							class="pgn-move"
							class:played={row.white?.isPlayed}
							class:target={row.white?.isTarget}
							class:future={row.white && !row.white.isPlayed && !row.white.isTarget}
							class:active={row.white?.isActive}
						>{row.white?.isTarget && !row.white?.isPlayed ? '??' : row.white?.san ?? ''}</span>
						<span
							class="pgn-move"
							class:played={row.black?.isPlayed}
							class:target={row.black?.isTarget}
							class:future={row.black && !row.black.isPlayed && !row.black.isTarget}
							class:active={row.black?.isActive}
						>{row.black?.isTarget && !row.black?.isPlayed ? '??' : row.black?.san ?? ''}</span>
					</div>
				{/each}
			</div>
		</div>

		<div class="board-area">
			{#if feedback}
				<div class="feedback" class:correct={feedback === 'correct'} class:incorrect={feedback === 'incorrect'}>
					{feedback === 'correct' ? 'Correct!' : 'Incorrect'}
				</div>
			{/if}
			<div class="board-with-eval">
				<EvalBar score={evalScore} mate={evalMate} orientation={repertoireColor} />
				<Board config={boardConfig} onMove={handleMove} shapes={hintShapes} />
			</div>
		</div>

		<div class="info-panel">
			{#if exploring}
				<div class="explore-indicator">
					<p class="explore-label">Exploring</p>
					<p class="explore-hint">Use arrow keys to navigate, play any move</p>
					<button class="resume-btn" onclick={exitExploration}>Resume Drill (Esc)</button>
				</div>
			{:else if phase === 'loading'}
				<p>Loading positions...</p>
			{:else if phase === 'drilling'}
				<p>Your move!</p>
				{#if currentPosition}
					<p class="hint">Move {currentPosition.move_node.move_number}</p>
				{/if}
				<button class="hint-btn" onclick={showHint}>Hint</button>
				<label class="toggle-label">
					<input type="checkbox" bind:checked={continueFromPosition} onchange={() => { if (!continueFromPosition) previousLine = []; }} />
					Continue from position
				</label>
			{:else if phase === 'done'}
				<div class="summary">
					<h3>Session Complete</h3>
					<div class="stats">
						<div class="stat correct">
							<span class="stat-value">{sessionCorrect}</span>
							<span class="stat-label">Correct</span>
						</div>
						<div class="stat incorrect">
							<span class="stat-value">{sessionIncorrect}</span>
							<span class="stat-label">Incorrect</span>
						</div>
					</div>
					{#if totalDue > positions.length}
						<button class="primary" onclick={loadBatch}>
							Continue ({totalDue - positions.length} remaining)
						</button>
					{/if}
					<a href="/repertoire/{repertoireId}" class="back-link">Back to Repertoire</a>
				</div>
			{/if}
		</div>
	</div>
</div>

<style>
	.drill-page {
		max-width: 1100px;
		margin: 0 auto;
	}

	.drill-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 1rem;
	}

	.progress {
		color: var(--color-muted);
		font-size: 0.9rem;
	}

	.drill-layout {
		display: grid;
		grid-template-columns: 200px 1fr 250px;
		gap: 1.5rem;
		align-items: start;
	}

	.pgn-panel {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 1rem;
	}

	.pgn-panel h3 {
		margin: 0 0 0.75rem;
		font-size: 0.95rem;
	}

	.pgn-rows {
		display: flex;
		flex-direction: column;
		gap: 0.15rem;
		font-size: 0.9rem;
	}

	.pgn-row {
		display: grid;
		grid-template-columns: 2rem 1fr 1fr;
		gap: 0.25rem;
		align-items: center;
	}

	.move-number {
		color: var(--color-muted);
		font-size: 0.85rem;
		text-align: right;
	}

	.pgn-move {
		padding: 0.15rem 0.35rem;
		border-radius: 3px;
	}

	.pgn-move.played {
		color: var(--color-text);
	}

	.pgn-move.target {
		background: var(--color-primary);
		color: white;
		font-weight: 600;
	}

	.pgn-move.future {
		color: var(--color-muted);
		opacity: 0.4;
	}

	.pgn-move.active {
		outline: 2px solid var(--color-primary);
		outline-offset: 1px;
		border-radius: 3px;
	}

	.board-area {
		position: relative;
	}

	.board-with-eval {
		display: grid;
		grid-template-columns: 28px 1fr;
		gap: 0.35rem;
	}

	.feedback {
		position: absolute;
		top: 50%;
		left: 50%;
		transform: translate(-50%, -50%);
		z-index: 10;
		padding: 0.5rem 1.5rem;
		border-radius: 8px;
		font-weight: 700;
		font-size: 1.5rem;
		pointer-events: none;
		animation: fadeout 1s forwards;
	}

	.feedback.correct {
		background: rgba(56, 161, 105, 0.9);
		color: white;
	}

	.feedback.incorrect {
		background: rgba(229, 62, 62, 0.9);
		color: white;
	}

	@keyframes fadeout {
		0% { opacity: 1; }
		70% { opacity: 1; }
		100% { opacity: 0; }
	}

	.info-panel {
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 8px;
		padding: 1.5rem;
	}

	.hint {
		color: var(--color-muted);
		font-size: 0.9rem;
	}

	.hint-btn {
		margin-top: 0.75rem;
		padding: 0.4rem 1rem;
		background: var(--color-surface);
		border: 1px solid var(--color-border);
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.85rem;
		color: var(--color-text);
	}

	.hint-btn:hover {
		background: var(--color-border);
	}

	.toggle-label {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		margin-top: 0.75rem;
		font-size: 0.85rem;
		color: var(--color-muted);
		cursor: pointer;
	}

	.toggle-label input {
		cursor: pointer;
	}

	.summary h3 {
		margin-bottom: 1rem;
	}

	.stats {
		display: flex;
		gap: 1rem;
		margin-bottom: 1.5rem;
	}

	.stat {
		flex: 1;
		text-align: center;
		padding: 1rem;
		border-radius: 8px;
	}

	.stat.correct {
		background: #f0fff4;
	}

	.stat.incorrect {
		background: #fff5f5;
	}

	.stat-value {
		display: block;
		font-size: 2rem;
		font-weight: 700;
	}

	.stat-label {
		font-size: 0.8rem;
		color: var(--color-muted);
	}

	button.primary {
		display: block;
		width: 100%;
		padding: 0.6rem;
		background: var(--color-primary);
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		margin-bottom: 0.75rem;
	}

	.back-link {
		display: block;
		text-align: center;
		font-size: 0.9rem;
	}

	.hidden {
		display: none;
	}

	.variant-selector {
		max-width: 500px;
		margin: 2rem auto;
		text-align: center;
	}

	.mode-toggle {
		display: flex;
		justify-content: center;
		gap: 0;
		margin-bottom: 1.25rem;
		border: 1px solid var(--color-border);
		border-radius: 6px;
		overflow: hidden;
		display: inline-flex;
	}

	.mode-btn {
		padding: 0.5rem 1.25rem;
		border: none;
		background: var(--color-surface);
		cursor: pointer;
		font-size: 0.9rem;
		color: var(--color-muted);
		transition: background 0.15s, color 0.15s;
	}

	.mode-btn.active {
		background: var(--color-primary);
		color: white;
		font-weight: 600;
	}

	.variant-selector h3 {
		margin-bottom: 1rem;
	}

	.variant-options {
		display: flex;
		flex-wrap: wrap;
		gap: 0.75rem;
		justify-content: center;
	}

	.variant-btn {
		padding: 0.5rem 1rem;
		border: 1px solid var(--color-border);
		border-radius: 6px;
		background: var(--color-surface);
		cursor: pointer;
		font-size: 0.9rem;
	}

	.variant-btn:hover {
		background: var(--color-primary);
		color: white;
		border-color: var(--color-primary);
	}

	.explore-indicator {
		text-align: center;
	}

	.explore-label {
		font-weight: 700;
		font-size: 1.1rem;
		color: var(--color-primary);
		margin-bottom: 0.5rem;
	}

	.explore-hint {
		font-size: 0.85rem;
		color: var(--color-muted);
		margin-bottom: 1rem;
	}

	.resume-btn {
		width: 100%;
		padding: 0.5rem 1rem;
		background: var(--color-primary);
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.9rem;
	}

	.resume-btn:hover {
		opacity: 0.9;
	}

	@media (max-width: 768px) {
		.drill-layout {
			grid-template-columns: 1fr;
		}

		.board-area {
			order: -1;
		}

		.board-with-eval {
			max-width: 100%;
		}

		.pgn-move {
			padding: 0.3rem 0.5rem;
		}

		.mode-btn {
			min-height: 44px;
			padding: 0.6rem 1.25rem;
		}

		.variant-btn {
			min-height: 44px;
			padding: 0.6rem 1rem;
		}

		.hint-btn {
			min-height: 44px;
			padding: 0.5rem 1rem;
		}

		.resume-btn {
			min-height: 44px;
		}

		button.primary {
			min-height: 44px;
		}

		.pgn-panel h3 {
			overflow: hidden;
			text-overflow: ellipsis;
			white-space: nowrap;
		}
	}
</style>
