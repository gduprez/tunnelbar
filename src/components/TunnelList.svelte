<script lang="ts">
  import {
    forwardSummary,
    isRunning,
    stateText,
    tunnelTarget,
    type Translate,
  } from '../lib/format';
  import type { TunnelView } from '../lib/types';
  import StatusDot from './StatusDot.svelte';
  import Toggle from './Toggle.svelte';

  let {
    tunnels,
    t,
    ontoggle,
    onopen,
    onadd,
  }: {
    tunnels: TunnelView[];
    t: Translate;
    ontoggle: (tunnel: TunnelView) => void;
    onopen: (tunnel: TunnelView) => void;
    onadd: () => void;
  } = $props();
</script>

{#if tunnels.length === 0}
  <div class="empty">
    <p>{t('panel.empty')}</p>
    <button type="button" class="primary" onclick={onadd}>{t('panel.add')}</button>
  </div>
{:else}
  <ul class="list">
    {#each tunnels as tunnel (tunnel.id)}
      <li>
        <div
          class="row"
          role="button"
          tabindex="0"
          onclick={() => onopen(tunnel)}
          onkeydown={(event) => event.key === 'Enter' && onopen(tunnel)}
        >
          <StatusDot state={tunnel.status.state} />
          <div class="text">
            <div class="name">{tunnel.name}</div>
            <div class="summary">
              {#if tunnel.status.state.state === 'stopped' || tunnel.status.state.state === 'up'}
                {tunnel.forward[0] ? forwardSummary(tunnel.forward[0]) : ''}
                {t('detail.via', { target: tunnelTarget(tunnel) })}
              {:else}
                {stateText(t, tunnel.status.state)}
              {/if}
            </div>
          </div>
          <Toggle
            checked={isRunning(tunnel.status.state)}
            label={t('toggle.label', { name: tunnel.name })}
            onchange={() => ontoggle(tunnel)}
          />
        </div>
      </li>
    {/each}
  </ul>
{/if}

<style>
  .list {
    margin: 0;
    padding: 0 6px;
    list-style: none;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 8px;
    border-radius: 8px;
    cursor: pointer;
  }

  .row:hover,
  .row:focus-visible {
    background: var(--hover);
    outline: none;
  }

  .text {
    flex: 1;
    min-width: 0;
  }

  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .summary {
    color: var(--secondary-text);
    font-size: 11px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    padding: 32px 16px;
    text-align: center;
    color: var(--secondary-text);
  }
</style>
