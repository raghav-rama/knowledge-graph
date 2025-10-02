import type { PageLoad } from './$types';

type DocumentStatus = 'Completed' | 'Processing' | 'Pending' | 'Failed';

const mockDocuments = [
  {
    id: 'doc-a34a3a69ed32b3ea5bfc6d85d73a2...',
    summary: 'Low Testosterone Associated With Obesity and the Metabolic Syndrome',
    status: 'Completed' as DocumentStatus,
    length: 40943,
    chunks: 10,
    created: '2/10/2025, 9:01:59 PM',
    updated: '2/10/2025, 9:16:46 PM'
  },
  {
    id: 'doc-675e2ad21423fe364b999f92617f8db...',
    summary: 'Journal Pre-proof The role of seminal oxidation reduction potential in ...',
    status: 'Completed' as DocumentStatus,
    length: 48142,
    chunks: 13,
    created: '2/10/2025, 9:01:31 PM',
    updated: '2/10/2025, 9:14:05 PM'
  },
  {
    id: 'doc-f55d2bdcc7063faabb78df2389925...',
    summary: 'V Volume 151 ( April 1994 Number 4 The Journal of URO...',
    status: 'Completed' as DocumentStatus,
    length: 42004,
    chunks: 11,
    created: '2/10/2025, 9:01:49 PM',
    updated: '2/10/2025, 9:13:38 PM'
  },
  {
    id: 'doc-2c5c9752bc7f67194999684fe9463bf...',
    summary: 'Ingredient: L-Arginine • L-Arginine is a physiological precursor...',
    status: 'Completed' as DocumentStatus,
    length: 61036,
    chunks: 18,
    created: '2/10/2025, 9:01:27 PM',
    updated: '2/10/2025, 9:07:00 PM'
  }
];

export const load: PageLoad = async ({ fetch }) => {
  let greeting: string | null = null;

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

  return {
    greeting,
    documents: mockDocuments
  };
};
