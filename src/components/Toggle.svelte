<script lang="ts">
  let { checked, label, onchange }: { checked: boolean; label: string; onchange: () => void } =
    $props();
</script>

<button
  type="button"
  role="switch"
  class="toggle"
  class:on={checked}
  aria-checked={checked}
  aria-label={label}
  title={label}
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
    width: 34px;
    height: 20px;
    padding: 0;
    border: none;
    border-radius: 10px;
    background: var(--toggle-off);
    cursor: pointer;
    transition: background 0.15s;
  }

  .toggle.on {
    background: var(--accent);
  }

  .knob {
    position: absolute;
    top: 2px;
    left: 2px;
    width: 16px;
    height: 16px;
    border-radius: 50%;
    background: white;
    box-shadow: 0 1px 2px rgb(0 0 0 / 0.3);
    transition: transform 0.15s;
  }

  .on .knob {
    transform: translateX(14px);
  }
</style>
