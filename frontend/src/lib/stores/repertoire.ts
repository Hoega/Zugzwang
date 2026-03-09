import { writable } from 'svelte/store';
import type { Repertoire, MoveTreeNode } from '$lib/types';

export const currentRepertoire = writable<Repertoire | null>(null);
export const moveTree = writable<MoveTreeNode[]>([]);
export const selectedNodeId = writable<string | null>(null);
