<script lang="ts">
	export type DocumentRow = {
		id: string;
		summary: string;
		status: 'Completed' | 'Processing' | 'Pending' | 'Failed';
		length: number;
		chunks: number;
		created: string;
		updated: string;
	};

	const props = $props<{
		documents?: DocumentRow[];
		selectedId?: string | null;
		onSelect?: (id: string) => void;
		onSort?: (column: keyof DocumentRow) => void;
	}>();

	const documents = $derived(props.documents ?? []);
	const selectedId = $derived(props.selectedId ?? null);
	const onSelect = props.onSelect ?? (() => {});
	const onSort = props.onSort ?? (() => {});

	const handleSelect = (id: string) => {
		onSelect(id);
	};

	const statusClass = (status: DocumentRow['status']) => {
		switch (status) {
			case 'Completed':
				return 'status-badge--success';
			case 'Processing':
				return 'status-badge--processing';
			case 'Pending':
				return 'status-badge--pending';
			case 'Failed':
				return 'status-badge--error';
		}
	};
</script>

<div class="table-wrapper">
	<table class="document-table">
		<thead>
			<tr>
				<th scope="col">ID</th>
				<th scope="col">Summary</th>
				<th scope="col">Status</th>
				<th scope="col">Length</th>
				<th scope="col">Chunks</th>
				<th scope="col" class="is-sortable" onclick={() => onSort('created')}> Created </th>
				<th scope="col" class="is-sortable" onclick={() => onSort('updated')}> Updated </th>
				<th scope="col" class="is-center">Select</th>
			</tr>
		</thead>
		<tbody>
			{#if documents.length === 0}
				<tr>
					<td colspan="8" class="is-empty">No documents available.</td>
				</tr>
			{:else}
				{#each documents as doc}
					<tr class:selected={doc.id === selectedId}>
						<td title={doc.id}>{`${doc.id.substring(0, 10)}...`}</td>
						<td class="summary" title={doc.summary}>{doc.summary}</td>
						<td>
							<span class={'status-badge ' + statusClass(doc.status)}>{doc.status}</span>
						</td>
						<td>{doc.length.toLocaleString()}</td>
						<td>{doc.chunks}</td>
						<td>{doc.created}</td>
						<td>{doc.updated}</td>
						<td class="is-center">
							<input
								type="radio"
								name="document"
								aria-label={`Select ${doc.summary}`}
								checked={doc.id === selectedId}
								onchange={() => handleSelect(doc.id)}
							/>
						</td>
					</tr>
				{/each}
			{/if}
		</tbody>
	</table>
</div>

<style>
	.table-wrapper {
		border-radius: 18px;
		border: 1px solid rgba(28, 49, 68, 0.08);
		overflow: hidden;
		background: rgba(255, 255, 255, 0.9);
		box-shadow: 0 18px 35px -22px rgba(28, 49, 68, 0.4);
	}

	.document-table {
		width: 100%;
		border-collapse: collapse;
	}

	th,
	td {
		padding: 0.85rem 1rem;
		text-align: left;
		font-size: 0.95rem;
		color: rgba(28, 49, 68, 0.8);
	}

	thead th {
		font-size: 0.8rem;
		text-transform: uppercase;
		letter-spacing: 0.05em;
		color: rgba(28, 49, 68, 0.6);
		background: rgba(28, 49, 68, 0.05);
	}

	tbody tr {
		transition: background 0.15s ease;
	}

	tbody tr:hover {
		background: rgba(86, 163, 166, 0.08);
	}

	tr.selected {
		background: rgba(86, 163, 166, 0.12);
	}

	.summary {
		max-width: 420px;
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}

	.status-badge {
		display: inline-flex;
		align-items: center;
		padding: 0.2rem 0.6rem;
		border-radius: 999px;
		font-size: 0.75rem;
		font-weight: 600;
	}

	.status-badge--success {
		background: rgba(86, 163, 166, 0.18);
		color: var(--color-prussian-blue);
	}

	.status-badge--processing {
		background: rgba(228, 183, 229, 0.2);
		color: var(--color-black-bean);
	}

	.status-badge--pending {
		background: rgba(254, 225, 199, 0.4);
		color: var(--color-black-bean);
	}

	.status-badge--error {
		background: rgba(55, 0, 10, 0.1);
		color: var(--color-black-bean);
	}

	.is-center {
		text-align: center;
	}

	.is-empty {
		text-align: center;
		padding: 3rem 0;
		color: rgba(28, 49, 68, 0.5);
	}

	.is-sortable {
		cursor: pointer;
		user-select: none;
	}

	.is-sortable:hover {
		color: var(--color-black-bean);
	}

	@media (max-width: 900px) {
		.summary {
			max-width: 240px;
		}
	}
</style>
