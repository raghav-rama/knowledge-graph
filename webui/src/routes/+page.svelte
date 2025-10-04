<script lang="ts">
	import AppShell from '$lib/components/AppShell.svelte';
	import DocumentTable, { type DocumentRow } from '$lib/components/DocumentTable.svelte';
	import FilterChips, { type FilterChip } from '$lib/components/FilterChips.svelte';
	import StatusFooter from '$lib/components/StatusFooter.svelte';
	import Toolbar, { type ToolbarAction } from '$lib/components/Toolbar.svelte';
	import TopNav, { type NavTab } from '$lib/components/TopNav.svelte';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	type FilterId = 'all' | DocumentRow['status'];

	let activeFilter = $state<FilterId>('all');
	let selectedId = $state<string | null>(null);
	let sortState = $state<{ column: keyof DocumentRow; direction: 'asc' | 'desc' } | null>({
		column: 'updated',
		direction: 'desc'
	});

	const navTabs: NavTab[] = [
		{ label: 'Documents', active: true },
		{ label: 'Knowledge Graph' },
		{ label: 'API' }
	];

	const toolbarActions: ToolbarAction[] = [
		{ id: 'scan', label: 'Scan', variant: 'primary', icon: '⎙' },
		{ id: 'pipeline', label: 'Pipeline Status', variant: 'secondary', icon: '⚙' }
	];

	const toolbarRightActions: ToolbarAction[] = [
		{ id: 'clear', label: 'Clear', variant: 'ghost' },
		{ id: 'upload', label: 'Upload', variant: 'primary', icon: '↑' }
	];

	const baseFilters: FilterChip[] = [
		{ id: 'all', label: 'All', count: 0 },
		{ id: 'Completed', label: 'Completed', count: 0 },
		{ id: 'Processing', label: 'Processing', count: 0 },
		{ id: 'Pending', label: 'Pending', count: 0 },
		{ id: 'Failed', label: 'Failed', count: 0 }
	];

	const documents = $derived((data.documents ?? []) as DocumentRow[]);
	const backendGreeting = $derived((data.greeting ?? null) as string | null);
	const statusMessage = $derived(backendGreeting ?? 'Unable to reach backend');

	const filterChips = $derived(
		baseFilters.map((entry) => ({
			...entry,
			count:
				entry.id === 'all'
					? documents.length
					: documents.filter((doc) => doc.status === entry.id).length
		}))
	);

	const filteredDocuments = $derived(
		activeFilter === 'all' ? documents : documents.filter((doc) => doc.status === activeFilter)
	);

	const sortedDocuments = $derived.by(() => {
		if (!sortState) {
			return filteredDocuments;
		}

		const { column, direction } = sortState;

		if (filteredDocuments.length === 0) {
			return filteredDocuments;
		}

		return [...filteredDocuments].sort((a, b) => {
			const aValue = a[column];
			const bValue = b[column];

			if (typeof aValue === 'number' && typeof bValue === 'number') {
				return direction === 'asc' ? aValue - bValue : bValue - aValue;
			}

			const aDate = new Date(String(aValue)).getTime();
			const bDate = new Date(String(bValue)).getTime();

			if (Number.isNaN(aDate) || Number.isNaN(bDate)) {
				return 0;
			}

			return direction === 'asc' ? aDate - bDate : bDate - aDate;
		});
	});

	const handleToolbarSelect = (action: ToolbarAction) => {
		console.info(`Toolbar action triggered: ${action.id}`);
	};

	const handleFilterChange = (id: string) => {
		activeFilter = id as FilterId;
	};

	const handleRefreshFilters = () => {
		console.info('Refresh filters requested');
	};

	const handleDocumentSelect = (id: string) => {
		selectedId = id;
	};

	const handleSort = (column: keyof DocumentRow) => {
		if (sortState && sortState.column === column) {
			sortState = {
				column,
				direction: sortState.direction === 'asc' ? 'desc' : 'asc'
			};
		} else {
			sortState = { column, direction: 'desc' };
		}
	};
</script>

{#snippet topnav()}
	<TopNav
		appName="Enhanced KG"
		workspace="Aubrai"
		tabs={navTabs}
		version="v0.0.1"
		onTabClick={(tab) => console.info('Tab clicked', tab.label)}
	/>
{/snippet}

{#snippet footer()}
	<StatusFooter label={statusMessage} tone={backendGreeting ? 'positive' : 'warning'} />
{/snippet}

<AppShell
	pageTitle="Document Management"
	subTitle="Uploaded knowledge base documents"
	{topnav}
	{footer}
>
	<Toolbar
		title="Document Management"
		actions={toolbarActions}
		rightActions={toolbarRightActions}
		onSelect={handleToolbarSelect}
	/>

	<section class="panel">
		<header class="panel__header">
			<div>
				<h2>Uploaded Documents</h2>
				<p>{statusMessage}</p>
			</div>
			<FilterChips
				filters={filterChips}
				activeId={activeFilter}
				onChange={handleFilterChange}
				onRefresh={handleRefreshFilters}
			/>
		</header>

		<DocumentTable
			documents={sortedDocuments}
			{selectedId}
			onSelect={handleDocumentSelect}
			onSort={handleSort}
		/>
	</section>
</AppShell>

<style>
	.panel {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.panel__header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 1.5rem;
		padding: 1rem 1.5rem;
		border-radius: 16px;
		background: rgba(255, 255, 255, 0.85);
		border: 1px solid rgba(28, 49, 68, 0.05);
	}

	.panel__header h2 {
		margin: 0 0 0.35rem 0;
		color: var(--color-prussian-blue);
	}

	.panel__header p {
		margin: 0;
		color: rgba(28, 49, 68, 0.6);
	}

	@media (max-width: 900px) {
		.panel__header {
			flex-direction: column;
			align-items: flex-start;
		}
	}
</style>
