import type { Messages } from './en';

// German catalog. Typed `: Messages` so any missing key is a compile error.
// Keep placeholders (`{n}`, `{*…*}`, `{err}`, `{others}`) intact.
export const de: Messages = {
  // Bottom nav
  nav_play: 'Spielen',
  nav_steps: 'Lösungsweg',
  nav_create: 'Erstellen',
  nav_print: 'Drucken',
  nav_howto: 'Anleitung',
  nav_aria: 'Hauptbereiche',

  // Page header
  header_share: 'Teilen',
  header_share_aria: 'Teilen',
  header_locale_aria: 'Sprache',

  // Play tab
  play_title: 'Spielen',
  play_new_puzzle: 'Neues Puzzle',
  play_undo: 'Zurück',
  play_redo: 'Vor',
  play_check: 'Prüfen',
  play_check_aria: 'Antworten prüfen',
  play_undo_aria: 'Zurück',
  play_redo_aria: 'Vor',
  play_notes_mode: 'Notizmodus',
  play_space_to_switch: 'Leertaste wechselt',
  play_hint_chip: 'Tippen für Notiz · Halten für Antwort',
  play_hint_dismiss_aria: 'Hinweis schließen',
  play_status_switching: 'Wechseln…',
  play_status_generating: 'Erstellen…',

  // InputBar
  inputbar_aria: 'Eingabetasten',
  inputbar_black_aria: 'Schwarze Zelle (halten) / Notiz (tippen)',
  inputbar_digit_aria: 'Ziffer {n} (halten zum Setzen, tippen für Notiz)',
  inputbar_o_aria: 'Muss eine Ziffer sein (Notiz)',
  inputbar_erase_aria: 'Zelle löschen',

  // Create tab
  create_title: 'Erstellen',
  create_set_target_aria: 'Zielwert auf {n} setzen',
  create_values_aria: 'Zielwerte',
  create_use: 'Dieses Puzzle verwenden',
  create_use_aria: 'Dieses Puzzle verwenden',
  create_use_title_valid: 'Dieses Puzzle im Spiel-Tab öffnen',
  create_use_title_invalid: 'Nur Puzzles mit eindeutiger Lösung können verwendet werden',
  create_share_invalid: 'Ungültiges Puzzle kann nicht geteilt werden',

  // Print tab
  print_title: 'Drucken',
  print_status: 'Druckbares Heft erstellen',
  print_size: 'Größe',
  print_pages: 'Seiten',
  print_six_per_page: 'Sechs Puzzles pro Seite',
  print_fewer_aria: 'Weniger Seiten',
  print_more_aria: 'Mehr Seiten',
  print_busy: 'Erstellen…',
  print_btn_one: '{n} Seite erstellen',
  print_btn_other: '{n} Seiten erstellen',
  print_footnote: 'Größere Puzzles und längere Hefte brauchen einen Moment.',

  // Print output
  print_promo: 'Doplo-Puzzles erstellt mit {url}',

  // How-to tab
  howto_title: 'Anleitung',
  howto_intro:
    'Jede Zeile und Spalte hat einen {*Zielwert*} am Anfang. Setze die Ziffern und zwei schwarze Felder so, dass das Puzzle aufgeht:',
  howto_rule1_title: 'Zwei schwarze Felder',
  howto_rule1_body: 'Jede Zeile und jede Spalte enthält genau {*zwei schwarze Felder*}.',
  howto_rule2_title: 'Eine Permutation dazwischen',
  howto_rule2_body:
    'Die übrigen Felder jeder Zeile und Spalte enthalten die Ziffern {*1 bis N − 2*} — jede genau einmal.',
  howto_rule3_title: 'Summe ergibt den Zielwert',
  howto_rule3_body:
    'Die Zahlen {*zwischen*} den beiden schwarzen Feldern müssen den Zielwert ergeben. Ein Zielwert von {*0*} bedeutet, dass die schwarzen Felder direkt nebeneinander liegen.',
  howto_example_heading: 'Beispiel Schritt für Schritt',
  howto_example_intro:
    'Hier ist ein frisches 5 × 5 Puzzle. Verwendet werden die Ziffern 1, 2 und 3. Wo fängst du an?',
  howto_step1_heading: 'Schritt 1 — Zeilen-Zielwert 6 ist die größtmögliche Summe',
  howto_step1_body:
    'Ziffern 1 + 2 + 3 = {*6*}. Zielwert 6 heißt: jede Ziffer liegt zwischen den beiden schwarzen Feldern, also gehören diese ganz an den Rand: Spalte 1 und Spalte 5.',
  howto_step2_heading: 'Schritt 2 — Spalten-Zielwert 0 heißt, die schwarzen Felder sind benachbart',
  howto_step2_body:
    'Zielwert 0 heißt: nichts zwischen den schwarzen Feldern — sie liegen direkt nebeneinander. Spalte 5 hat schon ein schwarzes Feld in Zeile 1, das zweite folgt also direkt darunter in Zeile 2.',
  howto_step3_heading: 'Schritt 3 — Zeilen-Zielwert 2 fixiert das zweite schwarze Feld',
  howto_step3_body:
    'Zeile 2 hat schon ein schwarzes Feld in Spalte 5. Zielwert 2 heißt: genau die Ziffer {*2*} liegt zwischen den schwarzen Feldern. Setze die 2 in Spalte 4, das zweite schwarze Feld in Spalte 3.',
  howto_outro:
    'Jeder Schluss schaltet den nächsten frei. Mach so weiter, bis das Puzzle gelöst ist — es gibt immer genau eine Lösung.',
  howto_controls_heading: 'Steuerung',
  howto_controls_action: 'Aktion',
  howto_controls_touch: 'Touch / Maus',
  howto_controls_kb: 'Tastatur',
  howto_ctrl_place_note: 'Notiz setzen',
  howto_ctrl_place_note_touch: 'Taste tippen',
  howto_ctrl_place_note_kb: 'Leertaste, dann Ziffer',
  howto_ctrl_place_value: 'Antwort setzen',
  howto_ctrl_place_value_touch: 'Taste halten',
  howto_ctrl_place_value_kb: 'Ziffer (Standard)',
  howto_ctrl_toggle_mode: 'Notiz/Antwort-Modus wechseln',
  howto_ctrl_toggle_mode_touch: '—',
  howto_ctrl_toggle_mode_kb: 'Leertaste',
  howto_ctrl_mark_black: 'Feld schwarz markieren',
  howto_ctrl_mark_black_touch: '■ halten',
  howto_ctrl_mark_black_kb: '0, B oder X',
  howto_ctrl_mark_digits: 'Als Ziffer markieren',
  howto_ctrl_mark_digits_touch: '○ tippen',
  howto_ctrl_mark_digits_kb: '9 oder O',
  howto_ctrl_erase: 'Zelle löschen',
  howto_ctrl_erase_touch: 'Radierer tippen',
  howto_ctrl_erase_kb: 'Backspace / Entf',
  howto_ctrl_move: 'Auswahl verschieben',
  howto_ctrl_move_touch: 'Zelle tippen',
  howto_ctrl_move_kb: 'Pfeiltasten / WASD',
  howto_source: 'Quellcode:',

  // Walkthrough tab
  wt_title: 'Lösungsweg',
  wt_status_no_puzzle: 'Kein Puzzle geladen.',
  wt_placeholder:
    'Wähle ein Puzzle im {*Spielen*}- oder {*Erstellen*}-Tab. Der Lösungsweg erscheint dann hier.',
  wt_error: 'Lösungsweg konnte nicht erstellt werden: {err}',
  wt_intro1:
    'Sieh zu, wie der Löser das aktuelle Puzzle Stück für Stück knackt. Jedes Gitter unten ist eine {*Welle*} — jede Änderung in einer Welle folgt aus dem, was vorher schon bekannt war.',
  wt_intro2:
    'Die Felder starten mit allen Ziffern (kleine Zahlen) plus einem {*x*} für „könnte schwarz sein". Sobald Möglichkeiten ausgeschlossen werden, schrumpfen die Notizen. Bleibt nur eine übrig, wird das Feld gefüllt. Felder, die sich in einer Welle ändern, sind gelb hervorgehoben.',
  wt_start: 'Start',
  wt_start_sub: 'Jedes Feld könnte noch jede Ziffer halten oder schwarz sein.',
  wt_wave: 'Welle {n}',
  wt_wave_one: '· {n} Notiz entfernt',
  wt_wave_other: '· {n} Notizen entfernt',
  wt_search: 'Suche',
  wt_guess_one: '· {n} Vermutung',
  wt_guess_other: '· {n} Vermutungen',
  wt_search_sub:
    'Reine Logik kommt hier nicht weiter — der Löser probiert ab jetzt Hypothesen und macht Rückschritte. Der Rest ist zu dicht, um Gitter für Gitter zu zeigen, also springen wir direkt zur Lösung.',
  wt_solved: 'Gelöst',
  wt_solved_sub: 'Das fertige Puzzle.',
  wt_status_waves_one: '{n} Welle',
  wt_status_waves_other: '{n} Wellen',
  wt_status_removed_one: '{n} Notiz entfernt',
  wt_status_removed_other: '{n} Notizen entfernt',
  wt_status_join: ' · ',

  // Walkthrough rule labels
  wt_rule_target_tuples: 'Zielsummen',
  wt_rule_arc: 'Möglichkeitsabgleich',
  wt_rule_singleton: 'Erzwungene Felder',
  wt_rule_hidden: 'Einziger Platz',
  wt_rule_black: 'Zwei-Schwarze-Regel',
  wt_rule_backtrack: 'Hypothese',

  // Walkthrough rule notes — solo: gezeigt, wenn die Welle nur diese Regel nutzt.
  wt_rule_target_tuples_note:
    'Manche Ziffern oder schwarzen Felder können in keiner Anordnung vorkommen, die den Zielwert der Zeile oder Spalte ergibt — diese werden ausgeschlossen.',
  wt_rule_arc_note:
    'Keine verbleibende Anordnung dieser Zeile oder Spalte unterstützt diese Möglichkeiten noch — sie werden gestrichen.',
  wt_rule_singleton_note:
    'Ein nahes Feld ist jetzt eindeutig bestimmt; sein Wert kann im Rest der Zeile oder Spalte nicht erneut auftauchen.',
  wt_rule_hidden_note:
    'Nur ein Feld in dieser Zeile oder Spalte kann diese Ziffer oder das Schwarz noch aufnehmen — die anderen verlieren die Möglichkeit.',
  wt_rule_black_note:
    'Jede Zeile und jede Spalte hat genau zwei schwarze Felder. Diese Möglichkeiten würden ein drittes erzeugen — also fallen sie weg.',
  wt_rule_backtrack_note:
    'Der Löser hat einen Versuch gestartet, um eine Sackgasse aufzubrechen. Bei von Hand lösbaren Puzzles selten.',

  // Walkthrough rule notes — dominant: gezeigt, wenn diese Regel die Welle anführt.
  wt_rule_target_tuples_dominant:
    'Die meisten Streichungen dieser Welle folgen aus den Zielsummen — manche Ziffern oder schwarzen Felder können in keiner Anordnung vorkommen, die den Zielwert der Zeile oder Spalte ergibt.',
  wt_rule_arc_dominant:
    'Die meisten Streichungen dieser Welle folgen aus dem Möglichkeitsabgleich — keine verbleibende Anordnung der Zeile oder Spalte unterstützt diese Möglichkeiten noch.',
  wt_rule_singleton_dominant:
    'Die meisten Streichungen dieser Welle folgen aus erzwungenen Feldern — ein nahes Feld ist jetzt eindeutig bestimmt, und sein Wert kann im Rest der Zeile oder Spalte nicht erneut auftauchen.',
  wt_rule_hidden_dominant:
    'Die meisten Streichungen dieser Welle folgen aus „einziger Platz" — nur ein Feld in einer Zeile oder Spalte kann eine bestimmte Ziffer oder das Schwarz noch aufnehmen, die anderen verlieren die Möglichkeit.',
  wt_rule_black_dominant:
    'Die meisten Streichungen dieser Welle folgen aus der Zwei-Schwarze-Regel — jede Zeile und jede Spalte hat genau zwei schwarze Felder, also fallen Möglichkeiten weg, die ein drittes erzeugen würden.',
  wt_rule_backtrack_dominant:
    'Die meisten Streichungen dieser Welle folgen aus Hypothesen — der Löser hat einen Versuch gestartet, um eine Sackgasse aufzubrechen. Bei von Hand lösbaren Puzzles selten.',

  // Classification chip
  cls_no_solution: 'Keine Lösung',
  cls_multiple: 'Mehrere Lösungen',
  cls_normal: 'Normal',
  cls_hard: 'Schwer',
  cls_very_hard: 'Sehr schwer',
  cls_extremely_hard: 'Extrem schwer',

  // Toasts
  toast_solved: 'Puzzle gelöst! 🎉',
  toast_check_empty: 'Trage zuerst Felder ein und prüfe dann.',
  toast_check_all_correct: 'Alle eingetragenen Felder sind richtig.',
  toast_one_wrong: 'Ein falsches Feld.',
  toast_n_wrong: '{n} falsche Felder.',
  toast_link_copied: 'Link in die Zwischenablage kopiert',
  toast_share_failed: 'Puzzle konnte nicht geteilt werden',

  // Web Share API
  share_title: 'Doplo-Puzzle',
  share_text: 'Probier dieses Doplo-Puzzle:',

  // Wasm errors
  err_row_targets_length: 'Anzahl Zeilen-Zielwerte passt nicht zur Puzzle-Größe.',
  err_col_targets_length: 'Anzahl Spalten-Zielwerte passt nicht zur Puzzle-Größe.',
  err_targets_length_mismatch: 'Zeilen- und Spalten-Zielwerte müssen gleich viele sein.',
  err_size_range: 'Größe muss zwischen 5 und 8 liegen.',
  err_unsolvable: 'Puzzle ist nicht lösbar.',
  err_multiple_solutions: 'Puzzle hat mehrere Lösungen.',
  err_incomplete_state: 'Löser lieferte einen unvollständigen Zustand.',
  err_target_out_of_range:
    'Zielwert {t} liegt außerhalb des Bereichs (Maximum {max} für Größe {size}).',

  // Locale switcher — autonyms (language's own name); same in every catalog.
  loc_en: 'English',
  loc_de: 'Deutsch',
  loc_pt: 'Português',
  loc_fr: 'Français',
};
