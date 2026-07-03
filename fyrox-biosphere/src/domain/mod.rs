use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Domain {
    Narrative,
    Music,
    Software,
    Agent,
    Visual,
    /// Real-world geographic scaffold with narrative overlay.
    /// Genesis=Planet, Mythos=Continent/Region, Container=Country/City, Capsule=Location/Character/Event
    Earth,
    Custom(String),
}

impl Domain {
    pub fn genesis_label(&self) -> &str {
        match self {
            Domain::Narrative => "Story world / Universe",
            Domain::Music => "Artist / Band / Musical persona",
            Domain::Software => "Application / Platform",
            Domain::Agent => "Simulation / World",
            Domain::Visual => "Design system / Brand",
            Domain::Earth => "Earth / Planet",
            Domain::Custom(_) => "Custom Genesis",
        }
    }

    pub fn mythos_label(&self) -> &str {
        match self {
            Domain::Narrative => "Story arc / Region / Era",
            Domain::Music => "Album series / Season / World arc",
            Domain::Software => "Module / Department",
            Domain::Agent => "Guild / Department",
            Domain::Visual => "Style domain",
            Domain::Earth => "Continent / Major Region",
            Domain::Custom(_) => "Custom Mythos",
        }
    }

    pub fn container_label(&self) -> &str {
        match self {
            Domain::Narrative => "Scene collection / Culture / Faction",
            Domain::Music => "Album / EP / Compilation",
            Domain::Software => "Feature set / Sub-system",
            Domain::Agent => "Squad / Project",
            Domain::Visual => "Component family",
            Domain::Earth => "Country / City-State / Province",
            Domain::Custom(_) => "Custom Container",
        }
    }

    pub fn capsule_label(&self) -> &str {
        match self {
            Domain::Narrative => "Character / Event / Object / Lore entry",
            Domain::Music => "Individual track / Song",
            Domain::Software => "Component / Function / Node",
            Domain::Agent => "Individual Actor / Tool",
            Domain::Visual => "Individual token / Asset",
            Domain::Earth => "Location / Character / Event / Landmark",
            Domain::Custom(_) => "Custom Capsule",
        }
    }

    /// Returns true if this domain uses real-world geographic data as its scaffold.
    pub fn is_geographic(&self) -> bool {
        matches!(self, Domain::Earth)
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Domain::Narrative => write!(f, "Narrative"),
            Domain::Music => write!(f, "Music"),
            Domain::Software => write!(f, "Software"),
            Domain::Agent => write!(f, "Agent"),
            Domain::Visual => write!(f, "Visual"),
            Domain::Earth => write!(f, "Earth"),
            Domain::Custom(name) => write!(f, "{name}"),
        }
    }
}

impl std::str::FromStr for Domain {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s.to_lowercase().as_str() {
            "narrative" => Domain::Narrative,
            "music" => Domain::Music,
            "software" => Domain::Software,
            "agent" => Domain::Agent,
            "visual" => Domain::Visual,
            "earth" => Domain::Earth,
            _ => Domain::Custom(s.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn domain_parsing() {
        assert_eq!("narrative".parse::<Domain>().unwrap(), Domain::Narrative);
        assert_eq!("Music".parse::<Domain>().unwrap(), Domain::Music);
        assert_eq!(
            "custom_thing".parse::<Domain>().unwrap(),
            Domain::Custom("custom_thing".to_string())
        );
    }

    #[test]
    fn domain_labels() {
        assert_eq!(Domain::Agent.capsule_label(), "Individual Actor / Tool");
        assert_eq!(Domain::Music.container_label(), "Album / EP / Compilation");
    }
}
