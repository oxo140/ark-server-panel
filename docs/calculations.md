# Calculs des réglages (v1.8.0)

Depuis la v2.0.0, l’interface expose uniquement `MaxWildDinoLevel`, un réglage virtuel du panneau (entier de 1 à 300). Il n’est jamais écrit comme clé dans un INI. Sa sauvegarde écrit `DifficultyOffset=1`, `OverrideOfficialDifficulty=niveau/30` et `MaxDifficulty=False` dans sa section native. Les anciennes clés restent acceptées à l’import et dans l’éditeur texte. Le chargement, les calculs et les changements de langue ne normalisent pas les fichiers existants ; seule une modification enregistrée du niveau le fait.

Tous les calculs sont exécutés en Rust par `simulation::preview`. L’interface envoie le brouillon après une courte temporisation. Cette route ne sauvegarde aucun fichier et reste utilisable pendant que le serveur tourne. Les valeurs sont enregistrées uniquement avec le bouton d’enregistrement, serveur arrêté.

## Difficulté

Sans surcharge : difficulté = `DifficultyOffset × (difficulté de la carte − 0,5) + 0,5`. Base 4 pour The Island, Scorched Earth et Aberration ; base 5 pour les autres cartes intégrées. Une carte personnalisée n’a pas de base supposée. `OverrideOfficialDifficulty > 0` remplace le calcul par carte. Plafond ordinaire arrondi : `30 × difficulté`, Tek `36 × difficulté`, Wyvernes/Rock Drakes `38 × difficulté`, uniquement là où ces créatures existent. Grottes, missions, boss et mods sont exclus. `MaxDifficulty` est lu/écrit dans `[ShooterGameMode_Options]` et déclenche un avertissement de priorité et d’exception The Island. Les valeurs ne réaffectent pas les dinos existants.

Source : [difficulté, wiki communautaire ARK](https://ark.wiki.gg/wiki/Difficulty).

## Rex normal : références et hypothèses

Les constantes extraites par [ARK Smart Breeding](https://github.com/cadon/ARKStatsExtractor/tree/master/ARKBreedingStats/json) donnent 17 998,5601 s d’incubation, 333 333,333 s de maturation et 64 800–172 800 s de délai de reproduction. [La lecture du DevKit ASA publiée par Hypernatrema](https://hypernatrema.com/ark/mechanics/breeding-and-mutations) donne un accouplement de 120 s et décrit la formule d’empreinte utilisée par ASA. Ces valeurs restent des références vanilla, hors bonus de mods, traits, événements ou effets de monde. Le niveau 150 n’agit pas sur ces temps.

Maturation et incubation divisent la durée de base par leur vitesse. Le délai de reproduction multiplie l’intervalle de base ; l’accouplement divise 120 s par sa vitesse.

L’intervalle des soins est `28 800 × BabyCuddleIntervalMultiplier`. Pour le Rex, gain par soin = `BabyImprintAmountMultiplier / max(1, floor(maturation / intervalle))`, plafonné à 100 %. Le nombre de soins nécessaires est arrondi vers le haut. Un soin à l’instant où le dino devient adulte n’est pas compté. L’alerte signale si les soins nécessaires ne tiennent pas avant l’âge adulte, et affiche la marge totale pour les retards. Il s’agit d’une possibilité théorique avec des soins immédiats, pas d’une garantie universelle pour les autres espèces. Le wiki présente également une formule avec `−0,25` qui diffère de la lecture ASA ; l’outil utilise ici le modèle ASA avec plancher.

Apprivoisement : exemple Rex sauvage 150, nourriture disponible, pas de dégâts après KO. Affinité requise `3450 + 150 × 150`, gain par croquette exceptionnelle 400 et viande crue 50, rebasing permanent des taux ×4, puis `TamingSpeedMultiplier`. Quantités arrondies vers le haut. Drain inconscient `0,002314 × 180,0634 × DinoCharacterFoodDrainMultiplier`. Valeur de nourriture : croquette exceptionnelle 53,28 ; viande crue 50. Cela donne environ 17 croquettes / 36 min ou 130 viandes / 4 h 20 à ×1. L’outil estime la durée par repas, sans promettre de quantité de narcotiques ni tenir compte du pourrissement. Voir [Rex et références extraites du DevKit](https://hypernatrema.com/ark/browse/creatures/rex) et [Rex sur le wiki](https://ark.wiki.gg/wiki/Rex).

Les bonus solo natifs sont appliqués lorsqu’activés : apprivoisement ×2,5 ; délai d’accouplement ×0,125 ; incubation ×10 ; maturation ×36,799 ; intervalle des soins ×0,167. Référence : [multiplicateurs d’ARK Smart Breeding](https://github.com/cadon/ARKStatsExtractor/blob/master/ARKBreedingStats/json/serverMultipliers.json). Ils restent désactivés dans les profils solo/duo du panneau.

## Récolte et statistiques

Le modèle de rocher est volontairement simplifié : l’admin fournit une mesure de référence avec le même outil à ×1. Les valeurs initiales 10 coups et 100 ressources sont **fictives**. Coups ≈ `ceil(coups de référence × HarvestHealthMultiplier / PlayerHarvestingDamageMultiplier)` ; rendement total ≈ `rendement de référence × HarvestHealthMultiplier × HarvestAmountMultiplier`. Les types d’outils, dégâts, aléatoire et sur-récolte empêchent une prédiction universelle pour tous les rochers.

Le gain d’endurance joueur prend +10 par point comme base. Celui du Rex prend 10 % de son endurance après apprivoisement ; l’admin peut saisir cette référence, initialement 420. Le total d’un Rex 150 dépend de la répartition des points sauvages : aucun total universel n’est présenté. Les multiplicateurs de santé et mêlée apprivoisées conservent leurs bases officielles (0,2 / 0,17), et Add/Affinity 0,14 / 0,44, au lieu d’un faux défaut de 1.

Pour ASA, la vitesse des volants nécessite `bAllowFlyerSpeedLeveling=True` **et** `bAllowSpeedLeveling=True` dans la section de mode de jeu de `Game.ini`. Le bouton prépare les deux sans les enregistrer. Les paramètres natifs et les éventuelles restrictions des espèces/mods restent applicables. Sources : [configuration ARK](https://ark.wiki.gg/wiki/Server_configuration), [retours ASA sur Reddit](https://www.reddit.com/r/ArkSurvivalAscended/comments/1lfcf1g/).
