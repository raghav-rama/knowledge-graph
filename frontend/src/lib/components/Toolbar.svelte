<script lang="ts">
  export type ToolbarAction = {
    id: string;
    label: string;
    variant?: 'primary' | 'secondary' | 'ghost';
    icon?: string;
  };

  const props = $props<{
    title?: string;
    actions?: ToolbarAction[];
    rightActions?: ToolbarAction[];
    onSelect?: (action: ToolbarAction) => void;
  }>();

  const title = $derived(props.title ?? '');
  const actions = $derived(props.actions ?? []);
  const rightActions = $derived(props.rightActions ?? []);
  const onSelect = props.onSelect ?? (() => {});

  const handleClick = (action: ToolbarAction) => {
    onSelect(action);
  };
</script>

<div class="toolbar">
  <div class="toolbar__left">
    {#if title}
      <h2>{title}</h2>
    {/if}
    <div class="toolbar__actions">
      {#each actions as action}
        <button
          type="button"
          class="toolbar__button toolbar__button--{action.variant ?? 'primary'}"
          onclick={() => handleClick(action)}
        >
          {#if action.icon}
            <span class="toolbar__icon" aria-hidden="true">{action.icon}</span>
          {/if}
          {action.label}
        </button>
      {/each}
    </div>
  </div>
  <div class="toolbar__right">
    <div class="toolbar__actions">
      {#each rightActions as action}
        <button
          type="button"
          class="toolbar__button toolbar__button--{action.variant ?? 'ghost'}"
          onclick={() => handleClick(action)}
        >
          {#if action.icon}
            <span class="toolbar__icon" aria-hidden="true">{action.icon}</span>
          {/if}
          {action.label}
        </button>
      {/each}
    </div>
  </div>
</div>

<style>
  .toolbar {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    padding: 1rem 1.5rem;
    border-radius: 16px;
    background: rgba(255, 255, 255, 0.9);
    border: 1px solid rgba(28, 49, 68, 0.08);
    box-shadow: 0 12px 30px -22px rgba(28, 49, 68, 0.4);
  }

  .toolbar h2 {
    margin: 0 1rem 0 0;
    font-size: 1.25rem;
    color: var(--color-prussian-blue);
  }

  .toolbar__left,
  .toolbar__right {
    display: flex;
    align-items: center;
    gap: 1rem;
  }

  .toolbar__actions {
    display: inline-flex;
    gap: 0.5rem;
    flex-wrap: wrap;
  }

  .toolbar__button {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    padding: 0.55rem 1rem;
    border-radius: 999px;
    border: 1px solid transparent;
    font-weight: 600;
    cursor: pointer;
    transition: transform 0.2s ease, box-shadow 0.2s ease;
  }

  .toolbar__button--primary {
    background: var(--color-verdigris);
    color: var(--color-black-bean);
  }

  .toolbar__button--secondary {
    background: var(--color-pink-lavender);
    color: var(--color-black-bean);
  }

  .toolbar__button--ghost {
    background: transparent;
    border-color: rgba(28, 49, 68, 0.1);
    color: rgba(28, 49, 68, 0.75);
  }

  .toolbar__button:hover {
    transform: translateY(-1px);
    box-shadow: 0 8px 16px -14px rgba(55, 0, 10, 0.8);
  }

  .toolbar__icon {
    font-size: 0.85rem;
  }

  @media (max-width: 768px) {
    .toolbar {
      flex-direction: column;
      align-items: stretch;
    }

    .toolbar__left,
    .toolbar__right {
      width: 100%;
      justify-content: space-between;
    }
  }
</style>
