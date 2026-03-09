import { writable, derived } from 'svelte/store';
import type { DrillPosition } from '$lib/types';

export const drillQueue = writable<DrillPosition[]>([]);
export const currentIndex = writable<number>(0);
export const drillStartTime = writable<number>(0);
export const sessionResults = writable<{ correct: number; incorrect: number }>({
	correct: 0,
	incorrect: 0
});

export const currentDrill = derived(
	[drillQueue, currentIndex],
	([$queue, $index]) => $queue[$index] ?? null
);

export const drillProgress = derived(
	[currentIndex, drillQueue],
	([$index, $queue]) => ({
		current: $index + 1,
		total: $queue.length
	})
);
