import type { DrawShape } from 'chessground/draw';
import type { Key } from 'chessground/types';
import type { MultiPvLine } from './engine';

const arrowBrushes = ['green', 'yellow', 'blue', 'paleGreen', 'paleRed'];
const arrowColors = ['#15781B', '#e6bc21', '#003088', '#15781B80', '#88202080'];
const lineWidths = [15, 12, 10, 8, 7];

function fileIndex(c: string): number {
	return c.charCodeAt(0) - 'a'.charCodeAt(0);
}

function rankIndex(c: string): number {
	return parseInt(c) - 1;
}

export function isKnightMove(uci: string): boolean {
	const dx = Math.abs(fileIndex(uci[2]) - fileIndex(uci[0]));
	const dy = Math.abs(rankIndex(uci[3]) - rankIndex(uci[1]));
	return dx + dy === 3 && dx !== 0 && dy !== 0;
}

function knightArrowSvg(
	uci: string,
	color: string,
	orientation: 'white' | 'black',
	lineWidth: number
): string {
	const f0 = fileIndex(uci[0]);
	const r0 = rankIndex(uci[1]);
	const f1 = fileIndex(uci[2]);
	const r1 = rankIndex(uci[3]);

	// Delta in board coordinates (file right = +, rank up = +)
	let dx = f1 - f0;
	let dy = r1 - r0;

	// If board is flipped, coordinates are mirrored
	if (orientation === 'black') {
		dx = -dx;
		dy = -dy;
	}

	// SVG coordinates: x right = +, y down = +
	// So svgDx = dx * 100, svgDy = -dy * 100
	const svgDx = dx * 100;
	const svgDy = -dy * 100;

	// L-shape: long leg first (the axis with |delta| == 2), then short leg
	let midX: number, midY: number;
	if (Math.abs(dx) === 2) {
		// Long leg along x-axis first
		midX = 50 + svgDx;
		midY = 50;
	} else {
		// Long leg along y-axis first
		midX = 50;
		midY = 50 + svgDy;
	}

	const endX = 50 + svgDx;
	const endY = 50 + svgDy;

	// Scale stroke and arrowhead from lineWidth (matching chessground's per-square units)
	const scale = lineWidth / 64 * 100;
	const strokeWidth = scale;
	const headSize = 3 * scale;
	// Direction of last segment (from mid to end)
	const lastDx = endX - midX;
	const lastDy = endY - midY;
	const len = Math.sqrt(lastDx * lastDx + lastDy * lastDy);
	const ux = lastDx / len;
	const uy = lastDy / len;
	// Perpendicular
	const px = -uy;
	const py = ux;

	const tipX = endX;
	const tipY = endY;
	const baseX = endX - ux * headSize;
	const baseY = endY - uy * headSize;
	const leftX = baseX + px * headSize * 0.66;
	const leftY = baseY + py * headSize * 0.66;
	const rightX = baseX - px * headSize * 0.66;
	const rightY = baseY - py * headSize * 0.66;

	// Shorten the polyline so it doesn't poke through the arrowhead
	const shortenedEndX = endX - ux * headSize * 0.5;
	const shortenedEndY = endY - uy * headSize * 0.5;

	return `<svg xmlns="http://www.w3.org/2000/svg" overflow="visible"><polyline points="${50},${50} ${midX},${midY} ${shortenedEndX},${shortenedEndY}" fill="none" stroke="${color}" stroke-width="${strokeWidth}" stroke-linejoin="round" stroke-linecap="round" opacity="0.6"/><polygon points="${tipX},${tipY} ${leftX},${leftY} ${rightX},${rightY}" fill="${color}" opacity="0.6"/></svg>`;
}

export function makeEngineArrows(
	lines: MultiPvLine[],
	orientation: 'white' | 'black'
): DrawShape[] {
	return lines
		.filter((l) => l.bestMove.length >= 4)
		.map((l): DrawShape => {
			const uci = l.bestMove;
			const orig = uci.slice(0, 2) as Key;
			const dest = uci.slice(2, 4) as Key;
			const rank = l.rank - 1;
			const lineWidth = lineWidths[rank] ?? lineWidths[lineWidths.length - 1];

			if (isKnightMove(uci)) {
				const color = arrowColors[rank] ?? arrowColors[arrowColors.length - 1];
				return {
					orig,
					customSvg: {
						html: knightArrowSvg(uci, color, orientation, lineWidth),
						center: 'orig'
					}
				};
			}

			return {
				orig,
				dest,
				brush: arrowBrushes[rank] ?? 'paleRed',
				modifiers: { lineWidth }
			};
		});
}
