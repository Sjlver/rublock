import type { Messages } from './en';

// French catalog (fr). Typed `: Messages` so any missing key is a compile
// error. Keep placeholders (`{n}`, `{*…*}`, `{err}`, `{others}`) intact.
export const fr: Messages = {
  // Bottom nav
  nav_play: 'Jouer',
  nav_steps: 'Solution',
  nav_create: 'Créer',
  nav_print: 'Imprimer',
  nav_howto: 'Comment jouer',
  nav_aria: 'Sections principales',

  // Page header
  header_share: 'Partager',
  header_share_aria: 'Partager',
  header_locale_aria: 'Langue',

  // Play tab
  play_title: 'Jouer',
  play_new_puzzle: 'Nouveau puzzle',
  play_undo: 'Annuler',
  play_redo: 'Rétablir',
  play_check: 'Vérifier',
  play_check_aria: 'Vérifier les réponses',
  play_undo_aria: 'Annuler',
  play_redo_aria: 'Rétablir',
  play_notes_mode: 'Mode notes',
  play_space_to_switch: 'Espace pour basculer',
  play_hint_chip: 'Appui court pour une note · long pour la réponse',
  play_hint_dismiss_aria: "Fermer l'indice",
  play_status_switching: 'Changement…',
  play_status_generating: 'Génération…',

  // InputBar
  inputbar_aria: 'Boutons de saisie',
  inputbar_black_aria: 'Case noire (maintenir) / note (appui court)',
  inputbar_digit_aria: 'Chiffre {n} (maintenir pour placer, appui court pour note)',
  inputbar_o_aria: 'Doit être un chiffre (note)',
  inputbar_erase_aria: 'Effacer la case',

  // Create tab
  create_title: 'Créer',
  create_set_target_aria: 'Définir la cible à {n}',
  create_values_aria: 'Boutons de valeur cible',
  create_use: 'Utiliser ce puzzle',
  create_use_aria: 'Utiliser ce puzzle',
  create_use_title_valid: "Envoyer ce puzzle à l'onglet Jouer",
  create_use_title_invalid: 'Seuls les puzzles avec une solution unique peuvent être utilisés',
  create_share_invalid: 'Impossible de partager un puzzle invalide',

  // Print tab
  print_title: 'Imprimer',
  print_status: 'Générer un livret imprimable',
  print_size: 'Taille',
  print_pages: 'Pages',
  print_six_per_page: 'Six puzzles par page',
  print_fewer_aria: 'Moins de pages',
  print_more_aria: 'Plus de pages',
  print_busy: 'Génération…',
  print_btn_one: 'Générer {n} page',
  print_btn_other: 'Générer {n} pages',
  print_footnote: 'Les tailles plus grandes et les livrets plus longs prennent un moment à générer.',

  // Print output
  print_promo: 'Puzzles Doplo générés sur {url}',

  // How-to tab
  howto_title: 'Comment jouer',
  howto_intro:
    'Chaque ligne et colonne a une {*cible*} en tête. Place les chiffres et deux cases noires pour que le puzzle soit cohérent :',
  howto_rule1_title: 'Deux cases noires',
  howto_rule1_body: 'Chaque ligne et chaque colonne contient exactement {*deux cases noires*}.',
  howto_rule2_title: 'Une permutation entre elles',
  howto_rule2_body:
    'Les autres cases de chaque ligne et colonne contiennent les chiffres {*1 à N − 2*} — chacun apparaissant une fois.',
  howto_rule3_title: 'Somme égale à la cible',
  howto_rule3_body:
    'Les nombres {*entre*} les deux cases noires doivent additionner la cible affichée. Une cible de {*0*} signifie que les deux cases noires sont adjacentes.',
  howto_example_heading: 'Exemple pas à pas',
  howto_example_intro:
    'Voici un nouveau puzzle 5 × 5. Les chiffres utilisés sont 1, 2 et 3. Par où commencer ?',
  howto_step1_heading: 'Étape 1 — La cible 6 de la ligne est la somme maximale possible',
  howto_step1_body:
    'Chiffres 1 + 2 + 3 = {*6*}. Cible 6 signifie que tous les chiffres se trouvent entre les deux cases noires, donc les cases noires vont aux extrémités : colonne 1 et colonne 5.',
  howto_step2_heading: 'Étape 2 — La cible 0 de la colonne 5 signifie que les cases noires sont voisines',
  howto_step2_body:
    'Cible 0 signifie rien entre les cases noires — elles doivent être adjacentes. La colonne 5 a déjà une case noire en ligne 1, donc la deuxième case noire se place juste en dessous, en ligne 2.',
  howto_step3_heading: 'Étape 3 — La cible 2 de la ligne 2 fixe la deuxième case noire',
  howto_step3_body:
    'La ligne 2 a déjà une case noire en colonne 5. Cible 2 signifie que seul le chiffre {*2*} se trouve entre les cases noires. Place le 2 en colonne 4 et la deuxième case noire en colonne 3.',
  howto_outro:
    "Chaque déduction ouvre la suivante. Continue jusqu'à ce que le puzzle soit complet — il y a toujours exactement une solution.",
  howto_controls_heading: 'Contrôles',
  howto_controls_action: 'Action',
  howto_controls_touch: 'Tactile / souris',
  howto_controls_kb: 'Clavier',
  howto_ctrl_place_note: 'Placer une note',
  howto_ctrl_place_note_touch: 'Appuyer sur le bouton',
  howto_ctrl_place_note_kb: 'Espace, puis chiffre',
  howto_ctrl_place_value: 'Placer une réponse',
  howto_ctrl_place_value_touch: 'Maintenir le bouton',
  howto_ctrl_place_value_kb: 'Chiffre (par défaut)',
  howto_ctrl_toggle_mode: 'Basculer mode note/réponse',
  howto_ctrl_toggle_mode_touch: '—',
  howto_ctrl_toggle_mode_kb: 'Espace',
  howto_ctrl_mark_black: 'Marquer la case en noir',
  howto_ctrl_mark_black_touch: 'Maintenir ■',
  howto_ctrl_mark_black_kb: '0, B ou X',
  howto_ctrl_mark_digits: 'Marquer comme chiffre uniquement',
  howto_ctrl_mark_digits_touch: 'Appuyer sur ○',
  howto_ctrl_mark_digits_kb: '9 ou O',
  howto_ctrl_erase: 'Effacer la case',
  howto_ctrl_erase_touch: 'Appuyer sur la gomme',
  howto_ctrl_erase_kb: 'Retour arrière / Suppr',
  howto_ctrl_move: 'Déplacer la sélection',
  howto_ctrl_move_touch: 'Appuyer sur la case',
  howto_ctrl_move_kb: 'Touches fléchées / WASD',
  howto_source: 'Code source :',

  // Walkthrough tab
  wt_title: 'Solution',
  wt_status_no_puzzle: 'Aucun puzzle chargé.',
  wt_placeholder:
    "Choisis un puzzle dans l'onglet {*Jouer*} ou {*Créer*}. La solution pas à pas apparaîtra ici.",
  wt_error: 'Impossible de générer la solution : {err}',
  wt_intro1:
    'Regarde le solveur progresser sur le puzzle actuel. Chaque grille ci-dessous est une {*vague*} — chaque changement dans une vague découle uniquement de ce qui était connu avant elle.',
  wt_intro2:
    "Les cases commencent avec tous les chiffres (petits nombres) plus un {*x*} pour « pourrait être noire ». Au fil des éliminations, les notes rétrécissent. Quand il ne reste qu'une option, la case est remplie. Les cases qui ont changé dans une vague sont surlignées en jaune.",
  wt_start: 'Début',
  wt_start_sub: "Chaque case peut encore contenir n'importe quel chiffre ou être noire.",
  wt_wave: 'Vague {n}',
  wt_wave_one: '· {n} note supprimée',
  wt_wave_other: '· {n} notes supprimées',
  wt_search: 'Recherche',
  wt_guess_one: '· {n} hypothèse',
  wt_guess_other: '· {n} hypothèses',
  wt_search_sub:
    'La déduction pure ne suffit pas ici — à partir de ce point, le solveur teste des hypothèses et fait marche arrière. Le reste est trop dense pour être montré grille par grille, donc on passe directement à la réponse.',
  wt_solved: 'Résolu',
  wt_solved_sub: 'Le puzzle final.',
  wt_status_waves_one: '{n} vague',
  wt_status_waves_other: '{n} vagues',
  wt_status_removed_one: '{n} note supprimée',
  wt_status_removed_other: '{n} notes supprimées',
  wt_status_join: ' · ',
  wt_extra_rules: 'La vague inclut aussi des déductions de {others}.',

  // Walkthrough rule labels
  wt_rule_target_tuples: 'Sommes cibles',
  wt_rule_arc: 'Vérification des possibilités',
  wt_rule_singleton: 'Cases forcées',
  wt_rule_hidden: 'Seul endroit possible',
  wt_rule_black: 'Règle des deux cases noires',
  wt_rule_backtrack: 'Hypothèse',

  // Walkthrough rule notes
  wt_rule_target_tuples_note:
    "Certains placements de chiffre ou de case noire ne peuvent faire partie d'aucun arrangement dont la somme atteint la cible de la ligne ou colonne — ceux-là sont supprimés.",
  wt_rule_arc_note:
    'Aucun arrangement restant de cette ligne ou colonne ne supporte encore ces options, donc elles sont éliminées.',
  wt_rule_singleton_note:
    'Une case voisine est maintenant entièrement déterminée, et sa valeur ne peut pas se répéter dans le reste de sa ligne ou colonne.',
  wt_rule_hidden_note:
    'Une seule case de cette ligne ou colonne peut encore contenir ce chiffre ou cette case noire, donc les autres le perdent comme candidat.',
  wt_rule_black_note:
    'Chaque ligne et chaque colonne a exactement deux cases noires. Ces options créeraient une troisième — elles sont donc supprimées.',
  wt_rule_backtrack_note:
    "Le solveur a tenté une hypothèse pour sortir d'une impasse. Rare pour les puzzles solubles à la main.",

  // Classification chip
  cls_no_solution: 'Aucune solution',
  cls_multiple: 'Plusieurs solutions',
  cls_normal: 'Normal',
  cls_hard: 'Difficile',
  cls_very_hard: 'Très difficile',
  cls_extremely_hard: 'Extrêmement difficile',

  // Toasts
  toast_solved: 'Puzzle résolu ! 🎉',
  toast_check_empty: 'Remplis quelques cases, puis vérifie-les.',
  toast_check_all_correct: 'Toutes les cases remplies sont correctes.',
  toast_one_wrong: 'Une case incorrecte.',
  toast_n_wrong: '{n} cases incorrectes.',
  toast_link_copied: 'Lien copié dans le presse-papiers',
  toast_share_failed: 'Impossible de partager ce puzzle',

  // Web Share API
  share_title: 'Puzzle Doplo',
  share_text: 'Essaie ce puzzle Doplo :',

  // Wasm errors
  err_row_targets_length: 'Le nombre de cibles de ligne ne correspond pas à la taille du puzzle.',
  err_col_targets_length:
    'Le nombre de cibles de colonne ne correspond pas à la taille du puzzle.',
  err_targets_length_mismatch:
    'Les cibles de ligne et de colonne doivent avoir le même nombre.',
  err_size_range: 'La taille doit être comprise entre 5 et 8.',
  err_unsolvable: "Le puzzle n'a pas de solution.",
  err_multiple_solutions: 'Le puzzle a plusieurs solutions.',
  err_incomplete_state: 'Le solveur a renvoyé un état incomplet.',
  err_target_out_of_range:
    'La cible {t} est hors de portée (maximum {max} pour la taille {size}).',

  // Locale switcher — autonyms (language's own name); same in every catalog.
  loc_en: 'English',
  loc_de: 'Deutsch',
  loc_pt: 'Português',
  loc_fr: 'Français',
};
