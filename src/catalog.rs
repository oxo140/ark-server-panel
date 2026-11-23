use serde::Serialize;

#[derive(Serialize)]
pub struct Map {
    pub id: &'static str,
    pub name: &'static str,
    pub content: &'static str,
}
pub const MAPS: &[Map] = &[
    Map {
        id: "TheIsland_WP",
        name: "The Island",
        content: "Jeu de base",
    },
    Map {
        id: "ScorchedEarth_WP",
        name: "Scorched Earth",
        content: "Extension",
    },
    Map {
        id: "TheCenter_WP",
        name: "The Center",
        content: "Carte officielle",
    },
    Map {
        id: "Aberration_WP",
        name: "Aberration",
        content: "Extension",
    },
    Map {
        id: "Extinction_WP",
        name: "Extinction",
        content: "Extension",
    },
    Map {
        id: "Ragnarok_WP",
        name: "Ragnarok",
        content: "Carte officielle",
    },
    Map {
        id: "Valguero_WP",
        name: "Valguero",
        content: "Carte officielle",
    },
    Map {
        id: "Astraeos_WP",
        name: "Astraeos",
        content: "Contenu premium",
    },
    Map {
        id: "LostColony_WP",
        name: "Lost Colony",
        content: "Extension payante",
    },
    Map {
        id: "Genesis_WP",
        name: "Genesis: Part 1",
        content: "Extension",
    },
];

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn available_asa_maps_have_distinct_names_and_launch_codes() {
        let ids: std::collections::HashSet<_> = MAPS.iter().map(|m| m.id).collect();
        assert_eq!(ids.len(), MAPS.len());
        assert!(
            MAPS.iter()
                .all(|m| !m.name.contains("_WP") && m.id.ends_with("_WP"))
        );
        for id in ["Valguero_WP", "LostColony_WP", "Astraeos_WP", "Genesis_WP"] {
            assert!(ids.contains(id));
        }
        assert!(!MAPS.iter().any(|m| m.name == "Dragontopia"));
    }
}
