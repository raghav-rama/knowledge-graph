<script lang="ts">
	type Tone = 'positive' | 'warning' | 'negative';

	const toneClass = {
		positive: 'status-footer__indicator--positive',
		warning: 'status-footer__indicator--warning',
		negative: 'status-footer__indicator--negative'
	} satisfies Record<Tone, string>;

	const props = $props<{
		label?: string;
		tone?: Tone;
	}>();

	const label = $derived(props.label ?? 'Connected');
	const tone = $derived<Tone>((props.tone ?? 'positive') as Tone);
</script>

<div class="status-footer">
	<span class={`status-footer__indicator ${toneClass[tone]}`} aria-hidden="true"></span>
	<span>{label}</span>
</div>

<style>
	.status-footer {
		display: inline-flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.85rem;
		color: rgba(28, 49, 68, 0.75);
	}

	.status-footer__indicator {
		width: 10px;
		height: 10px;
		border-radius: 50%;
		background: rgba(86, 163, 166, 0.6);
		position: relative;
	}

	.status-footer__indicator::after {
		content: '';
		position: absolute;
		inset: -4px;
		border-radius: 50%;
		border: 2px solid currentColor;
		opacity: 0.3;
	}

	.status-footer__indicator--positive {
		color: var(--color-verdigris);
		background: var(--color-verdigris);
	}

	.status-footer__indicator--warning {
		color: var(--color-light-orange);
		background: var(--color-light-orange);
	}

	.status-footer__indicator--negative {
		color: var(--color-black-bean);
		background: var(--color-black-bean);
	}
</style>
