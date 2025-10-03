<script lang="ts">
	export type NavTab = {
		label: string;
		active?: boolean;
	};

	const noop: (tab: NavTab) => void = () => {};

	let {
		appName = 'Enhanced KG',
		workspace = 'Aubrai',
		tabs = [],
		version = 'v0.0.1',
		onTabClick = noop
	} = $props<{
		appName?: string;
		workspace?: string;
		tabs?: NavTab[];
		version?: string;
		onTabClick?: (tab: NavTab) => void;
	}>();
</script>

<nav class="topnav">
	<div class="topnav__brand">
		<span class="topnav__logo" aria-hidden="true">AUB</span>
		<div class="topnav__titles">
			<strong>{appName}</strong>
			<button type="button" class="topnav__workspace">
				{workspace}
				<span aria-hidden="true">⌄</span>
			</button>
		</div>
	</div>

	<div class="topnav__tabs" role="tablist">
		{#each tabs as tab}
			<button
				type="button"
				role="tab"
				class:topnav__tab--active={tab.active}
				class="topnav__tab"
				onclick={() => onTabClick(tab)}
				aria-selected={tab.active}
			>
				{tab.label}
			</button>
		{/each}
	</div>

	<div class="topnav__meta">
		<span class="topnav__badge">{version}</span>
		<button type="button" class="topnav__icon" aria-label="Sync status"> ⟳ </button>
	</div>
</nav>

<style>
	.topnav {
		display: grid;
		grid-template-columns: 1fr auto 1fr;
		align-items: center;
		gap: 1.5rem;
		padding: 1rem 2rem;
	}

	.topnav__brand {
		display: flex;
		align-items: center;
		gap: 0.75rem;
	}

	.topnav__logo {
		display: inline-flex;
		align-items: center;
		justify-content: center;
		width: 40px;
		height: 40px;
		border-radius: 12px;
		background: var(--color-prussian-blue);
		color: var(--color-light-orange);
		font-weight: 700;
	}

	.topnav__titles {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
	}

	.topnav__workspace {
		display: inline-flex;
		align-items: center;
		gap: 0.3rem;
		border: none;
		background: transparent;
		color: rgba(28, 49, 68, 0.8);
		font-size: 0.875rem;
		cursor: pointer;
	}

	.topnav__tabs {
		display: inline-flex;
		gap: 0.4rem;
		justify-self: center;
		background: rgba(255, 255, 255, 0.8);
		padding: 0.4rem;
		border-radius: 999px;
		border: 1px solid rgba(28, 49, 68, 0.08);
	}

	.topnav__tab {
		border: none;
		background: transparent;
		padding: 0.45rem 0.95rem;
		border-radius: 999px;
		font-weight: 600;
		color: rgba(28, 49, 68, 0.7);
		cursor: pointer;
		transition: background-color 0.2s ease;
	}

	.topnav__tab--active {
		background: var(--color-pink-lavender);
		color: var(--color-black-bean);
		box-shadow: 0 4px 12px -8px rgba(228, 183, 229, 0.8);
	}

	.topnav__meta {
		justify-self: end;
		display: inline-flex;
		align-items: center;
		gap: 0.75rem;
	}

	.topnav__badge {
		padding: 0.3rem 0.7rem;
		border-radius: 999px;
		background: var(--color-verdigris);
		color: var(--color-black-bean);
		font-size: 0.75rem;
		font-weight: 600;
	}

	.topnav__icon {
		width: 36px;
		height: 36px;
		border-radius: 50%;
		border: 1px solid rgba(28, 49, 68, 0.1);
		background: rgba(255, 255, 255, 0.9);
		cursor: pointer;
	}

	@media (max-width: 960px) {
		.topnav {
			grid-template-columns: 1fr;
			justify-items: stretch;
			gap: 1rem;
		}

		.topnav__tabs {
			justify-content: flex-start;
			overflow-x: auto;
		}

		.topnav__meta {
			justify-self: start;
		}
	}
</style>
