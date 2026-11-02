<script lang="ts">
  import { errorMessage, isRunning, type Translate } from '../lib/format';
  import type { TunnelView } from '../lib/types';
  import { rowSubtitle, stateTone } from '../lib/view';
  import StatusDot from './StatusDot.svelte';
  import Toggle from './Toggle.svelte';

  let {
    tunnels,
    t,
    now,
    ontoggle,
    onopen,
    onadd,
  }: {
    tunnels: TunnelView[];
    t: Translate;
    now: number;
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
      {@const tone = stateTone(tunnel.status.state)}
      {@const state = tunnel.status.state}
      <li class="card {tone}">
        <div class="row">
          <button type="button" class="open" onclick={() => onopen(tunnel)}>
            <StatusDot {tone} />
            <span class="text">
              <span class="name">{tunnel.name}</span>
              <span class="subtitle {tone}">{rowSubtitle(t, tunnel, now)}</span>
            </span>
          </button>
          <Toggle
            checked={isRunning(state)}
            {tone}
            label={t('toggle.label', { name: tunnel.name })}
            onchange={() => ontoggle(tunnel)}
          />
        </div>
        {#if state.state === 'failed'}
          <p class="explanation">{errorMessage(t, state.error)}</p>
          <button type="button" class="link details" onclick={() => onopen(tunnel)}>
            {t('list.see_output')}
          </button>
        {/if}
      </li>
    {/each}
  </ul>
{/if}

<style>
  .list {
    display: flex;
    flex-direction: column;
    gap: 4px;
    margin: 0;
    padding: 0 8px 8px;
    list-style: none;
  }

  .card {
    border-radius: 10px;
    border: 1px solid transparent;
    background: var(--surface);
  }

  .card.off {
    background: none;
  }

  .card.error {
    border-color: var(--error-border);
    background: var(--error-soft);
  }

  .row {
    display: flex;
    align-items: center;
    gap: 8px;
    padding-right: 10px;
  }

  .open {
    display: flex;
    flex: 1;
    align-items: center;
    gap: 12px;
    min-width: 0;
    height: auto;
    padding: 10px 0 10px 12px;
    background: none;
    font-weight: normal;
    text-align: left;
  }

  .text {
    display: flex;
    flex-direction: column;
    gap: 3px;
    min-width: 0;
  }

  .name {
    font-weight: 600;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .off .name {
    color: var(--secondary-text);
  }

  .subtitle {
    font-size: 12px;
    color: var(--secondary-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .subtitle.retry {
    color: var(--retry-text);
  }

  .subtitle.error {
    color: var(--error-text);
  }

  .explanation {
    margin: -2px 12px 6px 32px;
    font-size: 12px;
    line-height: 1.45;
  }

  .details {
    margin: 0 12px 10px 32px;
    font-size: 12px;
    font-weight: 500;
  }

  .empty {
    padding: 28px 16px 32px;
    text-align: center;
    color: var(--secondary-text);
  }

  .empty p {
    margin: 0 0 12px;
  }
</style>
