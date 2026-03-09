import { Chess, type Move, type Square } from 'chess.js';
import type { Key } from 'chessground/types';

export function toDests(chess: Chess): Map<Key, Key[]> {
	const dests = new Map<Key, Key[]>();
	for (const move of chess.moves({ verbose: true })) {
		const from = move.from as Key;
		const to = move.to as Key;
		if (!dests.has(from)) {
			dests.set(from, []);
		}
		dests.get(from)!.push(to);
	}
	return dests;
}

export function toUci(move: Move): string {
	return move.from + move.to + (move.promotion || '');
}

export function turnColor(fen: string): 'white' | 'black' {
	const parts = fen.split(' ');
	return parts[1] === 'w' ? 'white' : 'black';
}

export const STARTING_FEN = 'rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1';
