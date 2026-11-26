use crate::model::{self, Profile};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, path::Path};

pub const GUS: &str = "GameUserSettings.ini";
pub const GAME: &str = "Game.ini";
const SERVER: &str = "ServerSettings";
const MODE: &str = "/Script/ShooterGame.ShooterGameMode";

#[derive(Clone, Serialize)]
pub struct Setting {
    pub key: &'static str,
    pub file: &'static str,
    pub section: &'static str,
    pub label: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub kind: &'static str,
    pub min: f64,
    pub max: f64,
    pub step: f64,
    pub default: Value,
}
fn number(
    key: &'static str,
    file: &'static str,
    label: &'static str,
    category: &'static str,
    description: &'static str,
    range: (f64, f64, f64),
    default: f64,
) -> Setting {
    Setting {
        key,
        file,
        section: if file == GUS { SERVER } else { MODE },
        label,
        category,
        description,
        kind: "number",
        min: range.0,
        max: range.1,
        step: range.2,
        default: json!(default),
    }
}
fn boolean(
    key: &'static str,
    file: &'static str,
    label: &'static str,
    category: &'static str,
    description: &'static str,
    default: bool,
) -> Setting {
    Setting {
        key,
        file,
        section: if file == GUS { SERVER } else { MODE },
        label,
        category,
        description,
        kind: "boolean",
        min: 0.,
        max: 1.,
        step: 1.,
        default: json!(default),
    }
}
pub fn schema() -> Vec<Setting> {
    let mut fields = vec![
        boolean(
            "ServerPVE",
            GUS,
            "Mode PvE",
            "Règles du monde",
            "Active le mode joueur contre environnement.",
            false,
        ),
        number(
            "DifficultyOffset",
            GUS,
            "Difficulté de base",
            "Règles du monde",
            "S’applique sans surcharge de difficulté. Le plafond estimé selon la carte et vos réglages s’affiche juste en dessous.",
            (0., 1., 0.1),
            1.,
        ),
        number(
            "OverrideOfficialDifficulty",
            GUS,
            "Difficulté maximale",
            "Règles du monde",
            "0 laisse la carte décider ; 5 donne 150, 10 donne 300 pour les dinos ordinaires. Ne change pas les dinos déjà apparus.",
            (0., 10., 0.5),
            0.,
        ),
        boolean(
            "bUseSingleplayerSettings",
            GAME,
            "Bonus du mode solo natif",
            "Règles du monde",
            "Cumule des bonus cachés avec vos taux. Les préréglages du panneau le désactivent pour garder des taux prévisibles.",
            false,
        ),
        number(
            "XPMultiplier",
            GUS,
            "Expérience",
            "Progression & récolte",
            "Plus élevé : progression des niveaux plus rapide.",
            (0.1, 20., 0.1),
            1.,
        ),
        number(
            "HarvestAmountMultiplier",
            GUS,
            "Quantité de ressources",
            "Progression & récolte",
            "Plus élevé : davantage de ressources à chaque récolte.",
            (0.1, 20., 0.1),
            1.,
        ),
        number(
            "HarvestHealthMultiplier",
            GUS,
            "Résistance des ressources",
            "Progression & récolte",
            "Plus élevé : les rochers et arbres nécessitent davantage de coups.",
            (0.1, 10., 0.1),
            1.,
        ),
        number(
            "ResourcesRespawnPeriodMultiplier",
            GUS,
            "Délai de réapparition des ressources",
            "Progression & récolte",
            "Plus bas : les ressources réapparaissent plus vite, sous réserve des distances de blocage.",
            (0.01, 5., 0.01),
            1.,
        ),
        number(
            "TamingSpeedMultiplier",
            GUS,
            "Vitesse d’apprivoisement",
            "Apprivoisement",
            "Plus élevé : apprivoisement plus rapide. Dépend aussi de la nourriture et de l’espèce.",
            (0.1, 50., 0.1),
            1.,
        ),
        number(
            "DinoCharacterFoodDrainMultiplier",
            GUS,
            "Consommation de nourriture des dinos",
            "Apprivoisement",
            "Agit aussi sur les intervalles de repas pendant l’apprivoisement.",
            (0.1, 5., 0.1),
            1.,
        ),
        number(
            "MatingIntervalMultiplier",
            GAME,
            "Intervalle entre reproductions",
            "Élevage & imprégnation",
            "Plus bas : les femelles peuvent se reproduire plus souvent.",
            (0.001, 5., 0.001),
            1.,
        ),
        number(
            "MatingSpeedMultiplier",
            GAME,
            "Vitesse d’accouplement",
            "Élevage & imprégnation",
            "Plus élevé : accouplement plus rapide.",
            (0.1, 20., 0.1),
            1.,
        ),
        number(
            "EggHatchSpeedMultiplier",
            GAME,
            "Éclosion et gestation",
            "Élevage & imprégnation",
            "Plus élevé : incubation des œufs et gestation plus rapides.",
            (0.1, 200., 0.1),
            1.,
        ),
        number(
            "BabyMatureSpeedMultiplier",
            GAME,
            "Croissance des bébés",
            "Élevage & imprégnation",
            "Plus élevé : maturation plus rapide. À accorder avec l’intervalle de soins.",
            (0.1, 200., 0.1),
            1.,
        ),
        number(
            "BabyCuddleIntervalMultiplier",
            GAME,
            "Intervalle des soins",
            "Élevage & imprégnation",
            "Plus bas : soins demandés plus tôt. 0,033333 équivaut à environ 16 minutes pour une base de 8 h.",
            (0.001, 2., 0.001),
            1.,
        ),
        number(
            "BabyImprintAmountMultiplier",
            GAME,
            "Imprégnation gagnée par soin",
            "Élevage & imprégnation",
            "Plus élevé : davantage d’imprégnation à chaque interaction. Le résultat dépend de l’espèce.",
            (0.1, 100., 0.1),
            1.,
        ),
        number(
            "BabyImprintingStatScaleMultiplier",
            GAME,
            "Bonus des statistiques d’imprégnation",
            "Élevage & imprégnation",
            "1 conserve le bonus normal. Les préréglages ne renforcent pas ce bonus.",
            (0., 5., 0.1),
            1.,
        ),
        number(
            "BabyFoodConsumptionSpeedMultiplier",
            GAME,
            "Consommation des bébés",
            "Élevage & imprégnation",
            "Plus bas : les bébés consomment moins de nourriture.",
            (0.1, 5., 0.1),
            1.,
        ),
        number(
            "LayEggIntervalMultiplier",
            GAME,
            "Intervalle de ponte",
            "Agriculture & durée du jour",
            "Plus bas : les œufs non fécondés sont pondus plus souvent.",
            (0.1, 5., 0.1),
            1.,
        ),
        number(
            "CropGrowthSpeedMultiplier",
            GAME,
            "Croissance des cultures",
            "Agriculture & durée du jour",
            "Plus élevé : les cultures poussent plus vite.",
            (0.1, 20., 0.1),
            1.,
        ),
        number(
            "DayCycleSpeedScale",
            GUS,
            "Vitesse du cycle jour / nuit",
            "Agriculture & durée du jour",
            "Plus bas : journées et nuits plus longues.",
            (0.1, 5., 0.1),
            1.,
        ),
        number(
            "DayTimeSpeedScale",
            GUS,
            "Vitesse du jour",
            "Agriculture & durée du jour",
            "Plus bas : le jour dure plus longtemps.",
            (0.1, 5., 0.1),
            1.,
        ),
        number(
            "NightTimeSpeedScale",
            GUS,
            "Vitesse de la nuit",
            "Agriculture & durée du jour",
            "Plus élevé : les nuits passent plus vite.",
            (0.1, 5., 0.1),
            1.,
        ),
        number(
            "PlayerCharacterFoodDrainMultiplier",
            GUS,
            "Faim du joueur",
            "Survie & confort",
            "Plus bas : le joueur a moins souvent faim.",
            (0.1, 5., 0.1),
            1.,
        ),
        number(
            "PlayerCharacterWaterDrainMultiplier",
            GUS,
            "Soif du joueur",
            "Survie & confort",
            "Plus bas : le joueur a moins souvent soif.",
            (0.1, 5., 0.1),
            1.,
        ),
        number(
            "PlayerCharacterStaminaDrainMultiplier",
            GUS,
            "Consommation d’endurance",
            "Survie & confort",
            "Plus bas : l’endurance du joueur diminue moins vite.",
            (0.1, 5., 0.1),
            1.,
        ),
        number(
            "PlayerCharacterHealthRecoveryMultiplier",
            GUS,
            "Récupération de santé",
            "Survie & confort",
            "Plus élevé : récupération naturelle plus rapide.",
            (0.1, 5., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_Player[7]",
            GAME,
            "Poids gagné par niveau du joueur",
            "Survie & confort",
            "Multiplie uniquement les points de poids investis par le joueur, sans modifier les dinos.",
            (0.1, 10., 0.1),
            1.,
        ),
        boolean(
            "AllowFlyerCarryPvE",
            GUS,
            "Transport par les volants en PvE",
            "Survie & confort",
            "Autorise les volants à porter des créatures en PvE.",
            false,
        ),
        boolean(
            "ShowMapPlayerLocation",
            GUS,
            "Position du joueur sur la carte",
            "Affichage",
            "Affiche la position du joueur sur sa carte.",
            true,
        ),
        boolean(
            "ServerCrosshair",
            GUS,
            "Réticule",
            "Affichage",
            "Affiche le réticule de visée.",
            false,
        ),
        boolean(
            "AllowThirdPersonPlayer",
            GUS,
            "Vue à la troisième personne",
            "Affichage",
            "Autorise la vue à la troisième personne.",
            false,
        ),
        boolean(
            "ShowFloatingDamageText",
            GUS,
            "Nombres de dégâts",
            "Affichage",
            "Affiche les nombres de dégâts.",
            false,
        ),
    ];
    fields.extend(advanced_schema());
    fields
}
fn advanced_schema() -> Vec<Setting> {
    vec![
        boolean(
            "bAllowSpeedLeveling",
            GAME,
            "Vitesse du joueur et des dinos terrestres",
            "Déplacement & volants",
            "Autorise l’investissement de points en vitesse dans ASA.",
            false,
        ),
        boolean(
            "bAllowFlyerSpeedLeveling",
            GAME,
            "Vitesse des dinos volants",
            "Déplacement & volants",
            "Nécessite aussi la vitesse du joueur et des dinos terrestres. Certaines espèces ou mods gardent leurs propres restrictions.",
            false,
        ),
        boolean(
            "bAllowAnyoneBabyImprintCuddle",
            GAME,
            "Tout membre peut faire les soins",
            "Élevage & imprégnation",
            "Permet à un autre joueur d’effectuer les soins du bébé.",
            false,
        ),
        boolean(
            "bDisableStructurePlacementCollision",
            GAME,
            "Construction : ignorer certaines collisions",
            "Construction & confort",
            "Assouplit les collisions de placement des structures.",
            false,
        ),
        boolean(
            "bAllowPlatformSaddleMultiFloors",
            GAME,
            "Plusieurs étages sur selles plateformes",
            "Construction & confort",
            "Autorise plusieurs étages sur les plateformes.",
            false,
        ),
        boolean(
            "bUseCorpseLocator",
            GUS,
            "Repère du corps",
            "Affichage",
            "Affiche le repère de récupération du corps.",
            true,
        ),
        boolean(
            "AlwaysAllowStructurePickup",
            GUS,
            "Ramassage permanent des structures",
            "Construction & confort",
            "Autorise le ramassage des structures sans limite de temps.",
            false,
        ),
        boolean(
            "DisableStructureDecayPvE",
            GUS,
            "Désactiver la dégradation des structures PvE",
            "Construction & confort",
            "Conserve les structures PvE sans dégradation automatique.",
            false,
        ),
        boolean(
            "AllowCaveBuildingPvE",
            GUS,
            "Construction en grotte en PvE",
            "Construction & confort",
            "Autorise la construction dans les grottes PvE.",
            false,
        ),
        boolean(
            "AllowCaveBuildingPvP",
            GUS,
            "Construction en grotte en PvP",
            "Construction & confort",
            "Autorise la construction dans les grottes PvP.",
            true,
        ),
        boolean(
            "bDisableFriendlyFire",
            GAME,
            "Désactiver les dégâts alliés",
            "Combat & PvE",
            "Désactive les dégâts entre alliés.",
            false,
        ),
        boolean(
            "bPvEDisableFriendlyFire",
            GAME,
            "Désactiver les dégâts alliés en PvE",
            "Combat & PvE",
            "Désactive les dégâts alliés lorsque le serveur est en PvE.",
            false,
        ),
        boolean(
            "bAllowCustomRecipes",
            GAME,
            "Recettes personnalisées",
            "Construction & confort",
            "Autorise les recettes personnalisées.",
            true,
        ),
        boolean(
            "bDisableDinoRiding",
            GAME,
            "Interdire de monter les dinos",
            "Déplacement & volants",
            "Empêche de monter les dinos ; change fortement le gameplay.",
            false,
        ),
        boolean(
            "bDisableDinoTaming",
            GAME,
            "Interdire l’apprivoisement",
            "Combat & PvE",
            "Désactive les nouveaux apprivoisements.",
            false,
        ),
        boolean(
            "PreventDownloadSurvivors",
            GUS,
            "Bloquer les transferts de survivants",
            "Transferts",
            "Empêche le téléchargement de survivants sur ce serveur.",
            false,
        ),
        boolean(
            "PreventDownloadItems",
            GUS,
            "Bloquer les transferts d’objets",
            "Transferts",
            "Empêche le téléchargement d’objets sur ce serveur.",
            false,
        ),
        boolean(
            "PreventDownloadDinos",
            GUS,
            "Bloquer les transferts de dinos",
            "Transferts",
            "Empêche le téléchargement de dinos sur ce serveur.",
            false,
        ),
        number(
            "DinoCountMultiplier",
            GUS,
            "Densité des dinos sauvages",
            "Règles du monde",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0., 5., 0.1),
            1.,
        ),
        number(
            "PlayerDamageMultiplier",
            GUS,
            "Dégâts du joueur",
            "Combat & PvE",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0.1, 10., 0.1),
            1.,
        ),
        number(
            "PlayerResistanceMultiplier",
            GUS,
            "Dégâts reçus par le joueur",
            "Combat & PvE",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0.1, 10., 0.1),
            1.,
        ),
        number(
            "DinoDamageMultiplier",
            GUS,
            "Dégâts des dinos",
            "Combat & PvE",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0.1, 10., 0.1),
            1.,
        ),
        number(
            "DinoResistanceMultiplier",
            GUS,
            "Dégâts reçus par les dinos",
            "Combat & PvE",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0.1, 10., 0.1),
            1.,
        ),
        number(
            "PlayerHarvestingDamageMultiplier",
            GUS,
            "Puissance de récolte du joueur",
            "Progression & récolte",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0.1, 10., 0.1),
            1.,
        ),
        number(
            "DinoHarvestingDamageMultiplier",
            GAME,
            "Puissance de récolte des dinos",
            "Progression & récolte",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0.1, 10., 0.1),
            3.2,
        ),
        number(
            "BabyCuddleGracePeriodMultiplier",
            GAME,
            "Délai de grâce des soins",
            "Élevage & imprégnation",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0.1, 10., 0.1),
            1.,
        ),
        number(
            "BabyCuddleLoseImprintQualitySpeedMultiplier",
            GAME,
            "Perte de qualité des soins",
            "Élevage & imprégnation",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "SupplyCrateLootQualityMultiplier",
            GUS,
            "Qualité des caisses de butin",
            "Progression & récolte",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0.1, 10., 0.1),
            1.,
        ),
        number(
            "FishingLootQualityMultiplier",
            GAME,
            "Qualité du butin de pêche",
            "Progression & récolte",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0.1, 10., 0.1),
            1.,
        ),
        number(
            "ItemStackSizeMultiplier",
            GUS,
            "Taille des piles d’objets",
            "Construction & confort",
            "Multiplicateur du réglage natif. Les mods peuvent modifier son effet.",
            (0.1, 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_Player[0]",
            GAME,
            "Santé — joueur",
            "Joueur : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_Player[1]",
            GAME,
            "Endurance — joueur",
            "Joueur : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_Player[3]",
            GAME,
            "Oxygène — joueur",
            "Joueur : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_Player[4]",
            GAME,
            "Nourriture — joueur",
            "Joueur : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_Player[5]",
            GAME,
            "Eau — joueur",
            "Joueur : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_Player[8]",
            GAME,
            "Dégâts de mêlée — joueur",
            "Joueur : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_Player[9]",
            GAME,
            "Vitesse — joueur",
            "Joueur : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_Player[10]",
            GAME,
            "Résistance / fortitude — joueur",
            "Joueur : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_Player[11]",
            GAME,
            "Compétence artisanale — joueur",
            "Joueur : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoWild[0]",
            GAME,
            "Santé — sauvage",
            "Dinos : points sauvages",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoWild[1]",
            GAME,
            "Endurance — sauvage",
            "Dinos : points sauvages",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoWild[3]",
            GAME,
            "Oxygène — sauvage",
            "Dinos : points sauvages",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoWild[4]",
            GAME,
            "Nourriture — sauvage",
            "Dinos : points sauvages",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoWild[7]",
            GAME,
            "Poids — sauvage",
            "Dinos : points sauvages",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoWild[8]",
            GAME,
            "Dégâts de mêlée — sauvage",
            "Dinos : points sauvages",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoWild[9]",
            GAME,
            "Vitesse — sauvage",
            "Dinos : points sauvages",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed[0]",
            GAME,
            "Santé — dino apprivoisé",
            "Dinos apprivoisés : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            0.2,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed[1]",
            GAME,
            "Endurance — dino apprivoisé",
            "Dinos apprivoisés : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed[3]",
            GAME,
            "Oxygène — dino apprivoisé",
            "Dinos apprivoisés : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed[4]",
            GAME,
            "Nourriture — dino apprivoisé",
            "Dinos apprivoisés : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed[7]",
            GAME,
            "Poids — dino apprivoisé",
            "Dinos apprivoisés : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed[8]",
            GAME,
            "Dégâts de mêlée — dino apprivoisé",
            "Dinos apprivoisés : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            0.17,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed[9]",
            GAME,
            "Vitesse — dino apprivoisé",
            "Dinos apprivoisés : gains par point",
            "Multiplie le gain de cette statistique par point. ×2 double le gain standard ; cela ne double pas toute la statistique. Pour la vitesse, activer aussi les cases correspondantes.",
            (0., 10., 0.1),
            1.,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed_Add[0]",
            GAME,
            "Santé — bonus apprivoisement",
            "Dinos : bonus additif à l’apprivoisement",
            "Bonus d’apprivoisement : effet selon l’espèce, distinct des gains par point.",
            (0., 10., 0.1),
            0.14,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed_Add[8]",
            GAME,
            "Dégâts de mêlée — bonus apprivoisement",
            "Dinos : bonus additif à l’apprivoisement",
            "Bonus d’apprivoisement : effet selon l’espèce, distinct des gains par point.",
            (0., 10., 0.1),
            0.14,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed_Affinity[0]",
            GAME,
            "Santé — bonus apprivoisement",
            "Dinos : bonus d’efficacité d’apprivoisement",
            "Bonus d’apprivoisement : effet selon l’espèce, distinct des gains par point.",
            (0., 10., 0.1),
            0.44,
        ),
        number(
            "PerLevelStatsMultiplier_DinoTamed_Affinity[8]",
            GAME,
            "Dégâts de mêlée — bonus apprivoisement",
            "Dinos : bonus d’efficacité d’apprivoisement",
            "Bonus d’apprivoisement : effet selon l’espèce, distinct des gains par point.",
            (0., 10., 0.1),
            0.44,
        ),
        {
            let mut s = boolean(
                "MaxDifficulty",
                GAME,
                "Difficulté maximale forcée (150)",
                "Règles du monde",
                "Option native pouvant prendre le pas sur la surcharge. Désactiver pour un plafond personnalisé supérieur à 150.",
                false,
            );
            s.section = "ShooterGameMode_Options";
            s
        },
    ]
}
pub fn presets() -> Value {
    let shared = json!({"ServerPVE":true,"MaxWildDinoLevel":150,"bUseSingleplayerSettings":false,"BabyImprintAmountMultiplier":2,"BabyImprintingStatScaleMultiplier":1,"AllowFlyerCarryPvE":true,"ShowMapPlayerLocation":true,"ServerCrosshair":true,"AllowThirdPersonPlayer":true});
    let mut solo = shared.clone();
    let mut duo = shared;
    solo.as_object_mut().unwrap().extend(json!({"XPMultiplier":2,"HarvestAmountMultiplier":3,"TamingSpeedMultiplier":5,"ResourcesRespawnPeriodMultiplier":0.5,"MatingIntervalMultiplier":0.1,"MatingSpeedMultiplier":2,"EggHatchSpeedMultiplier":30,"BabyMatureSpeedMultiplier":30,"BabyCuddleIntervalMultiplier":0.033333,"BabyFoodConsumptionSpeedMultiplier":0.5,"CropGrowthSpeedMultiplier":3,"LayEggIntervalMultiplier":0.5,"PlayerCharacterFoodDrainMultiplier":0.5,"PlayerCharacterWaterDrainMultiplier":0.5,"PerLevelStatsMultiplier_Player[7]":2,"NightTimeSpeedScale":2}).as_object().unwrap().clone());
    duo.as_object_mut().unwrap().extend(json!({"XPMultiplier":1.5,"HarvestAmountMultiplier":2,"TamingSpeedMultiplier":3,"ResourcesRespawnPeriodMultiplier":0.75,"MatingIntervalMultiplier":0.2,"MatingSpeedMultiplier":1.5,"EggHatchSpeedMultiplier":15,"BabyMatureSpeedMultiplier":15,"BabyCuddleIntervalMultiplier":0.066667,"BabyFoodConsumptionSpeedMultiplier":0.75,"CropGrowthSpeedMultiplier":2,"LayEggIntervalMultiplier":0.75,"PlayerCharacterFoodDrainMultiplier":0.75,"PlayerCharacterWaterDrainMultiplier":0.75,"PerLevelStatsMultiplier_Player[7]":1.5,"NightTimeSpeedScale":1.5}).as_object().unwrap().clone());
    json!([
        {"id":"solo","label":"Solo PvE confort","summary":"XP ×2 · récolte ×3 · apprivoisement ×5 · élevage ×30","description":"Moins de corvées et d’attente pour une personne. Poids par niveau ×2. Ne modifie pas les dégâts ni la résistance. Bonus d’imprégnation standard.","values":solo},
        {"id":"duo","label":"Duo PvE équilibré","summary":"XP ×1,5 · récolte ×2 · apprivoisement ×3 · élevage ×15","description":"Progression plus proche du jeu de base pour deux joueurs, avec des temps d’attente réduits. Poids par niveau ×1,5.","values":duo}
    ])
}
fn header(line: &str) -> Option<&str> {
    let t = line.trim_start_matches('\u{feff}').trim();
    if t.starts_with('[') && t.ends_with(']') {
        Some(&t[1..t.len() - 1])
    } else {
        None
    }
}
fn entry(line: &str) -> Option<(&str, &str)> {
    let line = line.trim_start_matches('\u{feff}').trim();
    if line.starts_with(';') || line.starts_with('#') {
        None
    } else {
        line.split_once('=').map(|(a, b)| (a.trim(), b.trim()))
    }
}
fn scalar(value: &str) -> &str {
    value
        .split(" ;")
        .next()
        .unwrap_or(value)
        .split(" #")
        .next()
        .unwrap_or(value)
        .trim()
}
pub fn read_value(text: &str, section: &str, key: &str) -> Option<String> {
    let mut active = false;
    let mut found = None;
    for line in text.lines() {
        if let Some(h) = header(line) {
            active = h.eq_ignore_ascii_case(section);
        } else if active
            && let Some((k, v)) = entry(line)
            && k.eq_ignore_ascii_case(key)
        {
            found = Some(scalar(v).into());
        }
    }
    found
}
pub fn patch(text: &str, section: &str, key: &str, value: &str) -> String {
    let newline = if text.contains("\r\n") { "\r\n" } else { "\n" };
    let mut lines: Vec<String> = text.split_inclusive('\n').map(String::from).collect();
    let mut active = false;
    let mut found = false;
    let mut insertion = None;
    for (i, line) in lines.iter_mut().enumerate() {
        if let Some(h) = header(line) {
            if active {
                insertion = Some(i);
            }
            active = h.eq_ignore_ascii_case(section);
            if active {
                insertion = Some(i + 1);
            }
            continue;
        }
        if active {
            insertion = Some(i + 1);
            if let Some((k, _)) = entry(line)
                && k.eq_ignore_ascii_case(key)
            {
                let eq = line.find('=').unwrap();
                let ending = if line.ends_with("\r\n") {
                    "\r\n"
                } else if line.ends_with('\n') {
                    "\n"
                } else {
                    ""
                };
                let raw = &line[eq + 1..line.len() - ending.len()];
                let whitespace = &raw[..raw.len() - raw.trim_start().len()];
                let comment_index = raw.find(" ;").or_else(|| raw.find(" #"));
                let suffix = comment_index.map(|i| &raw[i..]).unwrap_or("");
                *line = format!(
                    "{}{}{}{}{}",
                    &line[..eq + 1],
                    whitespace,
                    value,
                    suffix,
                    ending
                );
                found = true;
            }
        }
    }
    if !found {
        let line = format!("{key}={value}{newline}");
        if let Some(i) = insertion {
            if i > 0 && !lines[i - 1].ends_with('\n') {
                lines[i - 1].push_str(newline);
            }
            lines.insert(i, line);
        } else {
            if !text.is_empty() && !text.ends_with('\n') {
                lines.push(newline.into());
            }
            lines.push(format!("[{section}]{newline}{line}"));
        }
    }
    lines.concat()
}
pub fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}
fn read_file(p: &Profile, name: &str) -> Result<(bool, String), String> {
    match fs::read(model::ini_path(p, name)?) {
        Ok(bytes) => {
            if bytes.len() > 1024 * 1024 {
                return Err("Configuration trop volumineuse.".into());
            }
            String::from_utf8(bytes).map(|s| (true, s)).map_err(|_| {
                "Édition guidée : convertir une copie des fichiers en UTF-8 avant modification."
                    .into()
            })
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok((false, String::new())),
        Err(e) => Err(e.to_string()),
    }
}
fn maximum_level_setting() -> Setting {
    number(
        "MaxWildDinoLevel",
        GUS,
        "Niveau maximum des dinos sauvages",
        "Règles du monde",
        "Plafond des dinos sauvages ordinaires. Agit aussi sur la qualité du butin et certains équilibrages de combat ; les dinos déjà présents gardent leur niveau.",
        (1., 300., 1.),
        150.,
    )
}
pub fn read(p: &Profile) -> Result<Value, String> {
    let (_, gus) = read_file(p, GUS)?;
    let (_, game) = read_file(p, GAME)?;
    let mut fields:Vec<_>=schema().iter().map(|s|{
        let raw=read_value(if s.file==GUS{&gus}else{&game},s.section,s.key);
        let parsed=raw.as_ref().and_then(|v|if s.kind=="boolean" {match v.to_ascii_lowercase().as_str(){"true"|"1"=>Some(json!(true)),"false"|"0"=>Some(json!(false)),_=>None}}else{v.parse::<f64>().ok().filter(|n|n.is_finite()).map(|n|json!(n))});
        let mut field=serde_json::to_value(s).unwrap();field["configured"]=json!(raw.is_some());field["value"]=parsed.clone().unwrap_or_else(||s.default.clone());
        field["warning"]=if raw.is_some()&&parsed.is_none(){json!("Valeur actuelle non reconnue. Elle sera conservée tant que vous ne modifiez pas ce réglage.")}else{Value::Null};field
    }).collect();
    let difficulty_keys = [
        "DifficultyOffset",
        "OverrideOfficialDifficulty",
        "MaxDifficulty",
    ];
    let raw: serde_json::Map<String, Value> = fields
        .iter()
        .filter(|f| difficulty_keys.contains(&f["key"].as_str().unwrap_or_default()))
        .map(|f| (f["key"].as_str().unwrap().to_string(), f["value"].clone()))
        .collect();
    let configured = fields.iter().any(|f| {
        difficulty_keys.contains(&f["key"].as_str().unwrap_or_default()) && f["configured"] == true
    });
    let raw_warning = fields
        .iter()
        .filter(|f| difficulty_keys.contains(&f["key"].as_str().unwrap_or_default()))
        .find_map(|f| f["warning"].as_str().map(str::to_owned));
    let level = crate::simulation::max_wild_level(
        &p.map,
        raw["DifficultyOffset"].as_f64().unwrap_or(1.),
        raw["OverrideOfficialDifficulty"].as_f64().unwrap_or(0.),
        raw["MaxDifficulty"].as_bool().unwrap_or(false),
    );
    let mut maximum = serde_json::to_value(maximum_level_setting()).unwrap();
    maximum["value"] = json!(level.unwrap_or(150));
    maximum["configured"] = json!(configured);
    maximum["virtual"] = json!(true);
    maximum["warning"] = if level.is_none() {
        json!(
            "Difficulté native de cette carte inconnue. Choisissez un niveau maximum pour définir une surcharge explicite."
        )
    } else {
        raw_warning.map_or(Value::Null, Value::String)
    };
    let index = fields
        .iter()
        .position(|f| f["key"] == "DifficultyOffset")
        .unwrap_or(0);
    fields.retain(|f| !difficulty_keys.contains(&f["key"].as_str().unwrap_or_default()));
    fields.insert(index, maximum);
    Ok(
        json!({"fields":fields,"rawDifficulty":raw,"hashes":{"GameUserSettings.ini":hash(gus.as_bytes()),"Game.ini":hash(game.as_bytes())},"presets":presets()}),
    )
}
pub fn validate_changes(changes: &Value) -> Result<Vec<(Setting, String)>, String> {
    let object = changes.as_object().ok_or("Liste de réglages invalide")?;
    if object.is_empty() {
        return Err("Aucun réglage modifié.".into());
    }
    if let Some(level) = object.get("MaxWildDinoLevel") {
        let n = level
            .as_f64()
            .filter(|n| n.is_finite() && *n >= 1. && *n <= 300. && n.fract() == 0.)
            .ok_or("Le niveau maximum doit être un entier entre 1 et 300.")?;
        let mut expanded = object.clone();
        expanded.remove("MaxWildDinoLevel");
        expanded.insert("DifficultyOffset".into(), json!(1));
        expanded.insert("OverrideOfficialDifficulty".into(), json!(n / 30.));
        expanded.insert("MaxDifficulty".into(), json!(false));
        return validate_changes(&Value::Object(expanded));
    }
    let definitions = schema();
    let mut result = vec![];
    for (key, value) in object {
        let s = definitions
            .iter()
            .find(|s| s.key == key)
            .ok_or_else(|| format!("Réglage non autorisé : {key}"))?;
        let text = if s.kind == "boolean" {
            if value
                .as_bool()
                .ok_or("Une case doit contenir true ou false")?
            {
                "True".into()
            } else {
                "False".into()
            }
        } else {
            let n = value
                .as_f64()
                .filter(|n| n.is_finite() && *n >= s.min && *n <= s.max)
                .ok_or_else(|| {
                    format!("{} : valeur attendue entre {} et {}", s.label, s.min, s.max)
                })?;
            n.to_string()
        };
        result.push((s.clone(), text));
    }
    Ok(result)
}
pub fn save(p: &Profile, hashes: &Value, changes: &Value) -> Result<(), String> {
    save_with(p, hashes, changes, model::save_bytes)
}
fn save_with(
    p: &Profile,
    hashes: &Value,
    changes: &Value,
    mut write: impl FnMut(&Path, &[u8]) -> Result<(), String>,
) -> Result<(), String> {
    let validated = validate_changes(changes)?;
    let mut docs = BTreeMap::new();
    for name in [GUS, GAME] {
        let (existed, text) = read_file(p, name)?;
        if hashes[name].as_str() != Some(&hash(text.as_bytes())) {
            return Err(
                "Un fichier INI a changé depuis son ouverture. Rechargez les réglages.".into(),
            );
        }
        docs.insert(name, (existed, text.clone(), text));
    }
    for (s, value) in validated {
        let doc = docs.get_mut(s.file).unwrap();
        doc.2 = patch(&doc.2, s.section, s.key, &value);
    }
    let changed: Vec<_> = docs
        .iter()
        .filter(|(_, (_, before, after))| before != after)
        .collect();
    let backup_id = uuid::Uuid::new_v4();
    for (name, (existed, _, _)) in &changed {
        let path = model::ini_path(p, name)?;
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        if *existed {
            fs::copy(&path, path.with_extension(format!("ini.{backup_id}.bak")))
                .map_err(|e| e.to_string())?;
        }
    }
    let mut written = vec![];
    for (name, (_, _, after)) in &changed {
        let path = model::ini_path(p, name)?;
        if let Err(error) = write(&path, after.as_bytes()) {
            let mut rollback_errors = vec![];
            for prior in written {
                let (existed, before, _) = &docs[prior];
                let path = model::ini_path(p, prior)?;
                let r = if *existed {
                    write(&path, before.as_bytes())
                } else {
                    fs::remove_file(path).map_err(|e| e.to_string())
                };
                if let Err(e) = r {
                    rollback_errors.push(e);
                }
            }
            return Err(if rollback_errors.is_empty() {
                format!(
                    "Échec d’enregistrement ; les modifications précédentes ont été annulées : {error}"
                )
            } else {
                format!(
                    "Échec d’enregistrement et restauration incomplète. Utiliser les copies .{backup_id}.bak : {error}"
                )
            });
        }
        written.push(*name);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn fixture() -> (std::path::PathBuf, Profile) {
        let root = std::env::temp_dir().join(format!("ark-tuning-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let p = Profile {
            id: "tuning-test".into(),
            name: "test".into(),
            directory: root.join("server").to_string_lossy().into(),
            map: "TheIsland_WP".into(),
            game_port: 7777,
            rcon_port: 27020,
            mods: String::new(),
        };
        (root, p)
    }
    fn write_fixture(p: &Profile) {
        let path = model::ini_path(p, GUS).unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path,"[ServerSettings]\r\nServerAdminPassword=leave-this-alone\r\nTamingSpeedMultiplier=2\r\nModSetting=abc\r\n").unwrap();
        fs::write(model::ini_path(p,GAME).unwrap(),"[/Script/ShooterGame.ShooterGameMode]\nOverrideEngramEntries=A\nOverrideEngramEntries=B\nBabyMatureSpeedMultiplier=1\n").unwrap();
    }
    #[test]
    fn guided_edit_changes_only_requested_values_across_both_files() {
        let (root, p) = fixture();
        write_fixture(&p);
        let before = read(&p).unwrap();
        assert!(before["hashes"][GUS].is_string());
        save(
            &p,
            &before["hashes"],
            &json!({"TamingSpeedMultiplier":5,"BabyMatureSpeedMultiplier":30}),
        )
        .unwrap();
        let gus = fs::read_to_string(model::ini_path(&p, GUS).unwrap()).unwrap();
        let game = fs::read_to_string(model::ini_path(&p, GAME).unwrap()).unwrap();
        assert!(gus.contains("ServerAdminPassword=leave-this-alone\r\n"));
        assert!(gus.contains("ModSetting=abc\r\n"));
        assert!(!gus.contains("XPMultiplier"));
        assert!(game.contains("OverrideEngramEntries=A\nOverrideEngramEntries=B\n"));
        assert_eq!(
            read_value(&game, MODE, "BabyMatureSpeedMultiplier").as_deref(),
            Some("30")
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn concurrent_file_edit_prevents_all_writes() {
        let (root, p) = fixture();
        write_fixture(&p);
        let before = read(&p).unwrap();
        let game_path = model::ini_path(&p, GAME).unwrap();
        fs::write(&game_path, "external change").unwrap();
        let gus_before = fs::read(model::ini_path(&p, GUS).unwrap()).unwrap();
        assert!(
            save(
                &p,
                &before["hashes"],
                &json!({"TamingSpeedMultiplier":5,"BabyMatureSpeedMultiplier":30})
            )
            .is_err()
        );
        assert_eq!(fs::read(&game_path).unwrap(), b"external change");
        assert_eq!(
            fs::read(model::ini_path(&p, GUS).unwrap()).unwrap(),
            gus_before
        );
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn second_file_write_failure_rolls_back_first_file() {
        let (root, p) = fixture();
        write_fixture(&p);
        let before = read(&p).unwrap();
        let game_path = model::ini_path(&p, GAME).unwrap();
        let gus_path = model::ini_path(&p, GUS).unwrap();
        let game_before = fs::read(&game_path).unwrap();
        let gus_before = fs::read(&gus_path).unwrap();
        let mut writes = 0;
        let result = save_with(
            &p,
            &before["hashes"],
            &json!({"TamingSpeedMultiplier":5,"BabyMatureSpeedMultiplier":30}),
            |path, bytes| {
                writes += 1;
                if writes == 2 {
                    return Err("simulated disk write failure".into());
                }
                model::save_bytes(path, bytes)
            },
        );
        assert!(result.is_err());
        assert_eq!(writes, 3);
        assert_eq!(fs::read(game_path).unwrap(), game_before);
        assert_eq!(fs::read(gus_path).unwrap(), gus_before);
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn missing_files_are_created_without_materializing_all_defaults() {
        let (root, p) = fixture();
        let before = read(&p).unwrap();
        assert!(
            before["fields"]
                .as_array()
                .unwrap()
                .iter()
                .all(|f| f["configured"] == false)
        );
        save(&p, &before["hashes"], &json!({"ServerPVE":true})).unwrap();
        assert_eq!(
            fs::read_to_string(model::ini_path(&p, GUS).unwrap()).unwrap(),
            "[ServerSettings]\nServerPVE=True\n"
        );
        assert!(!model::ini_path(&p, GAME).unwrap().exists());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn patch_preserves_other_keys_duplicates_comments_bom_and_crlf() {
        let input = "\u{feff}[ServerSettings]\r\n; notes\r\nTamingSpeedMultiplier = 2 ; garder\r\n[Other]\r\nTamingSpeedMultiplier=99\r\nUnknown=x\r\nOverrideEngramEntries=A\r\nOverrideEngramEntries=B\r\n";
        let output = patch(input, SERVER, "TamingSpeedMultiplier", "5");
        assert!(output.contains("TamingSpeedMultiplier = 5 ; garder\r\n"));
        assert!(output.contains("[Other]\r\nTamingSpeedMultiplier=99"));
        assert!(output.ends_with("OverrideEngramEntries=A\r\nOverrideEngramEntries=B\r\n"));
        assert!(output.starts_with('\u{feff}'));
    }
    #[test]
    fn patch_inserts_inside_target_section_and_handles_repeated_scalar() {
        assert_eq!(
            patch(
                "[ServerSettings]\nX=1\n[Other]\nY=2",
                SERVER,
                "ServerPVE",
                "True"
            ),
            "[ServerSettings]\nX=1\nServerPVE=True\n[Other]\nY=2"
        );
        assert_eq!(
            patch("[ServerSettings]\nX=1\nX=2", SERVER, "X", "3"),
            "[ServerSettings]\nX=3\nX=3"
        );
    }
    #[test]
    fn every_preset_is_valid_and_keeps_native_solo_bonuses_off() {
        for p in presets().as_array().unwrap() {
            assert!(!validate_changes(&p["values"]).unwrap().is_empty());
            assert_eq!(p["values"]["bUseSingleplayerSettings"], false);
            assert_eq!(p["values"]["ServerPVE"], true);
        }
        assert!(validate_changes(&json!({"ServerAdminPassword":"bad"})).is_err());
        assert!(validate_changes(&json!({"XPMultiplier":-1})).is_err());
    }
    #[test]
    fn advanced_stats_and_speed_options_save_in_the_correct_sections() {
        let (root, p) = fixture();
        write_fixture(&p);
        let before = read(&p).unwrap();
        save(&p,&before["hashes"],&json!({"PerLevelStatsMultiplier_Player[1]":2,"PerLevelStatsMultiplier_DinoTamed[1]":2,"bAllowSpeedLeveling":true,"bAllowFlyerSpeedLeveling":true,"MaxDifficulty":true})).unwrap();
        let game = fs::read_to_string(model::ini_path(&p, GAME).unwrap()).unwrap();
        assert_eq!(
            read_value(&game, MODE, "PerLevelStatsMultiplier_Player[1]").as_deref(),
            Some("2")
        );
        assert_eq!(
            read_value(&game, MODE, "bAllowFlyerSpeedLeveling").as_deref(),
            Some("True")
        );
        assert_eq!(
            read_value(&game, "ShooterGameMode_Options", "MaxDifficulty").as_deref(),
            Some("True")
        );
        assert!(game.contains("OverrideEngramEntries=A\nOverrideEngramEntries=B"));
        let keys: std::collections::HashSet<_> = schema().iter().map(|s| s.key).collect();
        assert_eq!(keys.len(), schema().len());
        fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn single_level_control_migrates_and_saves_native_difficulty_safely() {
        let (root, p) = fixture();
        write_fixture(&p);
        let gus = model::ini_path(&p, GUS).unwrap();
        let game = model::ini_path(&p, GAME).unwrap();
        fs::write(
            &gus,
            "[ServerSettings]\nDifficultyOffset=0.7\nOverrideOfficialDifficulty=10\nUnknown=keep\n",
        )
        .unwrap();
        fs::write(&game,"[ShooterGameMode_Options]\nMaxDifficulty=True\n[/Script/ShooterGame.ShooterGameMode]\nOther=keep\n").unwrap();
        let before = read(&p).unwrap();
        let fields = before["fields"].as_array().unwrap();
        assert_eq!(
            fields
                .iter()
                .find(|f| f["key"] == "MaxWildDinoLevel")
                .unwrap()["value"],
            150
        );
        assert!(!fields.iter().any(|f| f["key"] == "DifficultyOffset"
            || f["key"] == "OverrideOfficialDifficulty"
            || f["key"] == "MaxDifficulty"));
        let untouched = fs::read(&gus).unwrap();
        save(
            &p,
            &before["hashes"],
            &json!({"PerLevelStatsMultiplier_Player[1]":2}),
        )
        .unwrap();
        assert_eq!(fs::read(&gus).unwrap(), untouched);
        let current = read(&p).unwrap();
        save(&p, &current["hashes"], &json!({"MaxWildDinoLevel":180})).unwrap();
        let gus_text = fs::read_to_string(&gus).unwrap();
        let game_text = fs::read_to_string(&game).unwrap();
        assert_eq!(
            read_value(&gus_text, SERVER, "DifficultyOffset").as_deref(),
            Some("1")
        );
        assert_eq!(
            read_value(&gus_text, SERVER, "OverrideOfficialDifficulty").as_deref(),
            Some("6")
        );
        assert_eq!(
            read_value(&game_text, "ShooterGameMode_Options", "MaxDifficulty").as_deref(),
            Some("False")
        );
        assert!(gus_text.contains("Unknown=keep"));
        assert!(game_text.contains("Other=keep"));
        assert!(!gus_text.contains("MaxWildDinoLevel"));
        assert!(validate_changes(&json!({"MaxWildDinoLevel":150.5})).is_err());
        assert!(validate_changes(&json!({"MaxWildDinoLevel":301})).is_err());
        fs::remove_dir_all(root).unwrap();
    }
}
