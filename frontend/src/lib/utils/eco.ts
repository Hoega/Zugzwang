import ecoData from '$lib/data/eco.json';

interface EcoEntry { eco: string; name: string; fen: string; }

let fenMap: Map<string, { eco: string; name: string }> | null = null;

function positionKey(fen: string): string {
	return fen.split(' ').slice(0, 4).join(' ');
}

function getMap(): Map<string, { eco: string; name: string }> {
	if (!fenMap) {
		fenMap = new Map();
		for (const entry of ecoData as EcoEntry[]) {
			fenMap.set(positionKey(entry.fen), { eco: entry.eco, name: entry.name });
		}
	}
	return fenMap;
}

export function lookupOpening(fen: string): { eco: string; name: string } | null {
	return getMap().get(positionKey(fen)) ?? null;
}

const ECO_COLORS: Record<string, string> = {
	A: '#6366f1',
	B: '#ec4899',
	C: '#f59e0b',
	D: '#10b981',
	E: '#8b5cf6'
};

export function ecoColor(eco: string): string {
	return ECO_COLORS[eco.charAt(0)] ?? '#6b7280';
}
