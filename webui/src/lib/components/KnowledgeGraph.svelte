<script lang="ts">
	import { onMount, tick } from 'svelte';
	import cytoscape from 'cytoscape';
	import { z } from 'zod';
	import type { EntityResponse } from '$lib/codegen/types/EntityResponse';
	import type { GraphResponse } from '$lib/codegen/types/GraphResponse';
	import type { RelationshipEdgeResponse } from '$lib/codegen/types/RelationshipEdgeResponse';

	const entitySchema = z.object({
		id: z.string(),
		entity_name: z.string(),
		entity_description: z.string(),
		entity_type: z.string()
	}) satisfies z.ZodType<EntityResponse>;

	const relationshipSchema = z.object({
		id: z.string(),
		source_node_id: z.string(),
		target_node_id: z.string(),
		relation_description: z.string()
	}) satisfies z.ZodType<RelationshipEdgeResponse>;

	const graphResponseSchema = z.object({
		entities: z.array(entitySchema),
		relations: z.array(relationshipSchema)
	}) satisfies z.ZodType<GraphResponse>;

	let graphContainer: HTMLDivElement | null = null;
	let cy: cytoscape.Core | null = null;
	let loadError: string | null = null;

	function mapEntitiesRelations(obj: GraphResponse) {
		const entities = obj.entities.map((e) => {
			return {
				data: {
					id: e.id,
					description: e.entity_description,
					type: e.entity_type,
					name: e.entity_name
				}
			};
		});
		const relations = obj.relations.map((e) => {
			return {
				data: {
					source: e.source_node_id,
					target: e.target_node_id,
					description: e.relation_description
				}
			};
		});
		return [...entities, ...relations];
	}

	const cytoscapeConfig: Omit<cytoscape.CytoscapeOptions, 'container' | 'elements'> = {
		style: [
			{
				selector: 'node',
				style: {
					'background-color': '#666',
					label: 'data(name)',
					'text-valign': 'center',
					'text-halign': 'center',
					'font-size': '12px',
					width: '60px',
					height: '60px'
				}
			},
			{
				selector: 'edge',
				style: {
					width: 2,
					'line-color': '#ccc',
					'target-arrow-color': '#ccc',
					'target-arrow-shape': 'triangle',
					'curve-style': 'bezier',
					label: 'data(description)',
					'font-size': '10px',
					'text-rotation': 'autorotate',
					'text-margin-y': -10
				}
			}
		],
		minZoom: 0.1,
		maxZoom: 3,

		layout: {
			name: 'cose',
			animate: false,
			nodeRepulsion: 8000,
			idealEdgeLength: 100,
			padding: 30
		},
		textureOnViewport: true,
		motionBlur: false,
		hideEdgesOnViewport: true,
		hideLabelsOnViewport: true
	};

	async function fetchGraph(): Promise<GraphResponse> {
		const response = await fetch('/api/graph');
		if (!response.ok) {
			throw new Error(`Failed to load graph (${response.status})`);
		}

		const json = await response.json();
		return graphResponseSchema.parse(json);
	}

	onMount(() => {
		let destroyed = false;

		const init = async () => {
			await tick();

			if (!graphContainer || destroyed) {
				return;
			}

			try {
				const graph = await fetchGraph();

				if (destroyed) {
					return;
				}

				cy = cytoscape({
					...cytoscapeConfig,
					container: graphContainer,
					elements: mapEntitiesRelations(graph)
				});

				cy.resize();
				cy.fit();
			} catch (err) {
				console.error(err);
				loadError = err instanceof Error ? err.message : 'Failed to load graph';
			}
		};

		void init();

		return () => {
			destroyed = true;
			cy?.destroy();
			cy = null;
		};
	});
</script>

<section class="graph-panel">
	<header class="graph-panel__header">
		<div>
			<h2>Knowledge Graph</h2>
			<p>Visual relationships between indexed documents.</p>
		</div>
	</header>
	{#if loadError}
		<p class="graph-panel__error">{loadError}</p>
	{/if}
	<div class="graph-panel__canvas" bind:this={graphContainer}></div>
</section>

<style>
	.graph-panel {
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
		margin: 1.5rem 0;
	}

	.graph-panel__header {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		gap: 1.5rem;
		padding: 1rem 1.5rem;
		border-radius: 16px;
		background: rgba(255, 255, 255, 0.85);
		border: 1px solid rgba(28, 49, 68, 0.05);
	}

	.graph-panel__header h2 {
		margin: 0 0 0.35rem 0;
		color: var(--color-prussian-blue);
	}

	.graph-panel__header p {
		margin: 0;
		color: rgba(28, 49, 68, 0.6);
	}

	.graph-panel__canvas {
		width: 100%;
		min-height: 480px;
		border-radius: 16px;
		border: 1px solid rgba(28, 49, 68, 0.05);
		background: rgba(255, 255, 255, 0.85);
	}

	@media (max-width: 900px) {
		.graph-panel__header {
			flex-direction: column;
			align-items: flex-start;
		}
	}

	.graph-panel__error {
		margin: 0;
		padding: 0 1.5rem;
		color: var(--color-error, #b42318);
	}
</style>
