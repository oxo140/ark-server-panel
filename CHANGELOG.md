# Modifications

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
