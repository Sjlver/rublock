<script lang="ts">
  import PageHeader from '../PageHeader.svelte';
  import PuzzleGrid from '../PuzzleGrid.svelte';
  import { t, md } from '../../i18n/index.svelte';
  import type { CellValue, PuzzleData } from '../../state/types';
  import type { MessageKey } from '../../i18n/en';

  const exampleSize = 5;
  const exampleTargets: PuzzleData = {
    row_targets: [6, 2, 5, 1, 0],
    col_targets: [3, 0, 3, 0, 0],
  };

  function emptyValues(): CellValue[][] {
    return Array.from({ length: exampleSize }, () => Array<CellValue>(exampleSize).fill(null));
  }

  const step0Values = emptyValues();

  const step1Values = emptyValues();
  step1Values[0][0] = 'black';
  step1Values[0][4] = 'black';
  const step1Extras = new Map([
    ['0,0', { exNew: true }],
    ['0,4', { exNew: true }],
  ]);

  const step2Values = emptyValues();
  step2Values[0][0] = 'black';
  step2Values[0][4] = 'black';
  step2Values[1][4] = 'black';
  const step2Extras = new Map([['1,4', { exNew: true }]]);

  const step3Values = emptyValues();
  step3Values[0][0] = 'black';
  step3Values[0][4] = 'black';
  step3Values[1][4] = 'black';
  step3Values[1][2] = 'black';
  step3Values[1][3] = 2;
  const step3Extras = new Map([
    ['1,2', { exNew: true }],
    ['1,3', { exNew: true }],
  ]);

  const controls: { id: string; action: MessageKey; touch: MessageKey; kb: MessageKey }[] = [
    {
      id: 'place_note',
      action: 'howto_ctrl_place_note',
      touch: 'howto_ctrl_place_note_touch',
      kb: 'howto_ctrl_place_note_kb',
    },
    {
      id: 'place_value',
      action: 'howto_ctrl_place_value',
      touch: 'howto_ctrl_place_value_touch',
      kb: 'howto_ctrl_place_value_kb',
    },
    {
      id: 'toggle_mode',
      action: 'howto_ctrl_toggle_mode',
      touch: 'howto_ctrl_toggle_mode_touch',
      kb: 'howto_ctrl_toggle_mode_kb',
    },
    {
      id: 'mark_black',
      action: 'howto_ctrl_mark_black',
      touch: 'howto_ctrl_mark_black_touch',
      kb: 'howto_ctrl_mark_black_kb',
    },
    {
      id: 'mark_digits',
      action: 'howto_ctrl_mark_digits',
      touch: 'howto_ctrl_mark_digits_touch',
      kb: 'howto_ctrl_mark_digits_kb',
    },
    {
      id: 'erase',
      action: 'howto_ctrl_erase',
      touch: 'howto_ctrl_erase_touch',
      kb: 'howto_ctrl_erase_kb',
    },
    {
      id: 'move',
      action: 'howto_ctrl_move',
      touch: 'howto_ctrl_move_touch',
      kb: 'howto_ctrl_move_kb',
    },
  ];
</script>

<PageHeader title={t('howto_title')} />

<div class="tab-content">
  <div class="card">
    <p class="howto-prose">
      {@html md(t('howto_intro'))}
    </p>

    <div class="rule-row">
      <div class="rule-number">1</div>
      <div>
        <div class="rule-title">{t('howto_rule1_title')}</div>
        <div class="rule-body">
          {@html md(t('howto_rule1_body'))}
        </div>
      </div>
    </div>

    <div class="rule-row">
      <div class="rule-number">2</div>
      <div>
        <div class="rule-title">{t('howto_rule2_title')}</div>
        <div class="rule-body">
          {@html md(t('howto_rule2_body'))}
        </div>
      </div>
    </div>

    <div class="rule-row">
      <div class="rule-number">3</div>
      <div>
        <div class="rule-title">{t('howto_rule3_title')}</div>
        <div class="rule-body">
          {@html md(t('howto_rule3_body'))}
        </div>
      </div>
    </div>

    <div class="divider"></div>

    <h2 style="font-size:13.5px; font-weight:700; margin-bottom:8px;">
      {t('howto_example_heading')}
    </h2>
    <p class="howto-prose" style="font-size:13px;">
      {t('howto_example_intro')}
    </p>

    <!-- NOTE TO IMPLEMENTER: the worked examples below are from the original app.
         Keep them in the production redesign — see DESIGN_NOTES.md. -->

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step0Values} />
    </div>

    <h3 style="font-size:13px; font-weight:700; color:var(--accent-soft-ink); margin:12px 0 4px;">
      {t('howto_step1_heading')}
    </h3>
    <p class="howto-prose" style="font-size:13px;">
      {@html md(t('howto_step1_body'))}
    </p>

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step1Values} cellExtras={step1Extras} />
    </div>

    <h3 style="font-size:13px; font-weight:700; color:var(--accent-soft-ink); margin:12px 0 4px;">
      {t('howto_step2_heading')}
    </h3>
    <p class="howto-prose" style="font-size:13px;">
      {t('howto_step2_body')}
    </p>

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step2Values} cellExtras={step2Extras} />
    </div>

    <h3 style="font-size:13px; font-weight:700; color:var(--accent-soft-ink); margin:12px 0 4px;">
      {t('howto_step3_heading')}
    </h3>
    <p class="howto-prose" style="font-size:13px;">
      {@html md(t('howto_step3_body'))}
    </p>

    <div class="howto-step">
      <PuzzleGrid puzzle={exampleTargets} values={step3Values} cellExtras={step3Extras} />
    </div>

    <p class="howto-prose" style="font-size:13px; margin-top:10px;">
      {t('howto_outro')}
    </p>

    <div class="divider"></div>

    <h2 style="font-size:13.5px; font-weight:700; margin-bottom:8px;">
      {t('howto_controls_heading')}
    </h2>
    <div class="controls-table">
      <div
        class="controls-row-item"
        style="padding-bottom:6px; font-size:11px; font-weight:700;
           color:var(--muted); text-transform:uppercase; letter-spacing:0.05em;"
      >
        <div>{t('howto_controls_action')}</div>
        <div>{t('howto_controls_touch')}</div>
        <div>{t('howto_controls_kb')}</div>
      </div>
      {#each controls as row (row.id)}
        <div class="controls-row-item">
          <div class="controls-action">{t(row.action)}</div>
          <div class="controls-touch">{t(row.touch)}</div>
          <div class="controls-kb">{t(row.kb)}</div>
        </div>
      {/each}
    </div>

    <p style="margin-top:12px; font-size:12px; color:var(--muted); line-height:1.5;">
      {t('howto_source')}
      <a
        href="https://github.com/Sjlver/rublock"
        target="_blank"
        rel="noopener noreferrer"
        style="color:var(--accent); text-decoration:none;"
      >
        github.com/Sjlver/rublock
      </a>
    </p>
  </div>
</div>
