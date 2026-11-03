<script lang="ts">
  import type { Tone } from '../lib/view';

  let {
    checked,
    label,
    labelledby,
    tone = 'pending',
    disabled = false,
    onchange,
  }: {
    checked: boolean;
    /** Accessible name; or `labelledby`, the id of a visible label. */
    label?: string;
    labelledby?: string;
    tone?: Tone;
    disabled?: boolean;
    onchange: () => void;
  } = $props();
</script>

<button
  type="button"
  role="switch"
  class="toggle {tone}"
  class:on={checked}
  aria-checked={checked}
  aria-label={label}
  aria-labelledby={labelledby}
  title={label}
  {disabled}
  onclick={(event) => {
    event.stopPropagation();
    onchange();
  }}
>
  <span class="knob"></span>
</button>

<style>
  .toggle {
    position: relative;
    flex: none;
    width: 38px;
    height: 22px;
    padding: 0;
    border-radius: 11px;
    background: var(--toggle-off);
    transition: background 0.15s;
  }

  .on {
    background: var(--accent);
  }

  .on.up {
    background: var(--up);
  }

  .on.retry {
    background: var(--retry);
  }

  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 18px;
    height: 18px;
    border-radius: 50%;
    background: white;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.35);
    transition: transform 0.15s;
  }

  .on .knob {
    transform: translateX(16px);
  }
</style>
