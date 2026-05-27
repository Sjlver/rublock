import type { Messages } from './en';

// European Portuguese catalog (pt-PT). Typed `: Messages` so any missing key
// is a compile error. Keep placeholders (`{n}`, `{*…*}`, `{err}`, `{others}`)
// intact. Uses informal "tu" register, matching the German catalog.
export const pt: Messages = {
  // Bottom nav
  nav_play: 'Jogar',
  nav_steps: 'Resolução',
  nav_create: 'Criar',
  nav_print: 'Imprimir',
  nav_howto: 'Como jogar',
  nav_aria: 'Secções principais',

  // Page header
  header_share: 'Partilhar',
  header_share_aria: 'Partilhar',
  header_locale_aria: 'Idioma',

  // Play tab
  play_title: 'Jogar',
  play_new_puzzle: 'Novo puzzle',
  play_new_puzzle_difficulty_aria: 'Escolher dificuldade',
  play_undo: 'Anular',
  play_redo: 'Refazer',
  play_check: 'Verificar',
  play_check_aria: 'Verificar respostas',
  play_undo_aria: 'Anular',
  play_redo_aria: 'Refazer',
  play_notes_mode: 'Modo notas',
  play_space_to_switch: 'Espaço para alternar',
  play_hint_chip: 'Toque para nota · prime para resposta',
  play_hint_dismiss_aria: 'Fechar dica',
  play_status_switching: 'A alternar…',
  play_status_generating: 'A gerar…',

  // InputBar
  inputbar_aria: 'Botões de entrada',
  inputbar_black_aria: 'Célula preta (premir) / nota (toque)',
  inputbar_digit_aria: 'Dígito {n} (premir para colocar, toque para nota)',
  inputbar_o_aria: 'Tem de ser um dígito (nota)',
  inputbar_erase_aria: 'Apagar célula',

  // Create tab
  create_title: 'Criar',
  create_set_target_aria: 'Definir alvo para {n}',
  create_values_aria: 'Botões de valor alvo',
  create_use: 'Usar este puzzle',
  create_use_aria: 'Usar este puzzle',
  create_use_title_valid: 'Enviar este puzzle para o separador Jogar',
  create_use_title_invalid: 'Apenas puzzles com solução única podem ser usados',
  create_share_invalid: 'Não é possível partilhar um puzzle inválido',

  // Print tab
  print_title: 'Imprimir',
  print_status: 'Gerar uma brochura imprimível',
  print_size: 'Tamanho',
  print_pages: 'Páginas',
  print_six_per_page: 'Seis puzzles por página',
  print_fewer_aria: 'Menos páginas',
  print_more_aria: 'Mais páginas',
  print_busy: 'A gerar…',
  print_btn_one: 'Gerar {n} página',
  print_btn_other: 'Gerar {n} páginas',
  print_footnote: 'Tamanhos maiores e brochuras mais longas demoram um momento a gerar.',

  // Print output
  print_promo: 'Puzzles Doplo gerados em {url}',

  // How-to tab
  howto_title: 'Como jogar',
  howto_intro:
    'Cada linha e coluna tem um {*alvo*} no topo. Coloca os dígitos e dois quadrados pretos para que o puzzle faça sentido:',
  howto_rule1_title: 'Dois pretos',
  howto_rule1_body: 'Cada linha e cada coluna contém exatamente {*dois quadrados pretos*}.',
  howto_rule2_title: 'Uma permutação no meio',
  howto_rule2_body:
    'As outras células de cada linha e coluna contêm os dígitos {*1 a N − 2*} — cada um a aparecer uma vez.',
  howto_rule3_title: 'Soma igual ao alvo',
  howto_rule3_body:
    'Os números {*entre*} os dois pretos têm de somar o alvo indicado. Um alvo de {*0*} significa que os dois pretos são adjacentes.',
  howto_example_heading: 'Exemplo passo a passo',
  howto_example_intro:
    'Aqui está um puzzle 5 × 5 novo. Os dígitos usados são 1, 2 e 3. Por onde começas?',
  howto_step1_heading: 'Passo 1 — O alvo 6 da linha é a soma máxima possível',
  howto_step1_body:
    'Dígitos 1 + 2 + 3 = {*6*}. Alvo 6 significa que todos os dígitos ficam entre os dois pretos, por isso os pretos vão para os extremos: coluna 1 e coluna 5.',
  howto_step2_heading: 'Passo 2 — O alvo 0 da coluna 5 significa que os pretos são vizinhos',
  howto_step2_body:
    'Alvo 0 significa nada entre os pretos — têm de ser adjacentes. A coluna 5 já tem um preto na linha 1, por isso o segundo preto fica logo abaixo, na linha 2.',
  howto_step3_heading: 'Passo 3 — O alvo 2 da linha 2 fixa o segundo preto',
  howto_step3_body:
    'A linha 2 já tem um preto na coluna 5. Alvo 2 significa que apenas o dígito {*2*} fica entre os pretos. Coloca o 2 na coluna 4 e o segundo preto na coluna 3.',
  howto_outro:
    'Cada dedução abre a próxima. Continua até o puzzle estar completo — há sempre exatamente uma solução.',
  howto_controls_heading: 'Controlos',
  howto_controls_action: 'Ação',
  howto_controls_touch: 'Toque / rato',
  howto_controls_kb: 'Teclado',
  howto_ctrl_place_note: 'Colocar uma nota',
  howto_ctrl_place_note_touch: 'Tocar no botão',
  howto_ctrl_place_note_kb: 'Espaço, depois dígito',
  howto_ctrl_place_value: 'Colocar uma resposta',
  howto_ctrl_place_value_touch: 'Premir o botão',
  howto_ctrl_place_value_kb: 'Dígito (predefinido)',
  howto_ctrl_toggle_mode: 'Alternar modo nota/resposta',
  howto_ctrl_toggle_mode_touch: '—',
  howto_ctrl_toggle_mode_kb: 'Espaço',
  howto_ctrl_mark_black: 'Marcar célula como preta',
  howto_ctrl_mark_black_touch: 'Premir ■',
  howto_ctrl_mark_black_kb: '0, B ou X',
  howto_ctrl_mark_digits: 'Marcar como dígito',
  howto_ctrl_mark_digits_touch: 'Tocar em ○',
  howto_ctrl_mark_digits_kb: '9 ou O',
  howto_ctrl_erase: 'Apagar célula',
  howto_ctrl_erase_touch: 'Tocar na borracha',
  howto_ctrl_erase_kb: 'Backspace / Delete',
  howto_ctrl_move: 'Mover seleção',
  howto_ctrl_move_touch: 'Tocar na célula',
  howto_ctrl_move_kb: 'Setas / WASD',
  howto_source: 'Código fonte:',

  // Walkthrough tab
  wt_title: 'Resolução',
  wt_status_no_puzzle: 'Nenhum puzzle carregado.',
  wt_placeholder:
    'Escolhe um puzzle no separador {*Jogar*} ou {*Criar*}. A resolução passo a passo aparecerá aqui.',
  wt_error: 'Não foi possível gerar a resolução: {err}',
  wt_intro1:
    'Vê o solucionador a desbravar o puzzle atual. Cada grelha abaixo é uma {*onda*} — cada alteração numa onda decorre apenas do que era conhecido antes dela.',
  wt_intro2:
    'As células começam com todos os dígitos (números pequenos) mais um {*x*} para "pode ser preto". À medida que opções são descartadas, as notas encolhem. Quando resta apenas uma opção, a célula é preenchida. As células que mudaram numa onda estão destacadas a amarelo.',
  wt_start: 'Início',
  wt_start_sub: 'Cada célula ainda pode conter qualquer dígito ou ser preta.',
  wt_wave: 'Onda {n}',
  wt_wave_one: '· {n} nota removida',
  wt_wave_other: '· {n} notas removidas',
  wt_search: 'Pesquisa',
  wt_guess_one: '· {n} tentativa',
  wt_guess_other: '· {n} tentativas',
  wt_search_sub:
    'A dedução pura não chega aqui — a partir deste ponto, o solucionador testa hipóteses e recua. O resto é demasiado denso para mostrar grelha a grelha, por isso saltamos diretamente para a solução.',
  wt_solved: 'Resolvido',
  wt_solved_sub: 'O puzzle final.',
  wt_status_waves_one: '{n} onda',
  wt_status_waves_other: '{n} ondas',
  wt_status_removed_one: '{n} nota removida',
  wt_status_removed_other: '{n} notas removidas',
  wt_status_join: ' · ',

  // Walkthrough rule labels
  wt_rule_target_tuples: 'Somas-alvo',
  wt_rule_arc: 'Verificação de possibilidades',
  wt_rule_singleton: 'Células forçadas',
  wt_rule_hidden: 'Único lugar',
  wt_rule_black: 'Regra dos dois pretos',
  wt_rule_backtrack: 'Hipótese',

  // Walkthrough rule notes — solo: mostrado quando a onda usa apenas esta regra.
  wt_rule_target_tuples_note:
    'Algumas posições de dígito ou preto não podem fazer parte de qualquer arranjo que some o alvo da linha ou coluna — essas são removidas.',
  wt_rule_arc_note:
    'Nenhum arranjo restante desta linha ou coluna suporta ainda estas opções, por isso são eliminadas.',
  wt_rule_singleton_note:
    'Uma célula próxima está agora totalmente determinada, e o seu valor não se pode repetir no resto da linha ou coluna.',
  wt_rule_hidden_note:
    'Apenas uma célula desta linha ou coluna ainda pode conter este dígito ou preto, por isso as outras perdem-no como candidato.',
  wt_rule_black_note:
    'Cada linha e cada coluna tem exatamente dois pretos. Estas opções criariam um terceiro — por isso desaparecem.',
  wt_rule_backtrack_note:
    'O solucionador testou uma hipótese para sair de um impasse. Raro em puzzles resolvíveis à mão.',

  // Walkthrough rule notes — dominante: mostrado quando esta regra lidera uma onda mista.
  wt_rule_target_tuples_dominant:
    'A maioria das eliminações desta onda vem das somas-alvo — algumas posições de dígito ou preto não podem fazer parte de qualquer arranjo que some o alvo da linha ou coluna.',
  wt_rule_arc_dominant:
    'A maioria das eliminações desta onda vem da verificação de possibilidades — nenhum arranjo restante da linha ou coluna suporta ainda estas opções.',
  wt_rule_singleton_dominant:
    'A maioria das eliminações desta onda vem de células forçadas — uma célula próxima está agora totalmente determinada, e o seu valor não se pode repetir no resto da linha ou coluna.',
  wt_rule_hidden_dominant:
    'A maioria das eliminações desta onda vem da regra do único lugar — apenas uma célula de uma linha ou coluna ainda pode conter um dado dígito ou preto, por isso as outras perdem-no como candidato.',
  wt_rule_black_dominant:
    'A maioria das eliminações desta onda vem da regra dos dois pretos — cada linha e cada coluna tem exatamente dois pretos, por isso opções que criariam um terceiro são removidas.',
  wt_rule_backtrack_dominant:
    'A maioria das eliminações desta onda vem de hipóteses — o solucionador testou tentativas para sair de um impasse. Raro em puzzles resolvíveis à mão.',

  // Classification chip
  cls_no_solution: 'Sem solução',
  cls_multiple: 'Várias soluções',
  cls_easy: 'Fácil',
  cls_medium: 'Médio',
  cls_challenging: 'Desafiador',
  cls_hard: 'Difícil',
  cls_very_hard: 'Muito difícil',
  cls_extremely_hard: 'Extremamente difícil',

  // Toasts
  toast_solved: 'Puzzle resolvido! 🎉',
  toast_check_empty: 'Preenche algumas células e depois verifica-as.',
  toast_check_all_correct: 'Todas as células preenchidas estão corretas.',
  toast_one_wrong: 'Uma célula errada.',
  toast_n_wrong: '{n} células erradas.',
  toast_link_copied: 'Ligação copiada para a área de transferência',
  toast_share_failed: 'Não foi possível partilhar este puzzle',

  // Web Share API
  share_title: 'Puzzle Doplo',
  share_text: 'Experimenta este puzzle Doplo:',

  // Support CTA (post-solve)
  support_copy_1: 'Gostas do rublock? Apoia o seu desenvolvimento.',
  support_copy_2: 'O rublock é gratuito e sem anúncios — ajuda a mantê-lo assim.',
  support_copy_3: 'Gostas destes puzzles? Contribui para que continuem.',
  support_copy_4: 'Feito por uma só pessoa. Um pequeno gesto ajuda muito.',
  support_copy_5: 'Se o rublock alegrou o teu dia, considera apoiá-lo.',
  support_button: 'Apoiar no {platform}',
  support_dismiss_aria: 'Fechar',

  // Wasm errors
  err_row_targets_length: 'O número de alvos de linha não corresponde ao tamanho do puzzle.',
  err_col_targets_length: 'O número de alvos de coluna não corresponde ao tamanho do puzzle.',
  err_targets_length_mismatch: 'Os alvos de linha e de coluna têm de ter o mesmo número.',
  err_size_range: 'O tamanho deve estar entre 5 e 8.',
  err_unsolvable: 'O puzzle não tem solução.',
  err_multiple_solutions: 'O puzzle tem várias soluções.',
  err_incomplete_state: 'O solucionador devolveu um estado incompleto.',
  err_target_out_of_range:
    'O alvo {t} está fora do intervalo (máximo {max} para o tamanho {size}).',

  // Locale switcher — autonyms (language's own name); same in every catalog.
  loc_en: 'English',
  loc_de: 'Deutsch',
  loc_pt: 'Português',
  loc_fr: 'Français',
};
