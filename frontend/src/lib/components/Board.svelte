<script lang="ts">
	import { onMount } from 'svelte';
	import { Chessground } from 'chessground';
	import type { Api } from 'chessground/api';
	import type { Config } from 'chessground/config';

	import type { DrawShape } from 'chessground/draw';

	let { config = {}, onMove, shapes = [] }: { config?: Config; onMove?: (orig: string, dest: string) => void; shapes?: DrawShape[] } = $props();

	let boardEl: HTMLDivElement | undefined = $state();
	let ground: Api | undefined = $state();

	onMount(() => {
		if (!boardEl) return;
		const baseConfig: Config = {
			...config,
			movable: {
				...config.movable,
				free: false,
				events: {
					after: (orig, dest) => {
						onMove?.(orig, dest);
					}
				}
			}
		};
		ground = Chessground(boardEl, baseConfig);

		return () => {
			ground?.destroy();
		};
	});

	$effect(() => {
		if (ground && config) {
			ground.set({
				...config,
				movable: {
					...config.movable,
					events: {
						after: (orig, dest) => {
							onMove?.(orig, dest);
						}
					}
				}
			});
		}
	});

	$effect(() => {
		if (ground) {
			ground.setAutoShapes(shapes);
		}
	});
</script>

<div class="board-wrap">
	<div bind:this={boardEl} class="board"></div>
</div>

<style>
	@import 'chessground/assets/chessground.base.css';
	@import 'chessground/assets/chessground.brown.css';
	@import 'chessground/assets/chessground.cburnett.css';

	.board-wrap {
		width: 100%;
		max-width: 560px;
		aspect-ratio: 1;
	}

	.board {
		width: 100%;
		height: 100%;
	}

	/* Make board coordinates more visible */
	.board :global(.cg-wrap coords) {
		font-size: 11px;
		font-weight: 600;
		opacity: 1;
		text-shadow: 0 1px 2px rgba(0, 0, 0, 0.3);
	}
</style>
