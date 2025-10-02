import type { PageLoad } from './$types';

export const load: PageLoad = async ({ fetch }) => {
  try {
    const response = await fetch('/api/');

    if (!response.ok) {
      console.error('Failed to fetch backend greeting', response.status, response.statusText);
      return { greeting: null };
    }

    const greeting = await response.text();
    return { greeting };
  } catch (error) {
    console.error('Unexpected error while fetching backend greeting', error);
    return { greeting: null };
  }
};
