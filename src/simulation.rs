//! Advisory calculations only: never writes INI or applies changes to a running server.
use crate::{config, model::Profile};
use serde_json::{Map, Value, json};

fn duration(seconds: f64) -> String {
    if !seconds.is_finite() || seconds < 0. {
        return "Indéterminé".into();
    }
    let s = seconds.round() as u64;
    let (d, h, m) = (s / 86400, s % 86400 / 3600, s % 3600 / 60);
    if d > 0 {
        format!("{d} j {h} h {m} min")
    } else if h > 0 {
        format!("{h} h {m} min {} s", s % 60)
    } else {
        format!("{m} min {} s", s % 60)
    }
}
fn reference(v: &Value, key: &str, default: f64) -> f64 {
    v[key]
        .as_f64()
        .filter(|n| n.is_finite() && *n > 0. && *n <= 1_000_000.)
        .unwrap_or(default)
}
fn map_difficulty(map: &str) -> Option<f64> {
    match map {
        "TheIsland_WP" | "ScorchedEarth_WP" | "Aberration_WP" => Some(4.),
        "TheCenter_WP" | "Extinction_WP" | "Ragnarok_WP" | "Valguero_WP" | "Astraeos_WP"
        | "LostColony_WP" | "Genesis_WP" => Some(5.),
        _ => None,
    }
}
pub fn preview(p: &Profile, changes: &Value, refs: &Value) -> Result<Value, String> {
    if changes.as_object().is_some_and(|m| !m.is_empty()) {
        config::validate_changes(changes)?;
    } else if !changes.is_null() && !changes.is_object() {
        return Err("Réglages de simulation invalides.".into());
    }
    let loaded = config::read(p)?;
    let mut values: Map<String, Value> = loaded["fields"]
        .as_array()
        .unwrap()
        .iter()
        .map(|f| (f["key"].as_str().unwrap().into(), f["value"].clone()))
        .collect();
    if let Some(c) = changes.as_object() {
        values.extend(c.clone());
    }
    Ok(calculate(&p.map, &values, refs))
}
fn calculate(map: &str, v: &Map<String, Value>, refs: &Value) -> Value {
    let get = |k: &str, d: f64| v.get(k).and_then(Value::as_f64).unwrap_or(d);
    let flag = |k: &str| v.get(k).and_then(Value::as_bool).unwrap_or(false);
    let sp = flag("bUseSingleplayerSettings");
    let mut tips = Map::new();
    let mut warnings = Vec::new();
    let base = map_difficulty(map);
    let offset = get("DifficultyOffset", 1.);
    let over = get("OverrideOfficialDifficulty", 0.);
    let diff = if flag("MaxDifficulty") {
        warnings.push("Difficulté maximale forcée activée : un plafond personnalisé peut être neutralisé ; The Island possède des exceptions. Désactivez cette case pour utiliser uniquement OverrideOfficialDifficulty.".to_string());
        Some(5.)
    } else if over > 0. {
        Some(over)
    } else {
        base.map(|b| offset * (b - 0.5) + 0.5)
    };
    let max = diff.map(|d| (d * 30.).round().max(1.) as u64);
    let difficulty=match (diff,max){(Some(d),Some(max))=>format!("Dinos sauvages ordinaires : niveau maximum {max} · difficulté effective {d:.2}. Tek : environ {} ; Wyvernes / Rock Drakes : environ {} lorsqu’ils sont présents. Hors grottes, boss, missions et mods. Les dinos déjà présents gardent leur niveau.",(d*36.).round() as u64,(d*38.).round() as u64),_=>"Carte personnalisée : difficulté native inconnue. Renseignez OverrideOfficialDifficulty pour calculer un plafond fiable.".into()};
    for k in [
        "DifficultyOffset",
        "OverrideOfficialDifficulty",
        "MaxDifficulty",
    ] {
        tips.insert(k.into(), json!(difficulty));
    }
    if over > 0. && !flag("MaxDifficulty") {
        tips.insert("DifficultyOffset".into(),json!(format!("{difficulty} OverrideOfficialDifficulty={over} est prioritaire : déplacer ce curseur ne change pas le plafond tant que la surcharge reste active.")));
    }
    if sp {
        warnings.push("Bonus solo natifs actifs : les calculs incluent les bonus de vitesse solo usuels. Les mods, événements et effets de monde ne sont pas inclus.".into());
    }
    if flag("bAllowFlyerSpeedLeveling") && !flag("bAllowSpeedLeveling") {
        warnings.push("Vitesse des volants activée mais vitesse générale désactivée : activez les deux cases pour ASA.".into());
    }
    let tame = get("TamingSpeedMultiplier", 1.) * if sp { 2.5 } else { 1. };
    let drain = get("DinoCharacterFoodDrainMultiplier", 1.);
    // Rex affinity 3450 + 150*level; current base affinity rates include the
    // permanent x4 official-rate rebasing. Unconscious drain from extracted data.
    let affinity = 3450. + 150. * 150.;
    let kibble = (affinity / (400. * 4. * tame)).ceil();
    let meat = (affinity / (50. * 4. * tame)).ceil();
    let food_per_second = 0.002314 * 180.0634 * drain;
    let kibble_time = kibble * 53.28 / food_per_second;
    let meat_time = meat * 50. / food_per_second;
    let tame_text = format!(
        "Rex sauvage 150, inconscient, sans dégâts : environ {kibble:.0} croquettes exceptionnelles en {}, ou {meat:.0} viandes crues en {}. Estimation à nourriture disponible dès le départ ; pourrissement et narcotiques non comptés.",
        duration(kibble_time),
        duration(meat_time)
    );
    for k in ["TamingSpeedMultiplier", "DinoCharacterFoodDrainMultiplier"] {
        tips.insert(k.into(), json!(tame_text));
    }
    let mating_interval = get("MatingIntervalMultiplier", 1.) * if sp { 0.125 } else { 1. };
    let mating = 120. / get("MatingSpeedMultiplier", 1.);
    let egg = 17998.5601 / (get("EggHatchSpeedMultiplier", 1.) * if sp { 10. } else { 1. });
    let mature = 333333.333 / (get("BabyMatureSpeedMultiplier", 1.) * if sp { 36.799 } else { 1. });
    let care = 28800. * get("BabyCuddleIntervalMultiplier", 1.) * if sp { 0.167 } else { 1. };
    let interval_text = format!(
        "Rex femelle : {} à {} entre deux reproductions (tirage aléatoire ; les délais déjà en cours peuvent être conservés).",
        duration(64800. * mating_interval),
        duration(172800. * mating_interval)
    );
    tips.insert("MatingIntervalMultiplier".into(), json!(interval_text));
    tips.insert(
        "MatingSpeedMultiplier".into(),
        json!(format!(
            "Rex : environ {} d’accouplement, si les parents restent à portée.",
            duration(mating)
        )),
    );
    tips.insert(
        "EggHatchSpeedMultiplier".into(),
        json!(format!(
            "Œuf de Rex : environ {} à température correcte (32–34 °C).",
            duration(egg)
        )),
    );
    tips.insert("BabyMatureSpeedMultiplier".into(),json!(format!("Rex : adulte après {} ; phase bébé environ {}. Le niveau 150 ne change pas ces durées.",duration(mature),duration(mature*0.1))));
    let nominal = (mature / care).floor().max(1.);
    let fraction = (get("BabyImprintAmountMultiplier", 1.) / nominal).min(1.);
    let required = (1. / fraction - 1e-10).ceil();
    let available = ((mature - 0.001) / care).floor().max(0.);
    let max_imprint = (available * fraction * 100.).min(100.);
    let possible = required * care < mature && max_imprint >= 100. - 1e-7;
    let slack = (mature - required * care).max(0.);
    let imprint = format!(
        "Rex : un soin toutes les {}, +{:.2} % par soin ; {:.0} soins pour 100 %. Dernier soin à {}, adulte à {}. {}",
        duration(care),
        fraction * 100.,
        required,
        duration(required * care),
        duration(mature),
        if possible {
            format!(
                "100 % théoriquement possible ; marge cumulée de retard : {}.",
                duration(slack)
            )
        } else {
            format!(
                "ATTENTION : 100 % impossible avant l’âge adulte ; maximum théorique {:.1} %. Réduisez l’intervalle des soins, ralentissez la maturation ou augmentez le gain par soin.",
                max_imprint
            )
        }
    );
    for k in [
        "BabyCuddleIntervalMultiplier",
        "BabyImprintAmountMultiplier",
    ] {
        tips.insert(k.into(), json!(imprint));
    }
    if !possible {
        warnings.push(imprint.clone());
    } else if slack < 60. {
        warnings.push("Empreinte Rex : marge inférieure à une minute ; prévoyez plus de temps pour les actions et les retards.".into());
    }
    let baby_food = 0.002314 * 510. * drain * get("BabyFoodConsumptionSpeedMultiplier", 1.) * 60.;
    tips.insert("BabyFoodConsumptionSpeedMultiplier".into(),json!(format!("Rex nouveau-né : environ {baby_food:.1} points de nourriture/minute à la naissance ; le débit diminue avec l’âge. Pas une quantité de viande à prévoir : statistiques, gaspillage et pourrissement changent le besoin réel.")));
    let hits = reference(refs, "rockHits", 10.);
    let yield_base = reference(refs, "rockYield", 100.);
    let hp = get("HarvestHealthMultiplier", 1.);
    let amount = get("HarvestAmountMultiplier", 1.);
    let damage = get("PlayerHarvestingDamageMultiplier", 1.);
    let harvest_hits = (hits * hp / damage).ceil().max(1.);
    let harvest_yield = yield_base * hp * amount;
    let harvest = format!(
        "Exemple simplifié, même rocher et outil : référence {hits:.0} coups / {yield_base:.0} ressources à ×1 → environ {harvest_hits:.0} coups et {harvest_yield:.0} ressources au total. Référence personnalisable ; ce n’est pas une mesure universelle des rochers. Butin aléatoire, outil, dégâts et sur-récolte peuvent changer le résultat."
    );
    for k in [
        "HarvestAmountMultiplier",
        "HarvestHealthMultiplier",
        "PlayerHarvestingDamageMultiplier",
    ] {
        tips.insert(k.into(), json!(harvest));
    }
    let player_stamina = 10. * get("PerLevelStatsMultiplier_Player[1]", 1.);
    let rex_stamina = reference(refs, "rexStamina", 420.);
    let rex_gain = rex_stamina * 0.1 * get("PerLevelStatsMultiplier_DinoTamed[1]", 1.);
    tips.insert(
        "PerLevelStatsMultiplier_Player[1]".into(),
        json!(format!(
            "Joueur : environ +{player_stamina:.1} endurance par point investi (base +10 à ×1)."
        )),
    );
    tips.insert("PerLevelStatsMultiplier_DinoTamed[1]".into(),json!(format!("Rex avec {rex_stamina:.0} endurance à l’apprivoisement : +{rex_gain:.1} par point investi. Le total d’un Rex 150 varie selon la répartition aléatoire de ses points.")));
    json!({"map":map,"maxWildLevel":max,"difficulty":diff,"tips":tips,"warnings":warnings,"cards":[{"title":"Niveaux sauvages","text":difficulty},{"title":"Rex 150 · apprivoisement","text":tame_text},{"title":"Rex · reproduction","text":format!("Accouplement {} · incubation {} · maturation {}. Intervalle femelle : {}–{}.",duration(mating),duration(egg),duration(mature),duration(64800.*mating_interval),duration(172800.*mating_interval))},{"title":"Empreinte du Rex","text":imprint},{"title":"Rocher · exemple réglable","text":harvest}],"imprint":{"canReach100":possible,"careSeconds":care,"matureSeconds":mature,"gainPercent":fraction*100.,"requiredCares":required,"availableCares":available,"maxPercent":max_imprint,"slackSeconds":slack},"taming":{"kibble":kibble,"kibbleSeconds":kibble_time,"rawMeat":meat,"rawMeatSeconds":meat_time},"harvest":{"hits":harvest_hits,"yield":harvest_yield},"assumptions":"Prévision vanilla ASA, Rex normal, actions immédiates ; hors mods, événements, buffs, traits et créatures spéciales. Les résultats sont des estimations, pas une lecture des dinos du serveur.","sources":[{"label":"Difficulté ARK","url":"https://ark.wiki.gg/wiki/Difficulty"},{"label":"Données extraites ARK Smart Breeding","url":"https://github.com/cadon/ARKStatsExtractor/tree/master/ARKBreedingStats/json"},{"label":"Rex ASA et reproduction (DevKit)","url":"https://hypernatrema.com/ark/browse/creatures/rex"},{"label":"Options vitesse ASA","url":"https://ark.wiki.gg/wiki/Server_configuration"}]})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn difficulty_uses_override_instead_of_map_offset() {
        let mut v = Map::new();
        v.insert("DifficultyOffset".into(), json!(1));
        assert_eq!(
            calculate("TheIsland_WP", &v, &Value::Null)["maxWildLevel"],
            120
        );
        assert_eq!(
            calculate("Ragnarok_WP", &v, &Value::Null)["maxWildLevel"],
            150
        );
        v.insert("OverrideOfficialDifficulty".into(), json!(10));
        v.insert("DifficultyOffset".into(), json!(0));
        assert_eq!(
            calculate("TheIsland_WP", &v, &Value::Null)["maxWildLevel"],
            300
        );
    }
    #[test]
    fn imprint_warns_when_first_care_is_after_adulthood() {
        let mut v = Map::new();
        v.insert("BabyMatureSpeedMultiplier".into(), json!(200));
        let r = calculate("TheIsland_WP", &v, &Value::Null);
        assert_eq!(r["imprint"]["canReach100"], false);
        assert_eq!(r["imprint"]["maxPercent"], 0.);
        v.insert("BabyCuddleIntervalMultiplier".into(), json!(0.001));
        assert_eq!(
            calculate("TheIsland_WP", &v, &Value::Null)["imprint"]["canReach100"],
            true
        );
    }
    #[test]
    fn default_rex_and_rounded_meals_match_reference() {
        let mut v = Map::new();
        let r = calculate("TheIsland_WP", &v, &Value::Null);
        assert_eq!(r["taming"]["kibble"], 17.);
        assert_eq!(r["taming"]["rawMeat"], 130.);
        assert_eq!(r["imprint"]["requiredCares"], 11.);
        v.insert("DinoCharacterFoodDrainMultiplier".into(), json!(2));
        let faster = calculate("TheIsland_WP", &v, &Value::Null);
        assert_eq!(faster["taming"]["kibble"], 17.);
        assert_eq!(
            faster["taming"]["kibbleSeconds"].as_f64().unwrap() * 2.,
            r["taming"]["kibbleSeconds"].as_f64().unwrap()
        );
    }
    #[test]
    fn harvest_example_combines_amount_and_health() {
        let mut v = Map::new();
        v.insert("HarvestAmountMultiplier".into(), json!(3));
        v.insert("HarvestHealthMultiplier".into(), json!(2));
        let r = calculate("TheIsland_WP", &v, &json!({"rockHits":10,"rockYield":100}));
        assert_eq!(r["harvest"]["hits"], 20.);
        assert_eq!(r["harvest"]["yield"], 600.);
    }
}
