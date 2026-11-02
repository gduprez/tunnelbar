<script lang="ts">
  import { validationMessage, type Translate } from '../lib/format';
  import type { ConfigProblem } from '../lib/types';

  let { problem, t, onopen }: { problem: ConfigProblem; t: Translate; onopen: () => void } =
    $props();
</script>

<div class="banner" role="alert">
  {#if problem.kind === 'io'}
    <p>{t('config.problem.io', { message: problem.message })}</p>
  {:else if problem.kind === 'parse'}
    <p>
      {problem.line == null
        ? t('config.problem.parse', { message: problem.message })
        : t('config.problem.parse_line', { message: problem.message, line: problem.line })}
    </p>
  {:else if problem.kind === 'invalid'}
    <p>{t('config.problem.invalid')}</p>
    <ul>
      {#each problem.errors as error, index (index)}
        <li>
          {#if error.tunnel != null}{t('config.problem.tunnel', { number: error.tunnel + 1 })}{/if}
          {validationMessage(t, error)}
        </li>
      {/each}
    </ul>
  {:else}
    <p>{t('config.problem.file_broken')}</p>
  {/if}
  <button type="button" class="link" onclick={onopen}>{t('config.open_file')}</button>
</div>

<style>
  .banner {
    margin: 0 12px 8px;
    padding: 10px 12px;
    border: 1px solid var(--error-border);
    border-radius: 10px;
    background: var(--error-soft);
    font-size: 12px;
    line-height: 1.45;
  }

  p,
  ul {
    margin: 0 0 6px;
  }

  ul {
    padding-left: 18px;
  }

  button {
    font-size: 12px;
    font-weight: 500;
  }
</style>
