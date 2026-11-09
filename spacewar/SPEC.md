# SPEC de rendu — spacewar (macroquad, 2 joueurs)

Fenêtre redimensionnable ; le jeu est dessiné dans un **repère logique
1024×768** (ratio 4:3, hommage au scope du PDP-1) mis à l'échelle
uniformément sur la fenêtre réelle (centré, bandes noires) par une
`Camera2D` recalculée à chaque frame — correct sur écran HiDPI.
Fond noir pur. Tout le rendu est **vectoriel** : traits fins
(épaisseur ≤ 2 px) brillants, aucun sprite, aucune image.
Police : **Noto Sans Mono** TTF embarquée (`assets/`, licence copiée à
côté), utilisée pour tout le texte et les mesures via
`draw_text_ex`/`measure_text`. Palette :

- Blanc pur `#FFFFFF` : vaisseaux, torpilles, texte
- Blanc atténué `#AAAAAA` : étoile centrale, aide, HUD secondaire
- Blanc chaud `#FFE0B0` : flammes de poussée, explosions
- Accent discret `#80C0FF` : trainées d'hyperspace, torpilles actives

## 1. Menu (état initial)

- Titre `SPACEWAR!` centré horizontalement, à ~25 % de la hauteur,
  police ~48 px, blanc.
- En dessous (centre horizontal), lignes d'aide ~20 px :
  - `Joueur 1 : A/D tourner · W poussée · S tir · Q hyperspace`
  - `Joueur 2 : ←/→ tourner · ↑ poussée · ↓ tir · M hyperspace`
  - `Premier à 5 victoires`
- `[ESPACE] commencer` clignotant (~2 Hz, visible/absent par périodes égales)
  centré horizontalement, à ~70 % de la hauteur.
- `[1] mode solo (test : J2 inerte)` — J2 gelé : pas d'input, pas de
  gravité, pas de collision torpille (outil de test, hors match officiel).
- Sélecteurs de couleur : `[C] couleur J1 : <nom>` et
  `[V] couleur J2 : <nom>`. Le nom de la couleur est affiché **dans sa
  propre couleur**. Palette : Blanc + arc-en-ciel (Rouge, Orange, Jaune,
  Vert, Cyan, Bleu, Magenta). Une pression sur la touche passe à la
  couleur suivante (cyclique). Les deux vaisseaux peuvent avoir la même
  couleur. Le trait du vaisseau ET ses torpilles prennent la couleur
  choisie (les torpilles restent identifiables à leur propriétaire).
- `[X] remanence : <intensité>` — effet de phosphore : le monde est rendu
  dans une texture où l'image précédente est re-dessinée avec un alpha de
  décroissance (la traînée garde la couleur des objets qui l'ont tracée).
  Quatre intensités, cycliques avec X : **Off** (0), **Faible** (0,35),
  **Moyenne** (0,65), **Forte** (0,85) — valeur affichée en accent.
  Ne s'applique qu'au monde (vaisseaux, torpilles, particules) ; l'UI
  (menu, HUD, textes) reste nette au-dessus.
- `ÉCHAP : quitter` en petit, bas de fenêtre.

## 2. Terrain (en jeu)

- **Étoile centrale** : cercle plein atténué r=20 px au centre exact
  (512, 384), entouré d'un cercle de « danger » en pointillés r=60 px.
  Elle est **fixe** et **tue** tout vaisseau ou torpille qui entre dans
  r=26 px (rayon dessiné + 6 px de marge ≈ rayon du vaisseau).
- **Gravité** : accélération constante dirigée vers l'étoile, appliquée aux
  deux vaisseaux et aux torpilles. Pas de frottement : l'inertie conservée.
- **Bords enveloppants** : sortir par un bord = réapparaître de l'autre côté
  (vaisseaux ET torpilles, position ET dessin se raccordent proprement).
  Les copies de raccord ne sont dessinées que pour les objets à moins de
  50 px du bord appelant : **rien n'apparaît dans les bandes noires**.
- **Vaisseaux** : deux silhouettes vectorielles distinctes, ~24 px de long,
  dessinées en traits blancs, orientées selon leur cap :
  - Joueur 1 (part gauche de l'écran, cap initial → droite) : **aiguille**,
    silhouette effilée.
  - Joueur 2 (part droite, cap initial ← gauche) : **soucoupe/fusée**,
    silhouette plus large.
  Un **compte à rebours 3·2·1** (~1 s par chiffre, chiffre géant au centre)
  précède le premier contrôle ; les vaisseaux sont figés pendant le décompte.
- **Poussée** : flamme vectorielle vacillante (blanc chaud) sortant de
  l'arrière, uniquement pendant l'appui. Le carburant (HUD) décroît pendant
  la poussée et se régénère lentement hors poussée ; réservoir vide = plus
  de poussée jusqu'à régénération partielle.
- **Torpilles** : petits segments lumineux (accent), vitesse initiale =
  vitesse du vaisseau + vitesse de tir vers l'avant. Max **4 en vol par
  vaisseau** ; durée de vie limitée (~3 s) ; détruites par l'étoile.
  Une torpille qui touche un vaisseau le détruit ; pas de tir ami
  (la torpille ignore son propre vaisseau pendant 0,2 s après le tir).
  Rechargement : 0,25 s entre deux tirs. La torpille prend la couleur
  de son propriétaire.
- **Hyperspace** : le vaisseau disparaît (petite implosion dessinée), puis
  réapparaît à une position aléatoire (jamais dans la zone de danger) ;
  risque **1/8** d'explosion à l'arrivée ; vitesse remise à zéro à
  l'arrivée (choix de gameplay documenté). Coût : 1 s d'indisponibilité.
- **Explosions** : gerbe de particules blanches/chaudes s'écartant en
  ligne droite (physique vectorielle, pas d'effets ronds), fade-out ~1 s.
- **HUD** (haut, centré horizontal, une seule ligne, 22 px) :
  `J1 ◇ n` · icône étoile **dessinée au trait** (⏣ n'existe dans aucune
  police courante) · `m ◇ J2`, où n/m = victoires. Sous chaque bloc score,
  barre de carburant fine (100 px), centrée. Symétrie autour du centre.

## 3. Fin de manche / de match

- À la destruction d'un vaisseau : manche terminée, message centré
  `Le joueur N marque !` (ou `Étoile ! Personne ne marque` si double mort ou
  mort par l'étoile des deux), score à jour, puis retour au **décompte
  3·2·1** après ~2 s.
- Premier à **5** : écran de fin centré :
  - `JOUEUR N GAGNE LA PARTIE !` (~40 px)
  - `score final : n - m`
  - `[ESPACE] rejouer · [ÉCHAP] quitter` (Échap quitte directement)

## 4. Contrôles (récapitulatif normatif)

| Action | Joueur 1 | Joueur 2 |
|---|---|---|
| Tourner gauche | A | flèche gauche |
| Tourner droite | D | flèche droite |
| Poussée | W | flèche haut |
| Tir | S | flèche bas |
| Hyperspace | Q | M |
| Commencer / rejouer | Espace (les deux) | |
| Quitter | Échap | |

Vitesse de rotation ~3,5 rad/s ; poussée ~300 px/s² ; gravité ~60 px/s²
vers le centre ; vitesse max torpille ~600 px/s. Ces valeurs sont la
référence du tuning : tout écart doit être justifié.

## 5. Checklist d'audit (agent indépendant)

1. `cargo build --release` propre, binaire `spacewar` installé dans
   `~/.local/bin`, md5 build == installé.
2. Lancer le jeu, vérifier menu (centrage titre/aide, clignotement),
   décompte, terrain (étoile centrée, HUD symétrique), fin de manche,
   écran de victoire — contre les sections 1–3 ci-dessus.
3. Vérifier la physique : inertie (le vaisseau glisse après relâchement),
   gravité visible (orbite possible), enveloppement des bords, limite de
   4 torpilles, carburant qui décroît/régénère.
4. Signaler tout écart avec la mesure ou le critère qui le prouve.
