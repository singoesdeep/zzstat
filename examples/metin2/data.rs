use bevy::asset::{io::Reader, Asset, AssetLoader, AsyncReadExt, LoadContext};
use bevy::reflect::TypePath;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponDef {
    pub id: u32,
    pub r#type: u32,
    pub is_special: bool,
    pub attack_values: serde_json::Value,
    pub growth: Vec<f64>,
}

impl WeaponDef {
    pub fn get_attack_values(&self, upgrade: usize) -> (f64, f64) {
        let upgrade = upgrade.min(self.growth.len() - 1);
        let vals = if self.is_special {
            &self.attack_values[upgrade]
        } else {
            &self.attack_values
        };

        let min_att = vals[2].as_f64().unwrap_or(0.0);
        let max_att = vals[3].as_f64().unwrap_or(0.0);

        let growth = self.growth[upgrade];
        (min_att + growth, max_att + growth)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonsterDef {
    pub id: u32,
    pub name: String,
    pub data: Vec<serde_json::Value>,
}

impl MonsterDef {
    pub fn level(&self) -> f64 {
        self.data[3].as_f64().unwrap_or(1.0)
    }

    pub fn defense(&self) -> f64 {
        self.data[10].as_f64().unwrap_or(0.0)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConstantsDef {
    #[serde(rename = "polymorphPowerTable")]
    pub polymorph_power_table: Vec<f64>,
    #[serde(rename = "skillPowerTable")]
    pub skill_power_table: Vec<f64>,
    #[serde(rename = "allowedWeaponsPerRace")]
    pub allowed_weapons_per_race: HashMap<String, Vec<u32>>,
}

pub struct Metin2Data {
    pub weapons: HashMap<u32, WeaponDef>,
    pub monsters: HashMap<u32, MonsterDef>,
    #[allow(dead_code)]
    pub constants: ConstantsDef,
}

// --- Bevy Asset Integration ---

#[derive(Asset, TypePath, Debug)]
pub struct JsonAsset(pub String);

#[derive(Default)]
pub struct JsonLoader;

impl AssetLoader for JsonLoader {
    type Asset = JsonAsset;
    type Settings = ();
    type Error = std::io::Error;

    async fn load<'a>(
        &'a self,
        reader: &'a mut Reader<'_>,
        _settings: &'a (),
        _load_context: &'a mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await?;
        let str = String::from_utf8(bytes)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(JsonAsset(str))
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}
