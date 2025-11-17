<script lang="ts">
	import AppShell from '$lib/components/AppShell.svelte';
	import DocumentTable, { type DocumentRow } from '$lib/components/DocumentTable.svelte';
	import FilterChips, { type FilterChip } from '$lib/components/FilterChips.svelte';
	import KnowledgeGraph from '$lib/components/KnowledgeGraph.svelte';
	import StatusFooter from '$lib/components/StatusFooter.svelte';
	import Toolbar, { type ToolbarAction } from '$lib/components/Toolbar.svelte';
	import TopNav, { type NavTab } from '$lib/components/TopNav.svelte';
	import { apiFetch } from '$lib/api';
	import type { PageProps } from './$types';

	let { data }: PageProps = $props();

	type TabLabel = 'Documents' | 'Knowledge Graph' | 'API';

	type FilterId = 'all' | DocumentRow['status'];
	type StatusTone = 'positive' | 'warning' | 'negative';

	type BackendDocument = {
		id: string;
		summary: string;
		status: string;
		length: number;
		chunks: number;
		created_at?: string | null;
		updated_at?: string | null;
	};

	type DocumentListResponse = {
		total: number;
		documents: BackendDocument[];
	};

	const backendGreeting: string | null = data.greeting ?? null;

	let documents = $state<DocumentRow[]>(data.documents ?? []);
	let activeFilter = $state<FilterId>('all');
	let selectedId = $state<string | null>(null);
	let sortState = $state<{ column: keyof DocumentRow; direction: 'asc' | 'desc' } | null>({
		column: 'updated',
		direction: 'desc'
	});

	let statusMessage = $state<string>(backendGreeting ?? 'Unable to reach backend');
	let statusTone = $state<StatusTone>(backendGreeting ? 'positive' : 'warning');
	let isUploading = $state(false);
	let fileInput: HTMLInputElement | null = $state(null);

	let navTabs = $state<NavTab[]>([
		{ label: 'Documents', active: true },
		{ label: 'Knowledge Graph' },
		{ label: 'API' }
	]);

	const activeTab = $derived<() => TabLabel>(() => {
		const current = navTabs.find((tab) => tab.active)?.label;
		return (current as TabLabel) ?? 'Documents';
	});

	const setActiveTab = (label: TabLabel) => {
		navTabs = navTabs.map((tab) => ({
			...tab,
			active: tab.label === label
		}));
	};

	const handleTabClick = (tab: NavTab) => {
		if (tab.label === 'Documents' || tab.label === 'Knowledge Graph' || tab.label === 'API') {
			setActiveTab(tab.label);
		}
	};

	const toolbarActions: ToolbarAction[] = [
		{ id: 'scan', label: 'Scan', variant: 'primary', icon: '⎙' },
		{ id: 'pipeline', label: 'Pipeline Status', variant: 'secondary', icon: '⚙' }
	];

	const toolbarRightActions = $derived<ToolbarAction[]>([
		{ id: 'clear', label: 'Clear', variant: 'ghost' },
		{
			id: 'upload',
			label: isUploading ? 'Uploading…' : 'Upload',
			variant: 'primary',
			icon: isUploading ? '⏳' : '↑'
		}
	]);

	const baseFilters: FilterChip[] = [
		{ id: 'all', label: 'All', count: 0 },
		{ id: 'Completed', label: 'Completed', count: 0 },
		{ id: 'Processing', label: 'Processing', count: 0 },
		{ id: 'Pending', label: 'Pending', count: 0 },
		{ id: 'Failed', label: 'Failed', count: 0 }
	];

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

	const toDocumentStatus = (status: string): DocumentRow['status'] => {
		switch (status) {
			case 'Completed':
			case 'Processing':
			case 'Pending':
			case 'Failed':
				return status;
			default:
				return 'Pending';
		}
	};

	const formatTimestamp = (value?: string | null): string => {
		if (!value) {
			return '—';
		}

		const date = new Date(value);
		return Number.isNaN(date.getTime()) ? value : date.toLocaleString();
	};

	const mapDocument = (doc: BackendDocument): DocumentRow => ({
		id: doc.id,
		summary: doc.summary,
		status: toDocumentStatus(doc.status),
		length: doc.length ?? 0,
		chunks: doc.chunks ?? 0,
		created: formatTimestamp(doc.created_at),
		updated: formatTimestamp(doc.updated_at)
	});

	const refreshDocuments = async (showStatus = false) => {
		try {
			const response = await apiFetch('/documents');
			if (!response.ok) {
				throw new Error(`${response.status} ${response.statusText}`);
			}
			const payload = (await response.json()) as DocumentListResponse;
			if (!Array.isArray(payload.documents)) {
				throw new Error('Malformed response from server');
			}
			const mapped = payload.documents.map(mapDocument);
			documents = mapped;
			if (selectedId && !mapped.some((doc) => doc.id === selectedId)) {
				selectedId = null;
			}
			if (showStatus) {
				statusTone = 'positive';
				statusMessage = `Loaded ${mapped.length} documents`;
			}
		} catch (error) {
			console.error('Failed to refresh documents', error);
			statusTone = 'negative';
			statusMessage =
				error instanceof Error ? `Refresh failed: ${error.message}` : 'Refresh failed';
		}
	};

	const handleToolbarSelect = (action: ToolbarAction) => {
		switch (action.id) {
			case 'upload':
				if (!isUploading && fileInput) {
					fileInput.click();
				}
				break;
			case 'clear':
				selectedId = null;
				break;
			case 'scan':
			case 'pipeline':
				if (!isUploading) {
					void refreshDocuments(true);
				}
				break;
			default:
				console.info(`Unhandled toolbar action: ${action.id}`);
		}
	};

	const handleFilterChange = (id: string) => {
		activeFilter = id as FilterId;
	};

	const handleRefreshFilters = () => {
		if (!isUploading) {
			void refreshDocuments(true);
		}
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

	const handleFileChange = async (event: Event) => {
		const input = event.currentTarget as HTMLInputElement;
		const file = input.files?.[0];
		if (!file) {
			return;
		}

		isUploading = true;
		statusTone = 'warning';
		statusMessage = `Uploading ${file.name}…`;

		try {
			const formData = new FormData();
			formData.append('file', file);

			const response = await apiFetch('/documents/upload', {
				method: 'POST',
				body: formData
			});

			const payload = await response.json().catch(() => null);

			if (!response.ok) {
				const message =
					(payload && typeof payload.message === 'string'
						? payload.message
						: response.statusText) || 'Upload failed';
				throw new Error(message);
			}

			const status = payload?.status ?? 'success';
			const message =
				(payload && typeof payload.message === 'string'
					? payload.message
					: `Upload completed for ${file.name}`) || `Upload completed for ${file.name}`;

			statusTone = status === 'success' ? 'positive' : 'warning';
			statusMessage = message;

			await refreshDocuments(false);
		} catch (error) {
			console.error('Document upload failed', error);
			statusTone = 'negative';
			statusMessage =
				error instanceof Error
					? `Upload failed: ${error.message}`
					: 'Upload failed due to an unexpected error';
		} finally {
			isUploading = false;
			if (fileInput) {
				fileInput.value = '';
			}
		}
	};
</script>

{#snippet topnav()}
	<TopNav
		appName="Enhanced KG"
		workspace="Aubrai"
		tabs={navTabs}
		version="v0.0.1"
		onTabClick={handleTabClick}
	/>
{/snippet}

{#snippet footer()}
	<StatusFooter label={statusMessage} tone={statusTone} />
{/snippet}

<svelte:head>
	<title>Enhanced KG</title>
	<meta name="description" content="This is an Enhanced Knowledge Graph." />
	<meta name="author" content="Satoshi" />
	<link rel="canonical" href="https://www.example.com/sample" />
</svelte:head>

<AppShell {topnav} {footer}>
	{#if activeTab() === 'Knowledge Graph'}
		<KnowledgeGraph />
	{:else if activeTab() === 'API'}
		<section class="panel panel--placeholder">
			<header class="panel__header">
				<div>
					<h2>API Console</h2>
					<p>API tools coming soon.</p>
				</div>
			</header>
		</section>
	{:else}
		<Toolbar
			title="Scholarly Articles"
			actions={toolbarActions}
			rightActions={toolbarRightActions}
			onSelect={handleToolbarSelect}
		/>

		<input
			type="file"
			accept=".txt,.md,.json,.csv"
			class="hidden-file-input"
			bind:this={fileInput}
			onchange={handleFileChange}
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
	{/if}
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

	.hidden-file-input {
		position: absolute;
		width: 0.1px;
		height: 0.1px;
		opacity: 0;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
		border: 0;
	}

	@media (max-width: 900px) {
		.panel__header {
			flex-direction: column;
			align-items: flex-start;
		}
	}

	.panel--placeholder {
		margin: 1.5rem 0;
	}
</style>
