<script lang="ts">
	let { score = null, mate = null, orientation = 'white' }: {
		score: number | null;
		mate: number | null;
		orientation: 'white' | 'black';
	} = $props();

	let whitePercent = $derived.by(() => {
		if (score === null && mate === null) return 50;
		if (mate !== null) {
			return mate > 0 ? 100 : 0;
		}
		// Lichess win% formula: 50 + 50 * (2 / (1 + exp(-0.00368208 * cp)) - 1)
		const winPct = 50 + 50 * (2 / (1 + Math.exp(-0.00368208 * score!)) - 1);
		return Math.max(1, Math.min(99, winPct));
	});

	let displayPercent = $derived(whitePercent);

	let evalText = $derived.by(() => {
		if (score === null && mate === null) return '...';
		if (mate !== null) {
			return `M${Math.abs(mate)}`;
		}
		const val = score! / 100;
		if (val > 0) return `+${val.toFixed(1)}`;
		if (val < 0) return val.toFixed(1);
		return '0.0';
	});

	let isWhiteAdvantage = $derived(
		mate !== null ? mate > 0 : (score ?? 0) >= 0
	);
</script>

<div class="eval-bar">
	<div class="eval-label" class:white-text={!isWhiteAdvantage}>
		{evalText}
	</div>
	<div class="bar" style="flex-direction: {orientation === 'white' ? 'column-reverse' : 'column'}">
		<div
			class="white-fill"
			style="height: {displayPercent}%"
		></div>
	</div>
</div>

<style>
	.eval-bar {
		display: flex;
		flex-direction: column;
		align-items: center;
		width: 28px;
		height: 100%;
		flex-shrink: 0;
	}

	.eval-label {
		font-size: 0.65rem;
		font-weight: 700;
		text-align: center;
		padding: 2px 0;
		color: #1a1a1a;
		line-height: 1;
		min-height: 14px;
		width: 100%;
	}

	.eval-label.white-text {
		color: #1a1a1a;
	}

	.bar {
		flex: 1;
		width: 100%;
		background: #333;
		border-radius: 3px;
		overflow: hidden;
		position: relative;
		display: flex;
		/* flex-direction set via inline style based on orientation */
	}

	.white-fill {
		width: 100%;
		background: #e8e8e8;
		transition: height 0.4s ease;
		border-radius: 0 0 3px 3px;
	}
</style>
