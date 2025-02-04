use std::fs;

use serde::Deserialize;
use serde_json::from_str;

use crate::{
    components::{AttackBonus, Consumable, Equipable},
    items::{ItemID, ItemInfo},
};

pub struct ItemDatabase {
    data: Vec<ItemInfo>,
}

#[derive(Deserialize)]
pub struct RawItemDatabase {
    data: Vec<RawItemInfo>,
}

impl ItemDatabase {
    pub(crate) fn empty() -> Self {
        Self { data: Vec::new() }
    }

    pub fn load() -> Self {
        let contents: String =
            fs::read_to_string("raws/items.json").expect("Unable to find items.json at `raws/items.json`");
        let raw_info_db: RawItemDatabase = from_str(&contents).expect("Bad JSON in items.json fix it");
        ItemDatabase { data: raw_info_db.data.iter().map(ItemInfo::from_raw).collect() }
    }

    pub fn get_by_name(&self, name: &str) -> Option<&ItemInfo> {
        self.data.iter().find(|i| i.name.eq(name))
    }

    /// Gets the entity by name without ensuring it exists.
    /// This could by panic but can be used when certain a name would exist for an item.
    pub fn get_by_name_unchecked(&self, name: &String) -> &ItemInfo {
        self.data.iter().find(|i| i.name.eq(name)).unwrap()
    }

    pub fn get_by_id(&self, id: ItemID) -> Option<&ItemInfo> {
        self.data.iter().find(|i| i.identifier == id)
    }
}

#[derive(Deserialize)]
pub struct RawItemInfo {
    /// Unique id to find the item's static data
    pub identifier: ItemID,
    pub name: String,
    pub examine_text: String,
    pub atlas_index: u8,
    pub fg: (u8, u8, u8),
    pub pickup_text: Option<String>,
    pub equipable: Option<String>,
    pub attack_bonus: Option<usize>,
    pub consumable: Option<RawConsumable>,
}

#[derive(Deserialize, Clone)]
pub struct RawConsumable {
    pub effect: String,
    pub amount: Option<usize>,
}

impl ItemInfo {
    fn from_raw(value: &RawItemInfo) -> Self {
        Self {
            identifier: value.identifier,
            name: value.name.clone(),
            examine_text: value.examine_text.clone(),
            atlas_index: value.atlas_index,
            fg: value.fg,
            pickup_text: value.pickup_text.clone(),
            equipable: value.equipable.clone().map(|e| Equipable::from_str(&e)),
            attack_bonus: value.attack_bonus.map(|bonus| AttackBonus(bonus as i32)),
            consumable: value.consumable.clone().map(|rc| Consumable::from_str(&rc.effect, rc.amount.unwrap())),
        }
    }
}
