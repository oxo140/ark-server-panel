# Modifications

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
