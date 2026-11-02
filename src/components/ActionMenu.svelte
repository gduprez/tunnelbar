<script lang="ts" module>
  export interface MenuItem {
    label: string;
    danger?: boolean;
    separatorBefore?: boolean;
    onselect: () => void;
  }
</script>

<script lang="ts">
  import { tick } from 'svelte';

  let { label, items }: { label: string; items: MenuItem[] } = $props();

  let open = $state(false);
  let trigger = $state<HTMLButtonElement>();
  let menu = $state<HTMLDivElement>();

  async function toggle() {
    open = !open;
    if (open) {
      await tick();
      menu?.querySelector<HTMLButtonElement>('[role="menuitem"]')?.focus();
    }
  }

  function close(refocus: boolean) {
    open = false;
    if (refocus) trigger?.focus();
  }

  function select(item: MenuItem) {
    close(false);
    item.onselect();
  }

  function onkeydown(event: KeyboardEvent) {
    const entries = [...(menu?.querySelectorAll<HTMLButtonElement>('[role="menuitem"]') ?? [])];
    const index = entries.indexOf(document.activeElement as HTMLButtonElement);
    if (event.key === 'Escape') {
      event.preventDefault();
      close(true);
    } else if (event.key === 'ArrowDown' || event.key === 'ArrowUp') {
      event.preventDefault();
      const step = event.key === 'ArrowDown' ? 1 : -1;
      entries[(index + step + entries.length) % entries.length]?.focus();
    }
  }

  function onwindowclick(event: MouseEvent) {
    const target = event.target as Node;
    if (open && !menu?.contains(target) && !trigger?.contains(target)) close(false);
  }
</script>

<svelte:window onclick={onwindowclick} />

<div class="anchor">
  <button
    bind:this={trigger}
    type="button"
    class="icon"
    aria-label={label}
    title={label}
    aria-haspopup="menu"
    aria-expanded={open}
    onclick={toggle}
  >
    <svg width="16" height="16" viewBox="0 0 16 16" fill="currentColor" aria-hidden="true">
      <circle cx="3.5" cy="8" r="1.4" />
      <circle cx="8" cy="8" r="1.4" />
      <circle cx="12.5" cy="8" r="1.4" />
    </svg>
  </button>
  {#if open}
    <div bind:this={menu} class="menu" role="menu" aria-label={label} tabindex="-1" {onkeydown}>
      {#each items as item (item.label)}
        {#if item.separatorBefore}
          <div class="separator" role="separator"></div>
        {/if}
        <button
          type="button"
          role="menuitem"
          class:danger={item.danger}
          onclick={() => select(item)}
        >
          {item.label}
        </button>
      {/each}
    </div>
  {/if}
</div>

<style>
  .anchor {
    position: relative;
  }

  .menu {
    position: absolute;
    top: 36px;
    right: 0;
    z-index: 10;
    display: flex;
    flex-direction: column;
    width: 220px;
    padding: 5px;
    box-sizing: border-box;
    border: 1px solid var(--separator);
    border-radius: 10px;
    background: var(--menu-background);
    box-shadow: var(--menu-shadow);
  }

  [role='menuitem'] {
    height: 30px;
    padding: 0 9px;
    border-radius: 6px;
    background: none;
    font-weight: normal;
    text-align: left;
  }

  [role='menuitem']:hover,
  [role='menuitem']:focus-visible {
    outline: none;
    background: var(--accent);
    color: white;
  }

  [role='menuitem'].danger {
    color: var(--error-text);
  }

  [role='menuitem'].danger:hover,
  [role='menuitem'].danger:focus-visible {
    background: var(--error);
    color: white;
  }

  .separator {
    height: 1px;
    margin: 4px 6px;
    background: var(--separator);
  }
</style>
