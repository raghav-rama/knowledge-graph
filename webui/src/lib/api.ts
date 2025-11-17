import { env } from '$env/dynamic/public';

const DEFAULT_BASE = '/api';

const normalizeBase = (value: string | undefined | null): string => {
	if (!value) {
		return DEFAULT_BASE;
	}
	const trimmed = value.trim();
	if (trimmed === '' || trimmed === '/') {
		return '';
	}
	return trimmed.replace(/\/+$/, '');
};

const normalizedBase = normalizeBase(env.PUBLIC_API_BASE_URL);

const normalizePath = (path: string): string => (path.startsWith('/') ? path : `/${path}`);

const buildUrl = (path: string): string => {
	const safeBase = normalizedBase;
	const safePath = normalizePath(path);
	return safeBase === '' ? safePath : `${safeBase}${safePath}`;
};

type FetchLike = typeof fetch;

export const apiBaseUrl = normalizedBase || DEFAULT_BASE;

export const apiFetch = (
	path: string,
	init?: RequestInit,
	fetchImpl?: FetchLike
): Promise<Response> => {
	const impl = fetchImpl ?? fetch;
	return impl(buildUrl(path), init);
};

export const apiUrl = (path: string): string => buildUrl(path);
