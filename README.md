# ♡ Jeux kawaii

Collection de petits jeux de Cyril, en terminal d'abord, en graphique ensuite.
Une seule stack de dev : **Rust**.

## Stack

| Domaine | Crate |
|---|---|
| Terminal (TUI) | [ratatui](https://ratatui.rs) + crossterm |
| Graphique (2D) | macroquad (à venir) |
| Commun | `jeux-core` (dictionnaire, Wiktionnaire, thème) |

Pourquoi Rust : binaire autonome léger (~2,5 Mo), pas de runtime, le
compilateur guide (vibe coding friendly), et une seule mentalité de code
pour tout.

## Structure

```
~/Work/jeux/
├── Cargo.toml          # workspace Cargo
├── jeux-core/          # briques partagées
│   └── src/lib.rs      #   score, dico, wiki, theme
├── motus/              # Motus (TUI, ratatui)
├── fubuki/             # Fubuki (TUI, ratatui)
├── takuzu/             # Takuzu (TUI, ratatui)
├── motus-rust/         # ancien motus standalone (→ remplacé par motus/)
├── motus-go/           # variante Go (référence comparaison)
├── motus-kawaii/       # variante Ruby (référence gameplay)
├── fubuki-kawaii/      # Go (existant, à porter)
├── takuzu-kawaii/      # Go (existant, à porter)
└── space-invaders-kawaii/  # Ruby (existant, à porter)
```

## jeux-core

Les briques que tous les jeux partagent, pour ne plus rien réinventer :

- **`score()`** — feedback Motus/Wordle (Good/Present/Absent + positions
  verrouillées)
- **`dico`** — dictionnaire français embarqué dans le binaire (`include_str!`),
  normalisation des accents (`être` → `etre`, clavier US friendly), secrets
  restreints aux noms + infinitifs + participes
- **`wiki`** — définitions Wiktionnaire (API wikitext, nettoyage du markup,
  via `curl`)
- **`theme`** — palette adaptative clair/sombre (rose kawaii `#D6488C`/`#FF9EC4`)

## Jeux

### motus (TUI)

5 à 9 lettres, 6 essais, première lettre donnée et verrouillée, lettres bien
placées reportées, définition Wiktionnaire à la fin.

```bash
cargo run -p motus --release
```

### À venir

### fubuki (TUI)

Grille 3×3, chiffres 1-9, sommes de lignes/colonnes vérifiées en continu
(= verte si correcte, rouge si fausse), palette des chiffres restants,
3 niveaux (2/1/0 indices).

```bash
cargo run -p fubuki --release
```

### takuzu (TUI)

Binairo : grilles 6×6 à 10×10, pas de triple, équilibre 0/1, lignes et
colonnes uniques. Génération à solution unique (backtracking + propagation),
gouttière ✓/✗ par ligne et colonne, 3 niveaux.

```bash
cargo run -p takuzu --release
```

### À venir

- space invaders (portage depuis Ruby)
- premier jeu graphique (macroquad)

## Conventions

- Palette adaptative clair/sombre sur tous les jeux (`theme::actif()`)
- UI en français, pluriels corrects (« 1 indice », « 2 indices »)
- Kawaii mais doux : pas d'aplats de fond agressifs, les couleurs vont sur
  les lettres/caractères, pas le fond
- Chaque jeu est un membre du workspace ; les binaires vont dans
  `~/.local/bin/` après build : `cp target/release/<jeu> ~/.local/bin/`
- Tests : `cargo test --workspace` — la logique de jeu vit dans `jeux-core`
  ou en fonctions pures testables, le rendu reste fin
- Dictionnaire : `data/mots.txt` (toutes formes) et `data/secrets.txt`
  (noms, infinitifs, participes, peu fréquents), normalisés sans accents

## Historique

La collection a grandi par portages : Ruby + Charm Ruby (motus, invaders),
Go + Bubble Tea (fubuki, takuzu, motus), puis Rust + Ratatui (motus, fubuki,
takuzu). Les variantes Ruby/Go restent des dépôts indépendants comme
références — la stack officielle est maintenant Rust.

[♪](https://github.com/Gatshub)
