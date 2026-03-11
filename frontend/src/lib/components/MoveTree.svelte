<script lang="ts">
	import type { MoveTreeNode } from '$lib/types';
	import MoveTree from './MoveTree.svelte';

	let { nodes, selectedId = null, onSelect = () => {}, depth = 0, transpositionIds = new Set(), gapIds = new Set() }: {
		nodes: MoveTreeNode[];
		selectedId?: string | null;
		onSelect?: (node: MoveTreeNode) => void;
		depth?: number;
		transpositionIds?: Set<string>;
		gapIds?: Set<string>;
	} = $props();

	const lineColors = ['#6366f1', '#ec4899', '#f59e0b', '#10b981', '#8b5cf6', '#ef4444'];

	type FlatEntry = {
		node: MoveTreeNode;
		variations: MoveTreeNode[];
	};

	type MoveRow = {
		white: FlatEntry | null;
		black: FlatEntry | null;
		moveNumber: number;
	};

	let rows = $derived.by((): MoveRow[] => {
		// Flatten mainline
		const flat: FlatEntry[] = [];
		let current = nodes;
		while (current.length > 0) {
			const node = current[0];
			const variations = current.slice(1);
			flat.push({ node, variations });
			current = node.children;
		}

		// Pair white + black into rows
		const result: MoveRow[] = [];
		let i = 0;
		while (i < flat.length) {
			const entry = flat[i];
			if (entry.node.is_white_move) {
				const row: MoveRow = { white: entry, black: null, moveNumber: entry.node.move_number };
				// Pair with next black move if it follows directly and white has no variations
				if (i + 1 < flat.length && !flat[i + 1].node.is_white_move && entry.variations.length === 0) {
					row.black = flat[i + 1];
					i += 2;
				} else {
					i += 1;
				}
				result.push(row);
			} else {
				// Black move alone (after white had variations, or start of a variation)
				result.push({ white: null, black: entry, moveNumber: entry.node.move_number });
				i += 1;
			}
		}
		return result;
	});
</script>

{#snippet moveBtn(entry: FlatEntry)}
	{#if entry.node.variant_name}
		<button class="variant-badge" onclick={() => onSelect(entry.node)}>{entry.node.variant_name}</button>
	{/if}
	<button
		class="move-btn"
		class:selected={entry.node.id === selectedId}
		class:gap={gapIds.has(entry.node.id)}
		onclick={() => onSelect(entry.node)}
	>
		<span class="san">{entry.node.san_move}</span>
		{#if entry.node.nag}<span class="nag" class:nag-good={entry.node.nag === '!' || entry.node.nag === '!!'} class:nag-bad={entry.node.nag === '?' || entry.node.nag === '??'} class:nag-interesting={entry.node.nag === '!?' || entry.node.nag === '?!'}>{entry.node.nag}</span>{/if}
		{#if transpositionIds.has(entry.node.id)}<span class="badge transposition" title="Transposition">T</span>{/if}
		{#if gapIds.has(entry.node.id)}<span class="badge gap-badge" title="No response prepared">!</span>{/if}
	</button>
{/snippet}

{#snippet variationBlock(entry: FlatEntry)}
	{#each entry.variations as variation}
		<div class="variation" style:border-left-color={lineColors[depth % lineColors.length]}>
			<MoveTree
				nodes={[variation]}
				{selectedId}
				{onSelect}
				depth={depth + 1}
				{transpositionIds}
				{gapIds}
			/>
		</div>
	{/each}
{/snippet}

<div class="move-tree" class:root={depth === 0}>
	{#each rows as row}
		<div class="move-row">
			<span class="move-number">{row.moveNumber}.</span>
			<div class="move-cell">
				{#if row.white}
					{@render moveBtn(row.white)}
				{:else}
					<span class="placeholder">...</span>
				{/if}
			</div>
			<div class="move-cell">
				{#if row.black}
					{@render moveBtn(row.black)}
				{/if}
			</div>
		</div>

		{#if row.white}
			{@render variationBlock(row.white)}
		{/if}
		{#if row.black}
			{@render variationBlock(row.black)}
		{/if}
	{/each}
</div>

<style>
	.move-tree {
		display: flex;
		flex-direction: column;
		padding: 0.5rem 0.75rem;
	}

	.move-tree:not(.root) {
		padding: 0;
	}

	.move-row {
		display: flex;
		align-items: center;
		padding: 1px 0;
	}

	.move-number {
		color: #888;
		min-width: 2.2em;
		text-align: right;
		margin-right: 6px;
		font-size: 0.85rem;
		font-variant-numeric: tabular-nums;
		flex-shrink: 0;
	}

	.move-cell {
		flex: 1;
		display: flex;
		align-items: center;
		min-height: 1.8em;
	}

	.placeholder {
		color: #999;
		padding: 3px 6px;
	}

	.move-btn {
		display: flex;
		align-items: center;
		background: none;
		border: none;
		cursor: pointer;
		padding: 3px 8px;
		border-radius: 4px;
		font-family: inherit;
		font-size: 0.95rem;
	}

	.move-btn:hover {
		background: var(--color-hover, #e0e0e0);
	}

	.move-btn.selected {
		background: var(--color-selected, #b0d0ff);
	}

	.san {
		font-weight: 500;
	}

	.variant-badge {
		display: inline-block;
		background: #e2e8f0;
		color: #4a5568;
		font-size: 0.7rem;
		font-weight: 600;
		padding: 1px 5px;
		border: none;
		border-radius: 3px;
		margin-right: 2px;
		cursor: pointer;
		font-family: inherit;
	}

	.variant-badge:hover {
		background: #cbd5e0;
	}

	.badge {
		display: inline-block;
		font-size: 0.6rem;
		font-weight: 700;
		width: 14px;
		height: 14px;
		line-height: 14px;
		text-align: center;
		border-radius: 50%;
		margin-left: 2px;
	}

	.badge.transposition {
		background: #bee3f8;
		color: #2b6cb0;
	}

	.badge.gap-badge {
		background: #fed7d7;
		color: #c53030;
	}

	.move-btn.gap {
		border-bottom: 2px solid #fc8181;
	}

	.nag {
		display: inline;
		font-weight: 700;
		font-size: 0.8rem;
		margin-left: 1px;
	}

	.nag-good {
		color: #38a169;
	}

	.nag-bad {
		color: #e53e3e;
	}

	.nag-interesting {
		color: #dd6b20;
	}

	.variation {
		display: flex;
		flex-direction: column;
		padding-left: 0.75rem;
		border-left: 3px solid #ccc;
		margin: 4px 0 4px 1.2em;
		font-size: 0.88rem;
		color: #555;
		border-radius: 0 0 0 3px;
	}

	@media (max-width: 768px) {
		.move-btn {
			padding: 6px 10px;
			min-height: 36px;
		}

		.move-cell {
			min-height: 36px;
		}

		.variant-badge {
			padding: 4px 8px;
			min-height: 32px;
		}
	}
</style>
