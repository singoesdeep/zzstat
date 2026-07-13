use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponDef {
    pub id: u32,
    pub r#type: u32,
    pub is_special: bool,
    pub attack_values: serde_json::Value, // Either flat [min_magic, max_magic, min_attack, max_attack] or 2D array if special
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
        self.data[10].as_f64().unwrap_or(0.0) // Index 10 is typically defense in monsterData (from Calculator.js)
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
    pub constants: ConstantsDef,
}

impl Metin2Data {
    pub fn load(base_dir: impl AsRef<Path>) -> Result<Self, Box<dyn std::error::Error>> {
        let base = base_dir.as_ref();
        
        let w_str = fs::read_to_string(base.join("weapons.json"))?;
        let weapons_list: Vec<WeaponDef> = serde_json::from_str(&w_str)?;
        let mut weapons = HashMap::new();
        for w in weapons_list {
            weapons.insert(w.id, w);
        }
        
        let m_str = fs::read_to_string(base.join("monsters.json"))?;
        let monsters_list: Vec<MonsterDef> = serde_json::from_str(&m_str)?;
        let mut monsters = HashMap::new();
        for m in monsters_list {
            monsters.insert(m.id, m);
        }
        
        let c_str = fs::read_to_string(base.join("constants.json"))?;
        let constants: ConstantsDef = serde_json::from_str(&c_str)?;
        
        Ok(Self {
            weapons,
            monsters,
            constants,
        })
    }
}
