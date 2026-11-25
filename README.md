# ARK Server Panel

### Version 1.8.0

- Disponibilité sans lancer le jeu : contrôle du processus ARK, du port UDP appartenant à ce processus, puis de l’annonce de disponibilité dans le journal ou d’une réponse RCON. RCON n’est plus obligatoire pour afficher un serveur prêt. Un bouton **Tester le serveur** actualise le diagnostic. La joignabilité depuis un autre ordinateur reste distincte du contrôle local.
- Détection native Windows de l’adaptateur Tailscale, y compris lorsqu’il manque dans les compteurs réseau. L’adresse `100.x.x.x` est choisie par défaut avec le port de jeu du profil ; une adresse saisie explicitement reste conservée.
- 91 réglages guidés, dont les gains par point du joueur et des dinos, les bonus d’apprivoisement et les cases de vitesse joueur/volants. Un bouton prépare les deux autorisations nécessaires à ASA.
- Exemples dynamiques calculés en Rust : plafond sauvage selon carte/difficulté, Rex 150 pour l’apprivoisement, temps de reproduction/incubation/maturation, soins et alerte pour atteindre 100 % d’empreinte. Exemple de récolte avec référence personnalisable. Les calculs restent consultatifs et ne sauvegardent pas les brouillons.

Les formules, sources et limites sont décrites dans [docs/calculations.md](docs/calculations.md).

### Correctif 1.7.2

Lost Colony requiert désormais une marge minimale de 16 Gio de mémoire virtuelle disponible au lancement, contre 8 Gio pour les autres cartes. Ce contrôle couvre mieux le pic de chargement observé ; il ne garantit pas la capacité de toutes les configurations et mods.

Après téléchargement et vérification d’une mise à jour du panneau, le superviseur redémarre automatiquement le panneau. Les processus ARK restent actifs ; il n’est plus nécessaire de cliquer une seconde fois sur Redémarrer. En cas d’échec de téléchargement ou de vérification, aucun redémarrage n’est déclenché. Une mise à jour du panneau ne redémarre pas Windows.

Lorsqu’un processus ARK lancé par le panneau s’arrête, son code de sortie et l’heure sont ajoutés aux diagnostics. Le panneau n’envoie aucun arrêt lorsque le serveur devient prêt. Si Windows épuise sa mémoire virtuelle, un changement de code ne peut pas maintenir le jeu en fonctionnement : vérifiez le fichier d’échange Windows.

### Correctif 1.7.1

La collecte des performances évite désormais les informations complètes de tous les processus : seuls les processus ARK sont mesurés en détail, et les contrôles d’identité ciblent le PID concerné. Le nombre de threads du panneau est limité. Avant un lancement, Windows doit disposer d’au moins 8 Gio de mémoire virtuelle disponible (marge minimale ; cartes et mods peuvent demander davantage). Sinon, le panneau affiche une erreur au lieu de lancer un serveur supplémentaire. Un épuisement global de la mémoire virtuelle Windows peut toujours fermer des applications : libérez de la mémoire ou augmentez le fichier d’échange Windows si nécessaire.

Le bouton **Stats du serveur** ouvre directement **Configuration INI**. Les statistiques restent accessibles dans leur onglet.

Panneau web indépendant pour gérer des serveurs **ARK: Survival Ascended sur Windows**. Le service, les opérations système, RCON, les sauvegardes et les mises à jour sont écrits en **Rust**. L’interface HTML/CSS/JavaScript est intégrée au binaire : aucun serveur Node.js, Python ou outil de compilation n’est nécessaire sur une installation utilisateur.

## Installer la V1

1. Télécharger `ark-server-panel-install.zip` dans les [publications GitHub](https://github.com/oxo140/ark-server-panel/releases).
2. Extraire l’archive dans un dossier conservé sur la machine Windows qui héberge ARK.
3. Ouvrir **Ouvrir ARK Panel.vbs**. Au premier lancement, le script récupère la dernière publication Windows x64 et contrôle son empreinte SHA-256. Le programme ouvre ensuite le navigateur sur `http://127.0.0.1:3080`.
4. Dans **Paramètres & mises à jour**, utiliser **Vérifier SteamCMD** pour une installation existante ou **Télécharger SteamCMD** pour installer l’outil officiel Valve. Le bouton **Installer / mettre à jour** d’un serveur effectue cette vérification lui aussi et propose le téléchargement si nécessaire.
5. Ajouter un profil : un dossier distinct par serveur, une carte et des ports distincts. Cliquer sur **Installer / mettre à jour**, suivre les journaux, puis démarrer le serveur lorsque SteamCMD a terminé avec succès.

Pour reprendre un serveur existant, créer un profil pointant vers son dossier racine contenant `ShooterGame`. La création du profil ne modifie pas ses fichiers. Ne pas cliquer sur l’installation si vous souhaitez uniquement l’importer.

L’exécutable Rust travaille en arrière-plan : l’administration se fait entièrement dans le navigateur. Fermer l’onglet n’arrête pas les serveurs. Le démarrage automatique avec Windows et l’installation comme service Windows ne sont pas inclus dans cette V1.

## Fonctions présentes

- Profils multiples avec contrôle des conflits de ports et des dossiers partagés.
- Lancement d’ARK, détection des processus correspondant au chemin du serveur, récupération de leur identité après redémarrage du panneau.
- Installation et validation via SteamCMD, état de l’opération et consultation des journaux.
- Console RCON et demande d’arrêt avec `SaveWorld`, puis `DoExit`. Aucun arrêt forcé automatique.
- Édition brute de `Game.ini` et `GameUserSettings.ini` en UTF-8, conservation des clés répétées et inconnues, détection des modifications concurrentes, copie préalable et écriture par remplacement atomique.
- Éditeur guidé de 33 réglages : curseurs, valeurs numériques précises et interrupteurs, regroupés par thème. Seuls les champs modifiés sont écrits ; une erreur sur le second fichier entraîne la restauration du premier.
- Préréglages **Solo PvE confort** et **Duo PvE équilibré**, préparés en brouillon et ajustables avant enregistrement.
- Noms lisibles pour les dix cartes ASA disponibles au 23 novembre 2026, avec option de carte personnalisée et conservation de l’identifiant technique au lancement.
- Sauvegarde du dossier `ShooterGame/Saved` complet, uniquement serveur arrêté, avec contrôle SHA-256 de chaque fichier. Les sauvegardes sont des dossiers, sans compression dans la V1.
- Restauration avec contrôle d’intégrité et conservation de l’ancien dossier Saved sous un autre nom.
- Vérification des versions GitHub au démarrage et toutes les douze heures ; téléchargement et activation manuels depuis le panneau.

## RCON et configuration

### Réglages portables et planification

Dans **Stats du serveur → Configuration INI**, **Exporter mes réglages** télécharge un profil JSON des paramètres guidés enregistrés. **Importer un profil JSON** applique ce profil à un serveur arrêté, avec copie préalable et contrôle des modifications concurrentes. Les ports, adresses, mots de passe et clés INI inconnues ne sont pas transférés par ce profil.

**Lire un fichier INI / texte** accepte un fichier UTF-8 ou UTF-16 avec BOM et l’ouvre en brouillon. Vérifier le fichier cible (`Game.ini` ou `GameUserSettings.ini`), puis enregistrer serveur arrêté. En revenant aux réglages guidés, les curseurs reflètent les valeurs reconnues. Les clés inconnues restent accessibles dans l’éditeur texte.

Dans **Planification**, chaque serveur dispose d’options désactivées par défaut : sauvegarde toutes les N heures (2 par défaut), redémarrage quotidien et mise à jour ASA à une heure locale Windows. Le panneau doit continuer à fonctionner dans la zone de notification. Les tâches quotidiennes manquées sont exécutées au retour du panneau ; le premier intervalle de sauvegarde commence à l’activation. Les dernières exécutions persistent après redémarrage du panneau.

Pour un serveur actif, la maintenance diffuse le message RCON chaque minute pendant le préavis (`{minutes}` est remplacé), envoie `SaveWorld` puis `DoExit`, attend la fin du processus, effectue la copie vérifiée et relance le serveur. Une mise à jour vérifie d’abord le build public ; elle interrompt le serveur uniquement si nécessaire et conserve une sauvegarde avant installation. Un arrêt non confirmé annule l’opération ; une mise à jour échouée conserve le serveur arrêté. Les sauvegardes restent sur disque sans purge automatique.

Les pastilles testent RCON toutes les quinze secondes : vert si `ListPlayers` répond, orange pendant les quinze premières minutes de démarrage ou la maintenance, bleu pendant SteamCMD et rouge si arrêté ou sans réponse après ce délai. Le test RCON est local ; la connexion effective depuis un PC joueur dépend aussi de son accès réseau. L’onglet Stats affiche le PID, la mémoire, la durée et la réponse RCON du serveur.

Dans `ShooterGame/Saved/Config/WindowsServer/GameUserSettings.ini`, définir :

```ini
[ServerSettings]
RCONEnabled=True
RCONPort=27020
ServerAdminPassword=REMPLACER_PAR_UN_MOT_DE_PASSE
```

Utiliser le même port RCON que dans le profil. Le mot de passe est lu dans le fichier de configuration du serveur au moment de la commande ; le panneau ne le copie pas dans ses profils ni dans Git. L’éditeur INI montre le contenu complet aux administrateurs disposant de la clé d’accès.

Les cartes proposées sont The Island, Scorched Earth, The Center, Aberration, Extinction, Ragnarok, Valguero, Astraeos, Lost Colony et Genesis: Part 1. Les cartes non encore sorties sur ASA ne sont pas présentées comme disponibles. Pour une carte modée, choisir **Carte personnalisée / mod** et saisir l’identifiant fourni par son auteur. Le choix des identifiants de mods reste à la charge de l’administrateur ; le panneau ne fournit pas de catalogue CurseForge.

## Réglages guidés et préréglages PvE

Dans **Gérer → Configuration INI**, l’éditeur guidé charge automatiquement les valeurs des deux fichiers. Les champs absents affichent une base proposée ; ils ne sont pas ajoutés au fichier tant qu’ils ne sont pas modifiés. Le bouton **Édition texte INI** conserve l’accès aux options avancées et aux paramètres de mods. Les changements des deux modes sont relus lors du passage de l’un à l’autre.

| Base proposée | XP | Récolte | Apprivoisement | Éclosion / croissance |
| --- | --- | --- | --- | --- |
| Solo PvE confort | ×2 | ×3 | ×5 | ×30 |
| Duo PvE équilibré | ×1,5 | ×2 | ×3 | ×15 |

Les deux bases activent PvE, fixent la difficulté maximale à 5 et désactivent `bUseSingleplayerSettings` pour éviter l’empilement de bonus natifs. Elles ajustent aussi les intervalles de reproduction et de soins, la faim, la soif et le poids gagné par niveau. Les dégâts et résistances existants sont conservés. L’imprégnation reste dépendante de l’espèce et des temps de maturation : aucun préréglage ne garantit automatiquement 100 % pour tous les dinos.

Ces profils sont des propositions éditoriales inspirées de retours de joueurs, et non un consensus ni des réglages officiels. Les tendances retenues sont une accélération modérée de la récolte/progression et une réduction plus forte des temps d’attente. Sources : [discussion PvE solo et amis](https://www.reddit.com/r/ARK/comments/os2jzx/), [réglages ASA en petit groupe](https://www.reddit.com/r/ARK/comments/1quthy9/help_us_set_up_our_ark_survival_ascended_server/), [documentation des paramètres](https://ark.wiki.gg/wiki/Server_configuration).

## Installation SteamCMD

Le panneau vérifie d’abord le chemin enregistré, son propre dossier d’outils, `C:\SteamCMD`, un dossier voisin du programme et les emplacements du `PATH`. Un fichier doit être un exécutable Windows PE nommé `steamcmd.exe` ; sa simple présence ne suffit pas.

Si l’outil manque, le panneau propose son téléchargement HTTPS depuis [le serveur officiel Valve](https://steamcdn-a.akamaihd.net/client/installer/steamcmd.zip). Seul le membre racine `steamcmd.exe` est extrait, avec limite de taille, contrôle CRC de l’archive et contrôle du format PE. Ce contrôle n’est pas une vérification de signature Authenticode. L’outil est placé dans `%LOCALAPPDATA%\ArkServerPanel\tools\steamcmd` et son chemin est enregistré. Ses composants se mettent à jour au premier lancement.

L’installation de SteamCMD ne démarre pas celle d’ARK : une nouvelle confirmation est demandée pour le serveur choisi. Les commandes d’installation de serveur revérifient l’emplacement juste avant de lancer l’outil.

## Quatre serveurs sur une machine

Exemple de ports de profils, à adapter aux ports déjà utilisés :

| Serveur | Dossier | Port de jeu UDP | RCON TCP |
| --- | --- | --- | --- |
| Island | `C:\ARK\Island` | 7777 | 27020 |
| Scorched | `C:\ARK\Scorched` | 7787 | 27030 |
| Center | `C:\ARK\Center` | 7797 | 27040 |
| Aberration | `C:\ARK\Aberration` | 7807 | 27050 |

Les règles de pare-feu, les ports éventuellement supplémentaires utilisés par la version du jeu, les paramètres de cluster et la connexion réelle des joueurs doivent être vérifiés sur votre installation. Un processus actif ne prouve pas que le serveur est prêt ou joignable en jeu.

## Accès Tailscale

Par défaut, le panneau écoute uniquement sur `127.0.0.1`. Pour une écoute sur l’adresse Tailscale de la machine :

```powershell
$env:ARK_PANEL_BIND = '100.101.102.103' # Remplacer par votre propre IP Tailscale
$env:ARK_PANEL_PORT = '3080'
& '.\launch.ps1'
```

Il faut redémarrer le panneau existant pour changer son adresse d’écoute. L’API accepte uniquement la valeur Host correspondant à l’adresse choisie ; utiliser cette IP explicite, pas `localhost` ou un nom MagicDNS. Depuis un autre appareil autorisé du tailnet, ouvrir `http://100.101.102.103:3080` et saisir la clé d’accès. Le programme ne configure pas le pare-feu Windows ni les règles Tailscale. Il n’ouvre aucun port sur la box.

L’API exige une clé d’accès et contrôle l’en-tête Host et les origines des requêtes. La clé est générée localement dans `%LOCALAPPDATA%\ArkServerPanel\access-key.txt`. Le raccourci l’insère dans le fragment d’URL ; l’interface l’enlève immédiatement de la barre d’adresse et la conserve dans la session du navigateur. Ne pas partager cette clé. La V1 utilise un compte administrateur commun et HTTP sur loopback ou Tailscale, sans gestion multiutilisateur ni TLS applicatif.

## GitHub et mises à jour

Le dépôt `oxo140/ark-server-panel` est **public**. Aucun jeton GitHub n’est demandé pour vérifier ou télécharger les publications. La V1 ne synchronise pas les sauvegardes ni les configurations vers GitHub.

Le panneau affiche séparément :

- la version et le SHA compilés dans le programme installé ;
- le commit actuel de `main` ;
- la dernière publication stable installable.

Une modification de `main` n’est pas automatiquement une nouvelle version utilisateur : une publication avec un tag est nécessaire. Le panneau récupère `release.json` et le binaire associés à cette publication, vérifie que le manifeste correspond au commit du tag, contrôle la taille et SHA-256, et prépare le fichier dans un nouveau dossier. Après clic sur **Redémarrer le panneau**, le superviseur lance le nouveau programme et vérifie son identité de démarrage et son SHA via `/health`. Si son démarrage échoue, la version précédente est relancée. Les processus ARK sont conservés ; les opérations SteamCMD et de fichiers bloquent la mise à jour du panneau.

Les empreintes protègent contre un téléchargement incomplet ou différent du manifeste, pas contre une compromission du compte GitHub qui publie la version. Pas de signature de code Windows dans cette V1. En cas d’erreur réseau, de dépôt inexistant, de publication absente ou de quota GitHub épuisé, le panneau affiche une erreur, sans déclarer l’installation à jour.

## Données et limites de la V1

Dossier par défaut : `%LOCALAPPDATA%\ArkServerPanel`. `ARK_PANEL_DATA` permet de choisir un autre dossier avant lancement. Il doit être distinct des dossiers de serveurs. Sauvegarder ce dossier séparément du programme.

Les sauvegardes, versions téléchargées, copies INI et anciens dossiers Saved ne sont pas supprimés automatiquement. Prévoir l’espace disque et la rétention manuellement. Ne pas lancer un serveur avec un outil extérieur pendant une sauvegarde ou une restauration. Une fermeture brutale de Windows peut interrompre une opération ; seules les sauvegardes avec un marqueur `complete.json` et des empreintes valides sont restaurables.

La V1 n’inclut pas : planificateur, Discord, orchestration de plusieurs machines, assistant réseau, rotation automatique des journaux, gestionnaire de clusters ou catalogue des mods. Les contrôles de base sont testés avec des fichiers et services simulés ; une installation et une connexion ARK réelles restent nécessaires pour valider chaque environnement.

## Historique des performances et serveur en arrière-plan

Le tableau de bord conserve les dix dernières minutes du CPU, de la RAM et des débits disque/réseau de la machine hôte, avec une mesure toutes les deux secondes. Les courbes se consultent au survol ou avec les flèches du clavier. Le réseau historique cumule les interfaces ; le disque cumule les volumes. Il ne s’agit pas des ressources d’un seul processus ARK.

Les mesures sont enregistrées localement dans `metrics-history.json` toutes les dix secondes et rechargées au prochain lancement, en ne conservant que la fenêtre des dix dernières minutes. Le relevé fonctionne tant que le panneau est actif ; une interruption apparaît comme un trou dans la courbe.

Le serveur ARK se lance sans la fenêtre provoquée par `-log`. Ses sorties et son fichier `ShooterGame.log` restent accessibles dans **Gérer / terminal**. Ce comportement prend effet au prochain lancement du serveur ; la mise à jour du panneau n’arrête pas un serveur déjà actif.

## Mise à jour publique sans compte

Depuis la version 1.5, la mise à jour utilise directement la dernière publication publique et les références Git du dépôt. Elle ne consomme plus le quota de l’API REST GitHub. Le manifeste doit toujours correspondre au commit du tag, puis le fichier doit respecter la taille et SHA-256 annoncés avant activation.

Si une ancienne version est bloquée par une erreur 403, extraire le nouveau paquet d’installation puis lancer **Mettre a jour ARK Panel.vbs**. Ce raccourci télécharge uniquement le panneau et prépare son remplacement. Si le panneau est déjà ouvert, cliquer ensuite sur **Redémarrer le panneau** dans ses paramètres. Les contrôles d’opérations en cours et la conservation des données restent les mêmes.

## Développement

Prérequis : Rust stable et les outils C++ MSVC pour compiler sous Windows. Node.js est uniquement utile pour vérifier la syntaxe JavaScript ; il n’est pas requis à l’exécution.

```powershell
cargo test --locked
cargo fmt --check
cargo clippy --locked -- -D warnings
cargo build --release --locked
.\target\release\ark-server-panel.exe
```

`--serve` démarre le serveur sans superviseur, pour les tests. Les mises à jour depuis l’interface nécessitent le lancement normal supervisé.

Organisation : `src/model.rs` pour les profils et fichiers, `catalog.rs` pour les cartes, `config.rs` pour les réglages guidés et les écritures INI, `steamcmd.rs` pour la détection et le téléchargement de l’outil, `manager.rs` pour les processus et copies vérifiées, `rcon.rs` pour le protocole, `updates.rs` pour GitHub, `supervisor.rs` pour activation/retour arrière, `server.rs` pour l’API et `web/` pour l’interface intégrée.

## Publier une version

1. Mettre à jour la version dans `Cargo.toml`, puis `cargo check` pour actualiser `Cargo.lock`.
2. Exécuter les contrôles et publier les sources sur `main`.
3. Créer et pousser un tag correspondant, par exemple `v1.0.0`.

Le workflow GitHub Actions compile et teste sous Windows, puis publie le binaire, le manifeste avec SHA-256 et l’archive d’installation. Le SHA de la compilation provient de `github.sha`. Les données locales, jetons, dossiers de serveurs et outils de compilation ne font pas partie des sources publiées.

## SteamCMD, versions ASA et performances

Le sélecteur de dossier Windows permet de choisir un disque et un emplacement pour SteamCMD ou un serveur. Il s’ouvre sur la machine qui héberge le panneau, y compris lors d’un accès distant par Tailscale. **Installer SteamCMD** télécharge l’outil officiel Valve dans le dossier choisi puis l’initialise. Les tâches SteamCMD tournent sans fenêtre CMD. La progression et les diagnostics restent accessibles dans la page web.

Chaque serveur propose **Vérifier la version ASA**, sans télécharger les fichiers du jeu, puis une confirmation distincte pour appliquer la mise à jour. Le serveur doit être arrêté pour modifier ses fichiers. La comparaison utilise le Build ID Steam de la branche publique, différent du numéro de patch affiché en jeu ; une vérification récente est requise pour confirmer.

L’installation utilise `+login anonymous +app_update 2430930 validate` : l’application dédiée ASA est accessible sans compte Steam possédant le jeu. Source : [Dedicated Server Setup](https://ark.wiki.gg/wiki/Dedicated_Server_Setup).

Le tableau de bord relève toutes les deux secondes le CPU, la mémoire, l’espace et les débits des disques, ainsi que les débits réseau de la machine Windows hôte. Les interfaces réseau sont sélectionnables ; un total incluant une interface VPN peut compter plusieurs fois le même trafic. Le lancement ouvre les journaux du serveur ; un processus actif ne garantit pas encore une connexion en jeu.


## Zone de notification et démarrage Windows

Une icône ARK apparaît près de l’horloge Windows (éventuellement dans les icônes masquées). Un double-clic ou le menu **Ouvrir ARK Server Panel** rouvre le navigateur. **Quitter le panneau** conserve les processus ARK ; la fermeture est refusée pendant une opération de fichiers ou SteamCMD. L’icône est recréée après un redémarrage d’Explorer ou une mise à jour du panneau.

Dans les paramètres, **Activer le démarrage automatique** ajoute un script appartenant au panneau dans le dossier Démarrage de l’utilisateur Windows. Il relance le panneau en arrière-plan à l’ouverture de session, avec les mêmes données, adresse et port, sans ouvrir le navigateur. Le bouton inverse retire cette entrée. Les serveurs ARK continuent à être lancés manuellement. Cette fonction utilise Windows Script Host et ne demande pas de droits administrateur.

La vérification de version ASA ne télécharge pas les fichiers du serveur. La barre reste animée tant que Steam ne fournit pas de mesure ; elle affiche le build public à la fin. Pour une installation ou validation, le pourcentage provient des lignes de progression SteamCMD et concerne l’étape en cours. Les détails techniques sont ouverts par défaut et restent repliables sous **Afficher les diagnostics détaillés**.


## Rejoindre un serveur et favoris

La carte du serveur affiche sa progression SteamCMD. **☆ Favori** conserve un favori dans le panneau ; les favoris passent en premier et un filtre est disponible dans Serveurs. Ces favoris ne sont pas écrits dans Steam ni dans le jeu.

**Connexion directe / IP** propose les adresses de la machine hôte, notamment Tailscale si présent. Choisissez l’adresse accessible depuis le PC du joueur puis enregistrez-la. Le panneau ajoute le port de jeu du profil, jamais le port RCON. 127.0.0.1 désigne uniquement la machine sur laquelle tourne le jeu ; aucun service externe n’est interrogé pour deviner une IP publique.

**Rejoindre** envoie au Steam du PC qui ouvre la page un lancement de l’application ASA 2399830 avec -ExecCmds="open IP:port". Cela s’appuie sur les arguments de console Unreal ; le lancement et l’adhésion automatique au serveur dépendent du client ASA et ne constituent pas une vérification de joignabilité. En secours, copiez open IP:port depuis le dialogue et collez-le dans la console ASA après activation de l’accès console dans le jeu. La connexion réelle, les mods et les serveurs protégés par mot de passe doivent être vérifiés en jeu.


## Mesures par serveur et SteamCMD parallèle

Les cartes des serveurs actifs affichent le CPU (part de la puissance totale de l’hôte), la RAM et les débits d’entrées/sorties de leur processus, relevés toutes les deux secondes et associés au PID et à son heure de création. Sous Windows, les compteurs d’E/S ne sont pas une mesure exclusive du disque physique. Le réseau reste mesuré globalement en haut de la page.

Chaque profil utilise sa propre instance dans `steamcmd-workers`, avec caches et fichiers de travail séparés. Les composants de SteamCMD peuvent se mettre à jour au premier lancement de chaque instance. Les données Steam personnelles et les dossiers `steamapps` ne sont pas dupliqués. Une seule opération peut modifier un serveur à la fois ; plusieurs profils distincts peuvent travailler en parallèle.

Les boutons Lancer, Vérifier ASA et Confirmer la mise à jour ne déclenchent plus l’ouverture des statistiques. Un surveillant Rust masque les fenêtres appartenant au processus ARK identifié par le panneau, sans fermer le serveur ni déplacer les fenêtres des autres applications.
