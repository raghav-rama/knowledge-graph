<script lang="ts">
	import { onMount, tick } from 'svelte';
	import cytoscape from 'cytoscape';

	let graphContainer: HTMLDivElement | null = null;
	let cy: cytoscape.Core | null = null;

	const cytoscapeConfig: Omit<cytoscape.CytoscapeOptions, 'container'> = {
		elements: [
			{
				data: { id: 'a' }
			},
			{
				data: { id: 'b' }
			},
			{
				data: { id: 'ab', source: 'a', target: 'b' }
			}
		],
		style: [
			{
				selector: 'node',
				style: {
					'background-color': '#666',
					label: 'data(id)'
				}
			},
			{
				selector: 'edge',
				style: {
					width: 3,
					'line-color': '#ccc',
					'target-arrow-color': '#ccc',
					'target-arrow-shape': 'triangle',
					'curve-style': 'bezier'
				}
			}
		],
		layout: {
			name: 'grid',
			rows: 1
		}
	};

	onMount(() => {
		let destroyed = false;

		const init = async () => {
			await tick();

			if (!graphContainer || destroyed) {
				return;
			}

			cy = cytoscape({
				...cytoscapeConfig,
				container: graphContainer
			});

			cy.resize();
			cy.fit();
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
</style>
