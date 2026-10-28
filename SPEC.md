# SPEC de rendu — collection jeux kawaii (TUI Rust)

Référence d'audit : chaque écran doit être vérifiable mécaniquement contre
cette spec. L'outil `check_center.py <session> <W> <H>` (émulation de grille
terminal : parsing SGR caractère par caractère, largeurs East-Asian, boîte
englobante du contenu visible) produit les mesures à comparer.

## Terminologie

- « centré horizontalement » : centre X de la boîte englobante = W/2 ± 1 col
- « centré verticalement » : centre Y de la boîte englobante = H/2 ± 1 ligne
- « bloc centré » : les deux
- Référence de test : pane 110×40 (centre 55, 20)

## SPEC commune (tous les jeux TUI)

1. **Menu** : bloc centré (H et V). Ordre : titre ♡ stylé (accent) →
   « Choisis ton niveau : » (ou « ta longueur ») → une ligne par niveau
   (préfixe `[n]`, niveau sélectionné en accent + gras, autres en teinte
   douce) → ligne d'aide faint. Aucun autre contenu.
2. **En jeu** : bloc centré (H et V). Ordre : titre → HUD (score/niveau/vies
   ou essai n/N) → terrain/grille → message → aide. Rien hors du bloc.
3. **Écran de fin** : identique au jeu avec victoire (🐾) ou défaite, message
   de fin, `[n] rejouer · esc menu · q quitter`.
4. Aucune ligne du contenu ne doit être tronquée par les bords du pane.
5. Pas de fond coloré ; les couleurs portent sur les caractères.

## SPEC par jeu

### motus
- Grille dessinée `┌─┬─┐` : cellules de 5 cols, `MAX_TRIES+1` bordures
  verticales ; lettres bien placées = **BLEU**, mal placées = **ROUGE**,
  absentes = gris, lettre donnée = teinte douce, curseur = accent + gras
  (`_` sur case vide sous le curseur).
- La définition Wiktionnaire est **wrappée par mots** à la largeur utile,
  max 4 lignes.

### fubuki
- Grille 3×3 + sommes `= nnn` à droite des lignes et sous les colonnes ;
  somme verte (correcte) / rouge (fausse) / grise (incomplète).
- Palette 1-9 : chiffres posés en teinte douce.

### takuzu
- Grille n×n en `○`/`●`/`·`, gouttière gauche + basse avec `✓`/`✗`/`·`
  (✗ rouge si doublon), bordures reliées.

### invaders (TUI)
- Terrain braille 44 cols × 15 lignes (88×60 points) : envahisseurs accent,
  vaisseau + balles ok, bombes ko, bunkers teinte douce.
- HUD : `score : n   niveau : x   vies : ♡ ♡`.
- Menu : ligne `[b] bunkers : on/off`.

### invaders-gfx (macroquad)
- Fenêtre fixe 960×660, fond sombre par défaut, 't' bascule le thème.
- Menu : titre + 3 niveaux (sélection encadrée + ▸, cliquable) + `[b] bunkers`
  + aide. En jeu : HUD haut gauche (score), haut droite (vies), terrain
  plein cadre, message en bas, aide au-dessus.
- Crabes animés 2 frames (pattes), explosion en particules à chaque kill,
  screen-shake à l'impact, flash rouge quand le vaisseau est touché.
- Les bombes touchent le vaisseau AVANT d'être rejetées hors champ.

## Checklist d'audit (à exécuter par l'agent auditeur)

1. Compiler `cargo build --release` et installer les binaires dans
   `~/.local/bin` (vérifier md5 build == installé).
2. Pour chaque jeu, chaque écran (menu, jeu, fin) : lancer dans tmux 110×40,
   appliquer check_center, comparer aux tolérances ±1.
3. Vérifier l'ordre des éléments contre la spec par jeu.
4. Signaler tout écart avec la mesure qui le prouve.

## Audit du 2026-10-27 (agent auditeur indépendant)

10 écrans vérifiés (menu + jeu ×4 jeux, fins motus/invaders) : 9 OK
immédiatement ; dernier écart (motus jeu, -2 lignes) corrigé dans la foulée.
Tous les écrans sont dans la tolérance ±1. Les binaires installés sont
vérifiés par md5 avant chaque audit.

## Recettage graphique (invaders-gfx) — critères géométriques

La fenêtre fait 960×660 (compensée du scale HiDPI). En plus du gameplay,
vérifier à l'ouverture et en jeu :

1. **Fenêtre** : taille 960×660 (`hyprctl clients`), flottante, centrée
   (`at ≈ (écran−960)/2, (écran−660)/2`), jamais en 0,0.
2. **Occupation** : le terrain occupe ≥ 85 % de la surface — la formation
   d'envahisseurs s'étale du bord gauche au bord droit avec des marges
   latérales **symétriques** (écart ≤ 10 px) ; les bunkers et le vaisseau
   sont répartis sur toute la largeur.
3. **Marges verticales** : rien ne chevauche le titre ni la ligne de
   message ; un espace ≥ 20 px sépare HUD / formation / bunkers / vaisseau.
4. **Marges latérales du jeu** : la formation, en se déplaçant, rebondit
   avant les bords (jamais rognée) ; le vaisseau ne sort jamais du cadre.
5. **Proportions** : sprites ~2× plus grands que la version TUI (lisible
   à 960 px) ; le pas de descente de la formation ne la fait jamais
   chevaucher les bunkers d'un seul pas.
6. **Hiérarchie visuelle** : le titre est le plus gros texte (46 px),
   HUD 22 px, messages 24 px, aide ≤ 20 px mais lisible (contrasté, pas
   dim sur fond sombre).
7. **Tofu** : aucun carré de remplacement de glyphe à l'écran (toutes les
   chaînes draw_text sont ASCII/latin-1 de base).

Chaque point doit pouvoir être validé par mesure (hyprctl clients, calcul
des marges depuis les constantes du code) ou par capture d'écran lue.
