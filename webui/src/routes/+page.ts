import type { DocumentRow } from '$lib/components/DocumentTable.svelte';
import type { PageLoad } from './$types';

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

	const timestamp = new Date(value);
	return Number.isNaN(timestamp.getTime()) ? value : timestamp.toLocaleString();
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

export const load: PageLoad = async ({ fetch }) => {
	let greeting: string | null = null;
	let documents: DocumentRow[] = [];

	try {
		const response = await fetch('/api/');
		if (response.ok) {
			greeting = await response.text();
		} else {
			console.error('Failed to fetch backend greeting', response.status, response.statusText);
		}
	} catch (error) {
		console.error('Unexpected error while fetching backend greeting', error);
	}

	try {
		const response = await fetch('/api/documents');
		if (response.ok) {
			const payload = (await response.json()) as DocumentListResponse;
			if (Array.isArray(payload.documents)) {
				documents = payload.documents.map(mapDocument);
			}
		} else {
			console.error(
				'Failed to fetch documents',
				response.status,
				response.statusText
			);
		}
	} catch (error) {
		console.error('Unexpected error while fetching documents', error);
	}

	return {
		greeting,
		documents
	};
};
