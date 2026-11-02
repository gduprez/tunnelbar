<script lang="ts">
  import type { TunnelState } from '../lib/types';

  let { state }: { state: TunnelState } = $props();
</script>

<span class="dot {state.state}" aria-hidden="true"></span>

<style>
  .dot {
    flex: none;
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: var(--dot-stopped);
  }

  .up {
    background: var(--dot-up);
  }

  .connecting,
  .reconnecting {
    background: var(--dot-pending);
    animation: pulse 1.2s ease-in-out infinite;
  }

  .failed {
    background: var(--dot-failed);
  }

  @keyframes pulse {
    50% {
      opacity: 0.35;
    }
  }
</style>
