# ARK Server Panel

Panneau web indépendant pour gérer des serveurs **ARK: Survival Ascended sur Windows**. Le service, les opérations système, RCON, les sauvegardes et les mises à jour sont écrits en **Rust**. L’interface HTML/CSS/JavaScript est intégrée au binaire : aucun serveur Node.js, Python ou outil de compilation n’est nécessaire sur une installation utilisateur.

## Installer la V1

1. Télécharger `ark-server-panel-install.zip` dans les [publications GitHub](https://github.com/oxo140/ark-server-panel/releases).
2. Extraire l’archive dans un dossier conservé sur la machine Windows qui héberge ARK.
3. Ouvrir **Ouvrir ARK Panel.vbs**. Au premier lancement, le script récupère la dernière publication Windows x64 et contrôle son empreinte SHA-256. Le programme ouvre ensuite le navigateur sur `http://127.0.0.1:3080`.
4. Dans **Paramètres & mises à jour**, renseigner le chemin absolu de `steamcmd.exe`, téléchargé depuis la distribution officielle de Valve.
5. Ajouter un profil : un dossier distinct par serveur, une carte et des ports distincts. Cliquer sur **Installer / mettre à jour**, suivre les journaux, puis démarrer le serveur lorsque SteamCMD a terminé avec succès.

Pour reprendre un serveur existant, créer un profil pointant vers son dossier racine contenant `ShooterGame`. La création du profil ne modifie pas ses fichiers. Ne pas cliquer sur l’installation si vous souhaitez uniquement l’importer.

L’exécutable Rust travaille en arrière-plan : l’administration se fait entièrement dans le navigateur. Fermer l’onglet n’arrête pas les serveurs. Le démarrage automatique avec Windows et l’installation comme service Windows ne sont pas inclus dans cette V1.

## Fonctions présentes

- Profils multiples avec contrôle des conflits de ports et des dossiers partagés.
- Lancement d’ARK, détection des processus correspondant au chemin du serveur, récupération de leur identité après redémarrage du panneau.
- Installation et validation via SteamCMD, état de l’opération et consultation des journaux.
- Console RCON et demande d’arrêt avec `SaveWorld`, puis `DoExit`. Aucun arrêt forcé automatique.
- Édition brute de `Game.ini` et `GameUserSettings.ini` en UTF-8, conservation des clés répétées et inconnues, détection des modifications concurrentes, copie préalable et écriture par remplacement atomique.
- Sauvegarde du dossier `ShooterGame/Saved` complet, uniquement serveur arrêté, avec contrôle SHA-256 de chaque fichier. Les sauvegardes sont des dossiers, sans compression dans la V1.
- Restauration avec contrôle d’intégrité et conservation de l’ancien dossier Saved sous un autre nom.
- Vérification des versions GitHub au démarrage et toutes les douze heures ; téléchargement et activation manuels depuis le panneau.

## RCON et configuration

Dans `ShooterGame/Saved/Config/WindowsServer/GameUserSettings.ini`, définir :

```ini
[ServerSettings]
RCONEnabled=True
RCONPort=27020
ServerAdminPassword=REMPLACER_PAR_UN_MOT_DE_PASSE
```

Utiliser le même port RCON que dans le profil. Le mot de passe est lu dans le fichier de configuration du serveur au moment de la commande ; le panneau ne le copie pas dans ses profils ni dans Git. L’éditeur INI montre le contenu complet aux administrateurs disposant de la clé d’accès.

Le choix des identifiants de mods et des noms de cartes reste à la charge de l’administrateur. Le panneau transmet les identifiants au serveur ; il ne fournit pas de catalogue CurseForge dans la V1.

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

La V1 n’inclut pas : planificateur, Discord, supervision CPU/RAM, orchestration de plusieurs machines, assistant réseau, rotation automatique des journaux, gestionnaire de clusters ou catalogue des mods. Les contrôles de base sont testés avec des fichiers et services simulés ; une installation et une connexion ARK réelles restent nécessaires pour valider chaque environnement.

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

Organisation : `src/model.rs` pour les profils et fichiers, `manager.rs` pour les processus et copies vérifiées, `rcon.rs` pour le protocole, `updates.rs` pour GitHub, `supervisor.rs` pour activation/retour arrière, `server.rs` pour l’API et `web/` pour l’interface intégrée.

## Publier une version

1. Mettre à jour la version dans `Cargo.toml`, puis `cargo check` pour actualiser `Cargo.lock`.
2. Exécuter les contrôles et publier les sources sur `main`.
3. Créer et pousser un tag correspondant, par exemple `v1.0.0`.

Le workflow GitHub Actions compile et teste sous Windows, puis publie le binaire, le manifeste avec SHA-256 et l’archive d’installation. Le SHA de la compilation provient de `github.sha`. Les données locales, jetons, dossiers de serveurs et outils de compilation ne font pas partie des sources publiées.
