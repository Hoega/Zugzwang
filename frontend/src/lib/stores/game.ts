import { writable, derived } from 'svelte/store';
import { Chess } from 'chess.js';
import { STARTING_FEN } from '$lib/utils/chess';

export const fen = writable<string>(STARTING_FEN);
export const currentNodeId = writable<string | null>(null);

export const chess = derived(fen, ($fen) => {
	const c = new Chess($fen);
	return c;
});
