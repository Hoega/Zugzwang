export interface EvalResult {
	score: number;
	mate: number | null;
	depth: number;
	pv: string;
}

export interface MultiPvLine {
	rank: number;
	score: number;
	mate: number | null;
	depth: number;
	pv: string;
	bestMove: string;
}

export interface MultiPvResult {
	lines: MultiPvLine[];
}

type EvalCallback = (result: EvalResult) => void;
type MultiPvCallback = (result: MultiPvResult) => void;

function createStockfishWorker(): Worker {
	return new Worker('/stockfish-worker.js#/stockfish.wasm');
}

export class Engine {
	private worker: Worker | null = null;
	private callback: EvalCallback | null = null;
	private multiPvCallback: MultiPvCallback | null = null;
	private multiPvLines: Map<number, MultiPvLine> = new Map();
	private multiPvCount = 0;
	private ready = false;
	private searching = false;
	private pendingEval: { fen: string; onEval: EvalCallback; depth: number } | null = null;
	private pendingMultiPv: { fen: string; numLines: number; onEval: MultiPvCallback; depth: number } | null = null;
	private waitingForReady = false;
	private restarting = false;

	async init(): Promise<void> {
		await this._initWorker();
	}

	private _initWorker(): Promise<void> {
		return new Promise((resolve, reject) => {
			try {
				this.worker = createStockfishWorker();
				this.searching = false;
				this.waitingForReady = false;

				this.worker.onerror = () => {
					// WASM crash — restart the worker
					if (!this.restarting) {
						this.restarting = true;
						this.ready = false;
						this.searching = false;
						this.waitingForReady = false;
						this.callback = null;
						this.worker?.terminate();
						this.worker = null;
						const pending = this.pendingEval;
						this._initWorker().then(() => {
							this.restarting = false;
							if (pending) {
								this.evaluate(pending.fen, pending.onEval, pending.depth);
							}
						}).catch(() => {
							this.restarting = false;
						});
					}
				};

				this.worker.onmessage = (e: MessageEvent) => {
					const data = e.data;
					if (typeof data !== 'string') return;

					if (data === 'uciok') {
						this.send('isready');
					} else if (data === 'readyok') {
						if (!this.ready) {
							this.ready = true;
							resolve();
						}
						this.waitingForReady = false;
						if (this.pendingMultiPv) {
							const { fen, numLines, onEval, depth } = this.pendingMultiPv;
							this.pendingMultiPv = null;
							this.pendingEval = null;
							setTimeout(() => this._startMultiPvSearch(fen, numLines, onEval, depth), 0);
						} else if (this.pendingEval) {
							const { fen, onEval, depth } = this.pendingEval;
							this.pendingEval = null;
							// Defer to let any remaining WASM setTimeout callbacks drain
							setTimeout(() => this._startSearch(fen, onEval, depth), 0);
						}
					} else if (data.startsWith('bestmove')) {
						this.searching = false;
					} else if (data.startsWith('info depth')) {
						if (this.multiPvCallback) {
							const line = this.parseMultiPvInfo(data);
							if (line) {
								this.multiPvLines.set(line.rank, line);
								const lines = Array.from(this.multiPvLines.values())
									.sort((a, b) => a.rank - b.rank);
								this.multiPvCallback({ lines });
							}
						} else {
							const result = this.parseInfo(data);
							if (result) {
								this.callback?.(result);
							}
						}
					}
				};
				this.send('uci');
			} catch (e) {
				reject(e);
			}
		});
	}

	private send(cmd: string) {
		this.worker?.postMessage(cmd);
	}

	private parseInfo(line: string): EvalResult | null {
		const depthMatch = line.match(/\bdepth (\d+)/);
		const depth = depthMatch ? parseInt(depthMatch[1]) : 0;

		if (line.includes(' multipv ') && !line.includes(' multipv 1')) return null;
		if (depth < 1) return null;

		let score = 0;
		let mate: number | null = null;

		const cpMatch = line.match(/\bscore cp (-?\d+)/);
		const mateMatch = line.match(/\bscore mate (-?\d+)/);

		if (mateMatch) {
			mate = parseInt(mateMatch[1]);
			score = mate > 0 ? 10000 : -10000;
		} else if (cpMatch) {
			score = parseInt(cpMatch[1]);
		} else {
			return null;
		}

		const pvMatch = line.match(/ pv (.+)$/);
		const pv = pvMatch ? pvMatch[1] : '';

		return { score, mate, depth, pv };
	}

	evaluate(fen: string, onEval: EvalCallback, depth: number = 18) {
		if (!this.worker || !this.ready) return;

		this.pendingEval = { fen, onEval, depth };

		if (!this.waitingForReady) {
			this.callback = null;
			this.waitingForReady = true;
			if (this.searching) {
				this.send('stop');
			}
			this.send('isready');
		}
	}

	private _startSearch(fen: string, onEval: EvalCallback, depth: number) {
		// If a newer eval came in while we were deferred, skip this one
		if (this.pendingEval) {
			const { fen: newFen, onEval: newCb, depth: newDepth } = this.pendingEval;
			this.pendingEval = null;
			this._startSearch(newFen, newCb, newDepth);
			return;
		}

		const isBlackToMove = fen.split(' ')[1] === 'b';
		this.multiPvCallback = null;
		this.callback = (result) => {
			onEval({
				...result,
				score: isBlackToMove ? -result.score : result.score,
				mate: result.mate !== null ? (isBlackToMove ? -result.mate : result.mate) : null
			});
		};
		this.searching = true;
		this.send('setoption name MultiPV value 1');
		this.send(`position fen ${fen}`);
		this.send(`go depth ${depth}`);
	}

	private parseMultiPvInfo(line: string): MultiPvLine | null {
		const depthMatch = line.match(/\bdepth (\d+)/);
		const depth = depthMatch ? parseInt(depthMatch[1]) : 0;
		if (depth < 1) return null;

		const pvMatch = line.match(/ pv (.+)$/);
		const pv = pvMatch ? pvMatch[1] : '';
		if (!pv) return null;

		const multipvMatch = line.match(/\bmultipv (\d+)/);
		const rank = multipvMatch ? parseInt(multipvMatch[1]) : 1;

		let score = 0;
		let mate: number | null = null;
		const cpMatch = line.match(/\bscore cp (-?\d+)/);
		const mateMatch = line.match(/\bscore mate (-?\d+)/);

		if (mateMatch) {
			mate = parseInt(mateMatch[1]);
			score = mate > 0 ? 10000 : -10000;
		} else if (cpMatch) {
			score = parseInt(cpMatch[1]);
		} else {
			return null;
		}

		const bestMove = pv.split(' ')[0];
		return { rank, score, mate, depth, pv, bestMove };
	}

	evaluateMultiPv(fen: string, numLines: number, onEval: MultiPvCallback, depth: number = 18) {
		if (!this.worker || !this.ready) return;

		this.pendingMultiPv = { fen, numLines, onEval, depth };
		this.pendingEval = null;

		if (!this.waitingForReady) {
			this.callback = null;
			this.multiPvCallback = null;
			this.waitingForReady = true;
			if (this.searching) {
				this.send('stop');
			}
			this.send('isready');
		}
	}

	private _startMultiPvSearch(fen: string, numLines: number, onEval: MultiPvCallback, depth: number) {
		if (this.pendingMultiPv) {
			const { fen: f, numLines: n, onEval: cb, depth: d } = this.pendingMultiPv;
			this.pendingMultiPv = null;
			this._startMultiPvSearch(f, n, cb, d);
			return;
		}
		if (this.pendingEval) {
			const { fen: f, onEval: cb, depth: d } = this.pendingEval;
			this.pendingEval = null;
			this._startSearch(f, cb, d);
			return;
		}

		const isBlackToMove = fen.split(' ')[1] === 'b';
		this.multiPvLines = new Map();
		this.multiPvCount = numLines;
		this.callback = null;
		this.multiPvCallback = (result) => {
			onEval({
				lines: result.lines.map(l => ({
					...l,
					score: isBlackToMove ? -l.score : l.score,
					mate: l.mate !== null ? (isBlackToMove ? -l.mate : l.mate) : null
				}))
			});
		};
		this.searching = true;
		this.send(`setoption name MultiPV value ${numLines}`);
		this.send(`position fen ${fen}`);
		this.send(`go depth ${depth}`);
	}

	stop() {
		if (this.searching) {
			this.send('stop');
		}
	}

	destroy() {
		this.stop();
		this.worker?.terminate();
		this.worker = null;
		this.ready = false;
	}
}
