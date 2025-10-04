<script lang="ts">
  export type FilterChip = {
    id: string;
    label: string;
    count: number;
  };

  const props = $props<{
    filters?: FilterChip[];
    activeId: string;
    onChange?: (id: string) => void;
    onRefresh?: () => void;
  }>();

  const filters = $derived(props.filters ?? []);
  const activeId = $derived(props.activeId);
  const handleChange = props.onChange ?? (() => {});
  const handleRefresh = props.onRefresh ?? (() => {});
</script>

<div class="filter-chips" role="tablist">
  {#each filters as filter}
    <button
      type="button"
      role="tab"
      class="filter-chip {filter.id === activeId ? 'filter-chip--active' : ''}"
      aria-selected={filter.id === activeId}
      onclick={() => handleChange(filter.id)}
    >
      <span>{filter.label}</span>
      <span class="filter-chip__count">{filter.count}</span>
    </button>
  {/each}
  <button
    type="button"
    class="filter-chips__refresh"
    aria-label="Refresh"
    onclick={handleRefresh}
  >
    ⟳
  </button>
</div>

<style>
  .filter-chips {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .filter-chip {
    display: inline-flex;
    align-items: center;
    gap: 0.4rem;
    padding: 0.4rem 0.8rem;
    border-radius: 999px;
    border: 1px solid rgba(28, 49, 68, 0.12);
    background: rgba(255, 255, 255, 0.85);
    color: rgba(28, 49, 68, 0.75);
    font-weight: 600;
    cursor: pointer;
    transition: background 0.2s ease, color 0.2s ease;
  }

  .filter-chip--active {
    background: var(--color-verdigris);
    border-color: var(--color-verdigris);
    color: var(--color-black-bean);
  }

  .filter-chip__count {
    display: inline-flex;
    align-items: center;
    justify-content: center;
    min-width: 24px;
    height: 24px;
    border-radius: 999px;
    background: rgba(28, 49, 68, 0.08);
    color: inherit;
    font-size: 0.85rem;
  }

  .filter-chip--active .filter-chip__count {
    background: rgba(0, 0, 0, 0.08);
  }

  .filter-chips__refresh {
    width: 36px;
    height: 36px;
    border-radius: 50%;
    border: 1px solid rgba(28, 49, 68, 0.1);
    background: rgba(255, 255, 255, 0.9);
    cursor: pointer;
  }
</style>
