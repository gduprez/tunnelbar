<script lang="ts">
  import type { Translate } from '../lib/format';

  let { t }: { t: Translate } = $props();
</script>

<div class="note" role="note">
  <p><strong>{t('tray_missing.title')}</strong></p>
  <p>{t('tray_missing.body')}</p>
  <pre class="mono">sudo apt install gnome-shell-extension-appindicator</pre>
  <p>{t('tray_missing.then')}</p>
</div>

<style>
  .note {
    margin: 0 12px 8px;
    padding: 10px 12px;
    border: 1px solid var(--separator);
    border-radius: 10px;
    background: var(--surface);
    font-size: 12px;
    line-height: 1.45;
  }

  p {
    margin: 0 0 6px;
  }

  pre {
    margin: 0 0 6px;
    padding: 6px 8px;
    border-radius: 6px;
    background: var(--surface-strong);
    white-space: pre-wrap;
    user-select: text;
    -webkit-user-select: text;
  }
</style>
