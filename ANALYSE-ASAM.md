# Analyse initiale d’ASAM

Vérification du 6 novembre 2026 — dépôt https://github.com/CSBrad/ASAM

## Conclusion

Le dépôt public est essentiellement un dépôt de distribution de binaires. Les branches `main` et `Old_ASAM` ne contiennent ni sources C#, ni solution `.sln`, ni projet `.csproj`. On ne peut donc pas analyser toute l’implémentation ni reconstruire un fork modifiable à partir des seuls fichiers publiés. Aucune licence de réutilisation n’a été repérée dans les arbres de ces deux branches ; les conditions de réutilisation restent à clarifier auprès de l’auteur avant une reprise de code.

## Éléments vérifiés

- `main` : commit e7206678b78c3c832825c8879aee00f48bfe0298, daté du 2 mars 2026, modification du README.
- Dernière release retournée par GitHub : `v1.2.3.58`, publiée le 10 mars 2026. Cela correspond à presque huit mois au moment de la vérification, sans démontrer un abandon.
- Les binaires présents sur `main` ont été ajoutés avec la version `v1.1.2.31` du 7 septembre 2024. Ils ne représentent donc pas nécessairement la dernière release.
- `Old_ASAM` : distributions plus anciennes, fichiers XML de bibliothèques, configurations, archives ; pas de projet source.
- Runtime de `main` : .NET 8 avec Microsoft.WindowsDesktop.App.
- Dépendances déclarées de `main` : Discord.Net 3.15.3, RconSharp 2.0.1, Newtonsoft.Json 13.0.3 et Microsoft.Extensions.DependencyInjection 8.0.0, entre autres. Ces versions ne sont pas confirmées pour la dernière release.
- Le runtime de `main` active `EnableUnsafeBinaryFormatterSerialization`. Ce réglage mérite une vérification si les sources deviennent disponibles ; il ne prouve pas que l’application utilise effectivement cette API ni qu’une faille est exploitable.

## Fonctionnalités documentées, sans validation d’exécution

Gestion de profils de serveurs et clusters, installation/mise à jour via SteamCMD, édition des configurations, RCON, sauvegardes/restauration, planification et intégration Discord. Le README mentionne également une intégration Ark Server API avec installation manuelle.

## Signaux issus des utilisateurs et des releases

- Issue 34 : installation des mods signalée comme défaillante ; cause non établie.
- Issue 36 : avertissement demandé pour les changements non enregistrés, sélection des cartes plus pratique, retour visuel lors de la sauvegarde et du téléchargement SteamCMD. Un utilisateur rapporte un blocage de l’interface ; ce comportement n’a pas été reproduit.
- Dans l’issue 36, l’auteur précise que le chemin des sauvegardes est déjà configurable : ne pas présenter cette fonction comme absente.
- Issue 35 : demande de traduction ; l’auteur indique qu’une traduction chinoise attend son intégration.
- Issue 37 : demande de support Linux. La faisabilité du serveur et de son environnement doit être étudiée séparément de celle de l’interface.
- La release 1.2.3.58 traite des exceptions aux clés INI dupliquées. Un futur éditeur doit préserver les entrées répétées utilisées par ARK.

## Proposition pour un successeur

Deux chemins possibles : obtenir les sources et les conditions de réutilisation de l’auteur pour un véritable fork, ou développer une application indépendante à partir d’un cahier des charges fonctionnel et des formats de configuration.

Architecture proposée, et non architecture observée dans ASAM :

1. Un cœur indépendant de l’interface : profils, configuration INI, validation des chemins et ports, états des serveurs.
2. Des services séparés pour les processus ARK, SteamCMD, RCON et les sauvegardes.
3. Un ordonnanceur qui empêche les opérations incompatibles de s’exécuter simultanément sur le même serveur.
4. Une interface avec progression, journaux lisibles et actions asynchrones.
5. Une intégration Discord optionnelle après stabilisation de la gestion locale.

Première version proposée : profils multiples, import de serveurs existants sur copie, lancement et arrêt contrôlés, console SteamCMD, édition INI préservant les clés inconnues et répétées, sauvegardes avec rétention et restauration vérifiée. Ajouter ensuite les redémarrages planifiés, la supervision et Discord.

Points à vérifier par des tests utiles : conservation des fichiers INI après lecture/écriture, chemins avec espaces et accents, conflit de ports, échec SteamCMD, déconnexion RCON, restauration d’une sauvegarde et reprise après interruption d’une opération.

## Limites de cette analyse

Aucun exécutable lancé, aucune décompilation effectuée, aucun serveur modifié. Il s’agit d’un audit du contenu public, des manifestes, de l’historique et des retours publiés ; pas d’une revue du code source. Aucun fork GitHub créé.

## Sources

- Arbre principal : https://github.com/CSBrad/ASAM/tree/main
- Ancienne branche : https://github.com/CSBrad/ASAM/tree/Old_ASAM
- Releases : https://github.com/CSBrad/ASAM/releases
- Manifestes : https://github.com/CSBrad/ASAM/blob/main/ASAM.deps.json et https://github.com/CSBrad/ASAM/blob/main/ASAM.runtimeconfig.json
- Retours : https://github.com/CSBrad/ASAM/issues/34 et https://github.com/CSBrad/ASAM/issues/36
