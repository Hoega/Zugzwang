<script lang="ts">
	import type { MoveNode } from '$lib/types';
	import { api } from '$lib/stores/api';

	let { move = null, repertoireId, onUpdate = () => {} }: {
		move?: MoveNode | null;
		repertoireId: string;
		onUpdate?: (updated: MoveNode) => void;
	} = $props();

	let comment = $state('');
	let nag = $state('');
	let variantName = $state('');
	let saving = $state(false);

	$effect(() => {
		if (move) {
			comment = move.comment ?? '';
			nag = move.nag ?? '';
			variantName = move.variant_name ?? '';
		}
	});

	async function save() {
		if (!move) return;
		saving = true;
		try {
			const data: { comment: string; nag: string; variant_name?: string } = { comment, nag };
			if (variantName !== (move.variant_name ?? '')) {
				data.variant_name = variantName;
			}
			const updated = await api.updateMove(repertoireId, move.id, data);
			onUpdate(updated);
		} catch (e) {
			console.error('Failed to save annotation:', e);
		}
		saving = false;
	}
</script>

{#if move}
	<div class="annotation-editor">
		<h3>{move.san_move}</h3>

		<label>
			Comment
			<textarea bind:value={comment} rows={3} placeholder="Add a comment..."></textarea>
		</label>

		<label>
			Variant Name
			<div class="variant-row">
				<input type="text" bind:value={variantName} placeholder="e.g. Exchange Variation" />
				{#if move.variant_name}
					<button class="remove-btn" type="button" onclick={() => { variantName = ''; save(); }} title="Remove variant name">✕</button>
				{/if}
			</div>
		</label>

		<label>
			NAG
			<select bind:value={nag}>
				<option value="">None</option>
				<option value="!">! Good move</option>
				<option value="!!">!! Brilliant</option>
				<option value="?">? Mistake</option>
				<option value="??">?? Blunder</option>
				<option value="!?">!? Interesting</option>
				<option value="?!">?! Dubious</option>
			</select>
		</label>

		<button onclick={save} disabled={saving}>
			{saving ? 'Saving...' : 'Save'}
		</button>
	</div>
{:else}
	<div class="annotation-editor empty">
		<p>Select a move to annotate</p>
	</div>
{/if}

<style>
	.annotation-editor {
		padding: 1rem;
	}

	.annotation-editor.empty {
		color: #999;
		font-style: italic;
	}

	h3 {
		margin: 0 0 0.5rem;
	}

	label {
		display: block;
		margin-bottom: 0.75rem;
		font-size: 0.85rem;
		font-weight: 500;
	}

	input[type='text'],
	textarea,
	select {
		display: block;
		width: 100%;
		margin-top: 0.25rem;
		padding: 0.5rem;
		border: 1px solid #ccc;
		border-radius: 4px;
		font-family: inherit;
	}

	.variant-row {
		display: flex;
		gap: 0.25rem;
		margin-top: 0.25rem;
	}

	.variant-row input {
		flex: 1;
		margin-top: 0;
	}

	.remove-btn {
		padding: 0.25rem 0.5rem;
		background: #e53e3e;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.8rem;
		flex-shrink: 0;
	}

	.remove-btn:hover {
		background: #c53030;
	}

	button {
		padding: 0.5rem 1rem;
		background: #4a90d9;
		color: white;
		border: none;
		border-radius: 4px;
		cursor: pointer;
	}

	button:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}
</style>
