<script lang="ts">
  import type { Tone } from '../lib/view';

  let { tone, large = false }: { tone: Tone; large?: boolean } = $props();
</script>

{#if tone === 'pending' || tone === 'retry'}
  <svg
    class="spinner {tone}"
    class:large
    width="10"
    height="10"
    viewBox="0 0 10 10"
    aria-hidden="true"
  >
    <circle class="track" cx="5" cy="5" r="3.6" fill="none" stroke-width="2" />
    <path d="M5 1.4a3.6 3.6 0 0 1 3.6 3.6" fill="none" stroke-width="2" stroke-linecap="round" />
  </svg>
{:else}
  <span class="dot {tone}" class:large aria-hidden="true"></span>
{/if}

<style>
  .dot {
    flex: none;
    box-sizing: border-box;
    width: 8px;
    height: 8px;
    border-radius: 50%;
  }

  .dot.large {
    width: 10px;
    height: 10px;
  }

  .up {
    background: var(--up);
    box-shadow: 0 0 0 3px var(--up-soft);
  }

  .error {
    background: var(--error);
    box-shadow: 0 0 0 3px var(--error-soft);
  }

  .off {
    border: 1.5px solid var(--off);
  }

  .spinner {
    flex: none;
    margin: 0 -1px;
    animation: spin 1s linear infinite;
  }

  .spinner.large {
    width: 12px;
    height: 12px;
  }

  .spinner.pending {
    stroke: var(--accent);
  }

  .spinner.retry {
    stroke: var(--retry);
  }

  .track {
    opacity: 0.3;
  }

  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }

  @media (prefers-reduced-motion: reduce) {
    .spinner {
      animation: none;
    }
  }
</style>
