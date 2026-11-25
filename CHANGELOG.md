# Modifications

## 1.7.0 — 25 novembre 2026

- Lancement, vérification ASA, installation et confirmation de mise à jour restent dans la carte ; les statistiques s’ouvrent uniquement sur demande.
- Masquage ciblé en Rust des fenêtres créées par le processus ARK, y compris après récupération d’un serveur déjà lancé ; aucun déplacement vers un autre bureau Windows.
- Instances SteamCMD isolées par profil, avec caches et dossiers de travail distincts, permettant plusieurs tâches simultanées sur des serveurs différents.
- Maintien de l’exclusion des opérations concurrentes sur un même serveur et du blocage de la mise à jour du panneau pendant les tâches.
- CPU, RAM et débits d’E/S propres à chaque processus ARK dans sa carte ; CPU normalisé à la puissance totale de la machine. Réseau global conservé en haut, sans lui attribuer un faux débit par serveur.


## 1.6.0 — 25 novembre 2026

- Cartes compactes sur trois colonnes sur grand écran, deux sur écran intermédiaire et une sur mobile ; palette conservée.
- Liste des serveurs lancés dans la barre latérale, accès aux statistiques et bouton Rejoindre.
- Pastilles vert/orange/bleu/rouge avec test RCON local périodique ; délai de démarrage de quinze minutes avant de signaler une absence de réponse.
- Statistiques par serveur : processus, mémoire, durée de fonctionnement et réponse ListPlayers.
- Export des réglages en JSON et import sur un autre serveur, sans transférer ports, adresses ni secrets ; import INI/texte UTF-8 ou UTF-16 en brouillon dans l’éditeur.
- Planification persistante par serveur : sauvegarde toutes les N heures (2 par défaut), redémarrage quotidien et vérification/mise à jour ASA quotidienne avec préavis en jeu personnalisable.
- Arrêt propre confirmé avant copie du monde, sauvegarde vérifiée avant mise à jour, relance uniquement si le serveur était actif ; aucune relance après une mise à jour échouée.

## 1.5.0 — 25 novembre 2026

- Lancement ARK sans fenêtre de journal : suppression de `-log`, sortie standard récupérée dans le panneau et mode sans console.
- Historique glissant de dix minutes du CPU, de la RAM et des débits disque/réseau de l’hôte : graphiques consultables à la souris ou au clavier, échantillonnage toutes les deux secondes et sauvegarde locale toutes les dix secondes.
- Vérification et téléchargement des mises à jour depuis les publications publiques et références Git, sans utiliser l’API REST GitHub ni demander de compte.
- Conservation des contrôles du tag, du commit, de la taille et de SHA-256 avant activation.
- Nouveau raccourci « Mettre a jour ARK Panel.vbs » pour préparer une mise à jour, y compris depuis une ancienne version bloquée par le quota API.

## 1.4.0 — 25 novembre 2026

- Barre SteamCMD avec pourcentage réel directement dans les cartes de serveur, sans ouvrir le terminal.
- Favoris dans le panneau, tri prioritaire et filtre des favoris dans la liste des serveurs.
- Choix et mémorisation de l’adresse de connexion par serveur, détection des IP locales et Tailscale, utilisation du port de jeu.
- Bouton Rejoindre via Steam avec demande de connexion ASA et commande open IP:port à copier en secours ; la connexion automatique dépend du client ASA et reste à valider en jeu.


## 1.3.2 — 25 novembre 2026

- Diagnostics détaillés ouverts par défaut, toujours repliables, avec couleurs distinctes pour erreurs, avertissements, succès et activité.
- Palette inspirée d’ARK : turquoise, bleu, améthyste et ambre, dégradés du logo, titres et progression, accents colorés des performances.


## 1.3.1 — 25 novembre 2026

- Ligne CPU, RAM, débits disque et réseau sous les barres de progression ASA et SteamCMD, actualisée toutes les deux secondes avec les mesures de la machine hôte.


## 1.3.0 — 25 novembre 2026

- Tâches SteamCMD sans fenêtre CMD, progression dans l’interface web et résultat explicite pour la vérification du build ASA.
- Barre animée pendant les étapes sans mesure, pourcentage réel des fichiers lorsque SteamCMD le fournit, diagnostics détaillés repliables.
- Icône de notification Windows native en Rust : ouverture du panneau, menu de fermeture et réapparition après redémarrage d’Explorer.
- Bouton d’activation/désactivation du démarrage à l’ouverture de session Windows, avec conservation des données et du port.


## 1.2.1 — 25 novembre 2026

- Rafraîchissement des informations ASA avant installation ; nouvelle tentative unique pour l’erreur SteamCMD « Missing configuration », avec diagnostic explicite si elle persiste.
- Prise en charge du redémarrage de SteamCMD après sa propre mise à jour pendant une vérification ou installation.
- Console Windows conservée jusqu’à Entrée, accents UTF-8 et résultat final conservé dans le terminal web.

## 1.2.0 — 25 novembre 2026

- Sélection native de dossiers Windows pour SteamCMD et les serveurs, y compris sur un autre disque.
- Installation et initialisation de SteamCMD avec console Windows facultative et terminal de diagnostic web.
- Vérification du build public ASA distincte de la confirmation de mise à jour ; connexion Steam anonyme.
- Mesures CPU, RAM, espace disque, débits disque et réseau de la machine hôte.
- Lancement du serveur avec accès direct aux journaux ; suivi des tâches SteamCMD après redémarrage du panneau.

## 1.1.0 — 23 novembre 2026

- Sélection des dix cartes ASA disponibles par nom lisible, conservation du code de lancement et option de carte personnalisée.
- Vérification de SteamCMD avant la confirmation d’installation d’ARK ; proposition de téléchargement depuis Valve si l’outil manque.
- Installation du bootstrap SteamCMD dans les données du panneau, vérification ZIP/PE et enregistrement du chemin sans lancement automatique d’ARK.
- Éditeur guidé de 33 réglages répartis en sept catégories, avec curseurs, valeurs précises, interrupteurs et recherche.
- Préréglages Solo PvE confort et Duo PvE équilibré : aperçu en brouillon avant enregistrement.
- Conservation des options inconnues, des commentaires et des clés répétées ; détection de changements externes et retour aux fichiers précédents si une écriture échoue.
- Lecture des fichiers INI avec marqueur UTF-8 BOM, y compris pour le mot de passe RCON.
- Rechargement de l’interface lors des prochains changements de version, avec conservation des brouillons non enregistrés.

Depuis une page ouverte en 1.0.x, actualiser le navigateur après la mise à jour du panneau pour charger la nouvelle interface.

## 1.0.1

- Correction de la récupération des processus Windows lorsque leurs chemins utilisent des séparateurs différents.

## 1.0.0

- Première version du panneau web Rust pour Windows : profils, SteamCMD, RCON, configurations, sauvegardes et mises à jour GitHub.
