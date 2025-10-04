<script lang="ts">
  import type { Snippet } from 'svelte';

  const props = $props<{
    pageTitle?: string;
    subTitle?: string;
    children: Snippet;
    topnav?: Snippet;
    footer?: Snippet;
  }>();

  const pageTitle = $derived(props.pageTitle ?? '');
  const subTitle = $derived(props.subTitle ?? '');
  const children = props.children;
  const topnav = props.topnav;
  const footer = props.footer;
</script>

<div class="app-shell">
  <header class="app-shell__header">
    {#if topnav}
      {@render topnav()}
    {/if}
  </header>
  <main class="app-shell__main" aria-label="Main content">
    <section class="app-shell__heading">
      <h1>{pageTitle}</h1>
      {#if subTitle}
        <p>{subTitle}</p>
      {/if}
    </section>
    {@render children()}
  </main>
  <footer class="app-shell__footer">
    {#if footer}
      {@render footer()}
    {/if}
  </footer>
</div>

<style>
  .app-shell {
    min-height: 100vh;
    display: flex;
    flex-direction: column;
    background: var(--color-light-orange);
    color: var(--color-prussian-blue);
  }

  .app-shell__header {
    border-bottom: 1px solid rgba(28, 49, 68, 0.08);
    background: rgba(255, 255, 255, 0.9);
    backdrop-filter: blur(8px);
    position: sticky;
    top: 0;
    z-index: 10;
  }

  .app-shell__main {
    flex: 1;
    padding: 2.5rem 2rem 1.5rem;
    display: flex;
    flex-direction: column;
    gap: 1.5rem;
  }

  .app-shell__heading {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }

  .app-shell__heading h1 {
    font-size: 1.75rem;
    margin: 0;
    color: var(--color-prussian-blue);
  }

  .app-shell__heading p {
    margin: 0;
    color: rgba(28, 49, 68, 0.72);
  }

  .app-shell__footer {
    padding: 0.75rem 2rem;
    background: rgba(28, 49, 68, 0.06);
    border-top: 1px solid rgba(28, 49, 68, 0.08);
  }

  @media (max-width: 768px) {
    .app-shell__main {
      padding: 1.5rem 1rem;
    }
  }
</style>
