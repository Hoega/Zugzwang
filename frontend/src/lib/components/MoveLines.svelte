<script lang="ts">
	import type { MoveTreeNode, MoveNode } from '$lib/types';
	import { api } from '$lib/stores/api';
	import { lookupOpening, ecoColor } from '$lib/utils/eco';

	let { nodes, selectedId = null, onSelect = () => {}, transpositionIds = new Set(), gapIds = new Set(), repertoireId = '', onRename, onDelete }: {
		nodes: MoveTreeNode[];
		selectedId?: string | null;
		onSelect?: (node: MoveTreeNode, lineIndex?: number) => void;
		transpositionIds?: Set<string>;
		gapIds?: Set<string>;
		repertoireId?: string;
		onRename?: (updated: MoveNode) => void;
		onDelete?: () => void;
	} = $props();

	let editingLineIdx = $state<number | null>(null);
	let editLabel = $state('');

	type Line = {
		label: string;
		moves: MoveTreeNode[];
		ecoTag: { eco: string; name: string } | null;
	};

	let lines = $derived.by((): Line[] => {
		const result: Line[] = [];
		let lineIndex = 0;

		function dfs(path: MoveTreeNode[], children: MoveTreeNode[]) {
			if (children.length === 0) {
				if (path.length === 0) return;
				// Find deepest variant_name in path
				let label = '';
				for (let i = path.length - 1; i >= 0; i--) {
					if (path[i].variant_name) {
						label = path[i].variant_name!;
						break;
					}
				}
				if (!label) {
					lineIndex++;
					label = `Line ${lineIndex}`;
				}
				let ecoTag: { eco: string; name: string } | null = null;
				for (let i = path.length - 1; i >= 0; i--) {
					const match = lookupOpening(path[i].fen);
					if (match) { ecoTag = match; break; }
				}
				result.push({ label, moves: [...path], ecoTag });
				return;
			}
			for (const child of children) {
				path.push(child);
				dfs(path, child.children);
				path.pop();
			}
		}

		dfs([], nodes);
		return result;
	});

	type MoveRow = {
		white: MoveTreeNode | null;
		black: MoveTreeNode | null;
		moveNumber: number;
	};

	function startEdit(idx: number) {
		editLabel = lines[idx].label;
		editingLineIdx = idx;
	}

	async function saveLineRename(lineIdx: number) {
		const line = lines[lineIdx];
		const trimmed = editLabel.trim();
		editingLineIdx = null;
		if (!trimmed || trimmed === line.label) return;

		// Find which node to tag with variant_name
		let targetNode: MoveTreeNode;

		// Look for existing variant_name bearer (deepest in path)
		let bearer: MoveTreeNode | null = null;
		for (let i = line.moves.length - 1; i >= 0; i--) {
			if (line.moves[i].variant_name) { bearer = line.moves[i]; break; }
		}

		if (bearer) {
			targetNode = bearer;
		} else {
			// Find divergence point from mainline
			const mainline = lines[0]?.moves ?? [];
			let divergeIdx = 0;
			for (let i = 0; i < line.moves.length; i++) {
				if (i >= mainline.length || line.moves[i].id !== mainline[i].id) {
					divergeIdx = i;
					break;
				}
			}
			targetNode = line.moves[divergeIdx];
		}

		try {
			const updated = await api.updateMove(repertoireId, targetNode.id, { variant_name: trimmed });
			onRename?.(updated);
		} catch (e) {
			console.error('Failed to rename line:', e);
		}
	}

	async function deleteLine(idx: number) {
		const line = lines[idx];
		if (!line.moves.length) return;
		if (!confirm(`Delete "${line.label}" and all its moves?`)) return;

		// Find the deepest move whose parent has multiple children
		let branchMove: MoveTreeNode | null = null;
		for (let i = line.moves.length - 1; i >= 0; i--) {
			if (i === 0) {
				if (nodes.length > 1) { branchMove = line.moves[i]; break; }
			} else {
				const parent = line.moves[i - 1];
				if (parent.children.length > 1) { branchMove = line.moves[i]; break; }
			}
		}
		const target = branchMove ?? line.moves[0];

		try {
			await api.deleteMove(repertoireId, target.id);
			onDelete?.();
		} catch (e) {
			console.error('Failed to delete line:', e);
		}
	}

	function pairMoves(moves: MoveTreeNode[]): MoveRow[] {
		const rows: MoveRow[] = [];
		let i = 0;
		while (i < moves.length) {
			const m = moves[i];
			if (m.is_white_move) {
				const row: MoveRow = { white: m, black: null, moveNumber: m.move_number };
				if (i + 1 < moves.length && !moves[i + 1].is_white_move) {
					row.black = moves[i + 1];
					i += 2;
				} else {
					i += 1;
				}
				rows.push(row);
			} else {
				rows.push({ white: null, black: m, moveNumber: m.move_number });
				i += 1;
			}
		}
		return rows;
	}
</script>

{#snippet moveBtn(node: MoveTreeNode, lineIdx: number)}
	<button
		class="move-btn"
		class:selected={node.id === selectedId}
		class:gap={gapIds.has(node.id)}
		onclick={() => onSelect(node, lineIdx)}
	>
		<span class="san">{node.san_move}</span>
		{#if node.nag}<span class="nag" class:nag-good={node.nag === '!' || node.nag === '!!'} class:nag-bad={node.nag === '?' || node.nag === '??'} class:nag-interesting={node.nag === '!?' || node.nag === '?!'}>{node.nag}</span>{/if}
		{#if transpositionIds.has(node.id)}<span class="badge transposition" title="Transposition">T</span>{/if}
		{#if gapIds.has(node.id)}<span class="badge gap-badge" title="No response prepared">!</span>{/if}
	</button>
{/snippet}

{#if lines.length === 0}
	<p class="empty">No moves yet</p>
{:else}
	<div class="move-lines">
		{#each lines as line, idx}
			{#if idx > 0}
				<hr class="line-divider" />
			{/if}
			<div class="line-block">
				<div class="line-header">
					{#if editingLineIdx === idx}
						<!-- svelte-ignore a11y_autofocus -->
						<input
							class="line-label-input"
							bind:value={editLabel}
							autofocus
							onkeydown={(e: KeyboardEvent) => { if (e.key === 'Enter') saveLineRename(idx); if (e.key === 'Escape') editingLineIdx = null; }}
							onblur={() => saveLineRename(idx)}
						/>
					{:else}
						<button class="line-label" onclick={() => startEdit(idx)}>
							{line.label}
							<svg class="pencil-icon" viewBox="0 0 16 16" width="12" height="12"><path fill="currentColor" d="M11.013 1.427a1.75 1.75 0 012.474 0l1.086 1.086a1.75 1.75 0 010 2.474l-8.61 8.61c-.21.21-.47.364-.756.445l-3.251.93a.75.75 0 01-.927-.928l.929-3.25a1.75 1.75 0 01.445-.758l8.61-8.61zm1.414 1.06a.25.25 0 00-.354 0L3.463 11.098a.25.25 0 00-.064.108l-.563 1.97 1.971-.564a.25.25 0 00.108-.064l8.61-8.61a.25.25 0 000-.354l-1.098-1.097z"/></svg>
						</button>
					{/if}
					{#if line.ecoTag}
						<a class="eco-tag" style:background={ecoColor(line.ecoTag.eco)} href="/openings?eco={line.ecoTag.eco}&name={encodeURIComponent(line.ecoTag.name)}">
							{line.ecoTag.eco} · {line.ecoTag.name}
						</a>
					{/if}
					<span class="line-count">{line.moves.length} moves</span>
					<button class="line-delete" title="Delete line" onclick={() => deleteLine(idx)}>✕</button>
				</div>
				{#each pairMoves(line.moves) as row}
					<div class="move-row">
						<span class="move-number">{row.moveNumber}.</span>
						<div class="move-cell">
							{#if row.white}
								{@render moveBtn(row.white, idx)}
							{:else}
								<span class="placeholder">...</span>
							{/if}
						</div>
						<div class="move-cell">
							{#if row.black}
								{@render moveBtn(row.black, idx)}
							{/if}
						</div>
					</div>
				{/each}
			</div>
		{/each}
	</div>
{/if}

<style>
	.move-lines {
		padding: 0.5rem 0.75rem;
	}

	.empty {
		padding: 1rem;
		color: #888;
		text-align: center;
	}

	.line-block {
		margin-bottom: 0.25rem;
	}

	.line-header {
		display: flex;
		align-items: center;
		gap: 0.4rem;
		padding: 0.25rem 0;
	}

	.line-label {
		all: unset;
		font-size: 0.8rem;
		font-weight: 600;
		color: var(--color-text);
		cursor: pointer;
		display: inline-flex;
		align-items: center;
		gap: 4px;
		border-bottom: 1px dashed transparent;
	}

	.line-label:hover {
		border-bottom-color: var(--color-muted, #999);
	}

	.pencil-icon {
		opacity: 0;
		transition: opacity 0.15s;
		color: var(--color-muted, #999);
	}

	.line-label:hover .pencil-icon {
		opacity: 1;
	}

	.line-label-input {
		font-size: 0.8rem;
		font-weight: 600;
		padding: 0 0.25rem;
		border: 1px solid var(--color-primary, #3b82f6);
		border-radius: 3px;
		outline: none;
		background: transparent;
		color: var(--color-text);
		width: 10rem;
	}

	.line-count {
		font-size: 0.7rem;
		color: #888;
		margin-left: auto;
	}

	.line-delete {
		all: unset;
		cursor: pointer;
		font-size: 0.75rem;
		color: #999;
		padding: 2px 4px;
		border-radius: 3px;
		opacity: 0;
		transition: opacity 0.15s, color 0.15s;
	}

	.line-block:hover .line-delete {
		opacity: 1;
	}

	.line-delete:hover {
		color: #e53e3e;
		background: rgba(229, 62, 62, 0.1);
	}

	.line-divider {
		border: none;
		border-top: 1px solid var(--color-border);
		margin: 0.4rem 0;
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

	.eco-tag {
		font-size: 0.68rem;
		font-weight: 600;
		color: white;
		padding: 1px 6px;
		border-radius: 3px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
		max-width: 12rem;
		text-decoration: none;
	}

	.eco-tag:hover {
		opacity: 0.85;
	}

	@media (max-width: 768px) {
		.move-btn {
			padding: 6px 10px;
			min-height: 36px;
		}

		.move-cell {
			min-height: 36px;
		}

		.line-delete {
			opacity: 1;
		}

		.pencil-icon {
			opacity: 0.5;
		}

		.line-header {
			flex-wrap: wrap;
		}

		.eco-tag {
			font-size: 0.75rem;
		}
	}
</style>
