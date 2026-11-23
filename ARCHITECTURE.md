# Gestionnaire ARK avec interface web

## Direction retenue

Application indépendante d’ASAM. L’utilisateur ouvre le gestionnaire dans son navigateur. Les serveurs ARK et SteamCMD s’exécutent sur la machine Windows qui héberge le gestionnaire.

Le navigateur communique avec un service local : il ne lance pas directement les exécutables et ne modifie pas lui-même les fichiers Windows.

```text
Navigateur web
      |
      | HTTP : actions et configuration
      | Flux temps réel : journaux, progression, état
      v
Service du gestionnaire sur Windows
      |-- Profils et paramètres
      |-- Processus ARK et suivi de leur état
      |-- Installation / mise à jour avec SteamCMD
      |-- Commandes RCON
      |-- Lecture / écriture des fichiers INI
      |-- Sauvegardes / restauration
      `-- Tâches planifiées
```

## Fonctionnement attendu

- Adresse locale configurable, par exemple `http://localhost:3000`.
- Le service peut fonctionner en arrière-plan, puis être installé comme service Windows pour démarrer avec la machine.
- Fermer l’onglet ne ferme pas les serveurs et n’annule pas les opérations en cours.
- Les opérations longues sont exécutées et suivies côté service. Une reconnexion du navigateur retrouve leur état.
- Les processus sont suivis individuellement : lancement demandé, processus présent et serveur répondant sont des états distincts.
- Après un redémarrage du service, le rapprochement avec les processus existants doit vérifier leur identité et leur chemin ; un PID seul ne suffit pas.
- L’accès depuis un autre ordinateur sera une option explicite, avec authentification et configuration réseau adaptées. Par défaut, écoute locale uniquement.

## Organisation proposée

1. Interface web : tableau de bord, fiches des serveurs, configuration, console, sauvegardes et planification.
2. API et flux temps réel : actions autorisées, validation des paramètres, état des tâches et journaux.
3. Services métier : cycle de vie ARK, SteamCMD, RCON, INI, sauvegardes et ordonnanceur.
4. Stockage local : profils, historique et tâches persistantes ; fichiers de serveurs et archives dans des dossiers configurables.

Implémentation V1 : service Rust avec Axum/Tokio, interface HTML/CSS/JavaScript intégrée au binaire et adresse locale commune. Aucun runtime Node.js ou Python requis chez l’utilisateur. Les mises à jour utilisent les publications GitHub Windows, un manifeste SHA-256 et un superviseur Rust. Voir README.md pour les fonctions effectivement présentes et les limites de cette première version ; la liste ci-dessous reste la cible d’évolution.

## Règles de fonctionnement

- Aucune commande système libre fournie par le navigateur : actions dédiées et arguments validés.
- Mots de passe RCON et jetons conservés côté service, exclus des journaux et des réponses ordinaires de l’API.
- Vérification des requêtes web et protection contre les actions provenant d’un autre site, y compris en accès local.
- Une opération exclusive par serveur pour éviter, par exemple, une restauration pendant une mise à jour.
- Écriture des configurations avec sauvegarde préalable, validation et remplacement maîtrisé ; conservation des entrées INI inconnues et répétées.
- Une archive terminée n’est pas à elle seule la preuve d’une sauvegarde cohérente : définir et vérifier la procédure de sauvegarde du serveur.
- La fermeture et le redémarrage du service suivent une politique explicite pour les serveurs en cours, sans arrêt implicite des processus.

## Première version

1. Tableau de bord et création/import de profils sans écraser les installations existantes.
2. Installation et mise à jour SteamCMD avec progression et journaux.
3. Démarrage, suivi, arrêt contrôlé et console RCON par serveur.
4. Édition des paramètres et fichiers INI.
5. Sauvegarde, rétention et restauration avec vérification.
6. Planification persistante des sauvegardes et redémarrages.

Discord et la gestion de plusieurs machines pourront être ajoutés après stabilisation de cette base.
