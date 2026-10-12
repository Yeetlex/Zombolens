//! Project Zomboid build 42.21 messages: the local character's state.
//! Message = 0x86 (RakNet ID_USER_PACKET_ENUM), type u16 BE, body. Types are
//! the ordinals of the game's PacketTypes$PacketType enum; numbers big-endian.

use crate::net::Endpoint;
use serde::Serialize;
use std::collections::HashMap;

/// Packet type ids of build 42.21 (PacketTypes$PacketType ordinals).
pub const PLAYER_UPDATE_RELIABLE: u16 = 13;
pub const PLAYER_DAMAGE: u16 = 164;
pub const PLAYER_STATS: u16 = 165;
pub const PLAYER_HEALTH: u16 = 166;
pub const PLAYER_INJURIES: u16 = 167;
pub const PLAYER_EFFECTS: u16 = 168;

/// CharacterStat in enum order: their order in PlayerStats from byte 6.
/// The last two (zombie fever and infection) are information the game hides
/// from the player: they are never read.
pub const STATS: [&str; 22] = [
    "anger", "boredom", "discomfort", "endurance", "fatigue", "fitness", "food_sickness", "hunger",
    "idleness", "intoxication", "morale", "nicotine_withdrawal", "pain", "panic", "poison", "sanity",
    "sickness", "stress", "temperature", "thirst", "unhappiness", "wetness",
];

pub const BODY_PARTS: [&str; 17] = [
    "Hand_L", "Hand_R", "ForeArm_L", "ForeArm_R", "UpperArm_L", "UpperArm_R", "Torso_Upper", "Torso_Lower",
    "Head", "Neck", "Groin", "UpperLeg_L", "UpperLeg_R", "LowerLeg_L", "LowerLeg_R", "Foot_L", "Foot_R",
];

#[derive(Clone, Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Part {
    pub name: &'static str,
    pub health: f32,
    pub cut: bool,
    pub bitten: bool,
    pub scratched: bool,
    pub bandaged: bool,
    pub bleeding: bool,
    pub deep_wounded: bool,
    pub bandage_life: Option<f32>,
    /// The game's health panel shows an infected wound (not zombie infection).
    pub wound_infected: bool,
    pub glass: bool,
    pub stitched: bool,
    pub fracture_time: f32,
    pub splint: bool,
    pub bullet: bool,
    pub burn_time: f32,
    pub need_burn_wash: bool,
    pub bandage_type: String,
    pub splint_item: String,
    pub wetness: f32,
    pub stiffness: f32,
    pub alcohol_level: f32,
    pub additional_pain: f32,
}

#[derive(Clone, Serialize, Default, Debug)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub server: Option<String>,
    /// Seconds since the last character packet (None: never).
    pub age_s: Option<f64>,
    pub stats: HashMap<&'static str, f32>,
    pub calories: Option<f32>,
    pub proteins: Option<f32>,
    pub lipids: Option<f32>,
    pub carbohydrates: Option<f32>,
    pub weight: Option<f32>,
    pub part_health: Vec<f32>,
    /// Overall health as the game shows it, approximated from the parts.
    pub health: Option<f32>,
    pub parts: Vec<Part>,
    pub max_weight: Option<i32>,
    pub corpse_sickness_rate: Option<f32>,
    pub catch_a_cold: Option<f32>,
    pub has_cold: Option<bool>,
    pub cold_strength: Option<f32>,
    pub walk_speed: Option<f32>,
    pub run_speed: Option<f32>,
    pub injured_idle: Option<f32>,
    pub effects: HashMap<&'static str, f32>,
    pub sleeping_pills: Option<i32>,
    pub position: Option<(f32, f32, i8)>,
    pub direction: Option<f32>,
    pub speed: Option<f32>,
    #[serde(skip)]
    pub last_ms: f64,
}

fn f32_at(m: &[u8], i: usize) -> Option<f32> {
    Some(f32::from_be_bytes(m.get(i..i + 4)?.try_into().ok()?))
}

/// PlayerDamage body: BodyDamage.save(), 17 parts then the main fields.
fn decode_damage(m: &[u8], s: &mut State) -> Option<()> {
    let mut i = 6; // 0x86, type, PlayerID (u16 id + i8 index)
    let max_weight = i32::from_be_bytes(m.get(i..i + 4)?.try_into().ok()?);
    let corpse = f32_at(m, i + 4)?;
    i += 8;
    let mut parts = Vec::with_capacity(17);
    for name in BODY_PARTS {
        let mut b = || -> Option<bool> {
            let v = *m.get(i)?;
            i += 1;
            (v <= 1).then_some(v == 1)
        };
        // fake_infected and infected are hidden from the player: skipped.
        let (cut, bitten, scratched, bandaged, bleeding, deep, _hidden_fake, _hidden_infected) =
            (b()?, b()?, b()?, b()?, b()?, b()?, b()?, b()?);
        let mut f = || -> Option<f32> {
            let v = f32_at(m, i)?;
            i += 4;
            Some(v)
        };
        let health = f()?;
        if !(0.0..=100.0).contains(&health) {
            return None;
        }
        let bandage_life = if bandaged { Some(f()?) } else { None };
        let iw = *m.get(i)? == 1;
        i += 1;
        let mut f = || -> Option<f32> {
            let v = f32_at(m, i)?;
            i += 4;
            Some(v)
        };
        // An infected wound shows in the game's health panel; its level does not.
        if iw {
            f()?;
        }
        let (_cut_t, _bite_t, _scratch_t, _bleed_t, alcohol, add_pain, _deep_t) = (f()?, f()?, f()?, f()?, f()?, f()?, f()?);
        let mut b = || -> Option<bool> {
            let v = *m.get(i)?;
            i += 1;
            (v <= 1).then_some(v == 1)
        };
        let (glass, _bxp, stitched) = (b()?, b()?, b()?);
        let stitch_t = f32_at(m, i)?;
        i += 4;
        let _ = stitch_t;
        let mut b = || -> Option<bool> {
            let v = *m.get(i)?;
            i += 1;
            (v <= 1).then_some(v == 1)
        };
        let (_sxp, _spxp) = (b()?, b()?);
        let fracture_time = f32_at(m, i)?;
        i += 4;
        let splint = *m.get(i)? == 1;
        i += 1;
        if splint {
            i += 4; // splint factor
        }
        let bullet = *m.get(i)? == 1;
        i += 1;
        let burn_time = f32_at(m, i)?;
        i += 4;
        let need_burn_wash = *m.get(i)? == 1;
        i += 1 + 4; // + last burn wash
        let mut text = || -> Option<String> {
            let n = u16::from_be_bytes(m.get(i..i + 2)?.try_into().ok()?) as usize;
            let t = String::from_utf8_lossy(m.get(i + 2..i + 2 + n)?).into_owned();
            i += 2 + n;
            Some(t)
        };
        let splint_item = text()?;
        let bandage_type = text()?;
        let mut f = || -> Option<f32> {
            let v = f32_at(m, i)?;
            i += 4;
            Some(v)
        };
        let (_cut2, wetness, stiffness, _comfrey, _garlic, _plantain) = (f()?, f()?, f()?, f()?, f()?, f()?);
        parts.push(Part {
            name,
            health,
            cut,
            bitten,
            scratched,
            bandaged,
            bleeding,
            deep_wounded: deep,
            bandage_life,
            wound_infected: iw,
            glass,
            stitched,
            fracture_time,
            splint,
            bullet,
            burn_time,
            need_burn_wash,
            bandage_type,
            splint_item,
            wetness,
            stiffness,
            alcohol_level: alcohol,
            additional_pain: add_pain,
        });
    }
    // saveMainFields: catchACold f, hasACold b, coldStrength f, timeToSneeze i32,
    // reduceFakeInfection b, healthFromFoodTimer f, painReduction f,
    // coldReduction f, infectionTime f, infectionMortalityDuration f, coldDamageStage f.
    let catch = f32_at(m, i)?;
    let has_cold = *m.get(i + 4)? == 1;
    let strength = f32_at(m, i + 5)?;
    let j = i + 5 + 4 + 4 + 1;
    let _ = j; // then infection timers (hidden from the player): not read
    s.max_weight = Some(max_weight);
    s.corpse_sickness_rate = Some(corpse);
    s.parts = parts;
    s.catch_a_cold = Some(catch);
    s.has_cold = Some(has_cold);
    s.cold_strength = Some(strength);
    Some(())
}

/// Server detection and the character's state.
#[derive(Default)]
pub struct Tracker {
    pub server: Option<Endpoint>,
    pub state: State,
}

impl Tracker {
    pub fn on_message(&mut self, src: Endpoint, dst: Endpoint, time: f64, m: &[u8]) {
        if m.len() < 3 || m[0] != 0x86 {
            return;
        }
        let ty = u16::from_be_bytes([m[1], m[2]]);
        // The server is whoever sends us a PlayerStats of the right size.
        if self.server.is_none() && ty == PLAYER_STATS && m.len() == 164 {
            self.server = Some(src);
            self.state.server = Some(format!("{}:{}", src.ip, src.port));
        }
        let Some(server) = self.server else { return };
        let from_server = src == server;
        let to_server = dst == server;
        let s = &mut self.state;
        match ty {
            PLAYER_STATS if from_server && m.len() >= 122 => {
                for (k, name) in STATS.iter().enumerate() {
                    if let Some(v) = f32_at(m, 6 + 4 * k) {
                        s.stats.insert(name, v);
                    }
                }
                s.calories = f32_at(m, 102);
                s.proteins = f32_at(m, 106);
                s.lipids = f32_at(m, 110);
                s.carbohydrates = f32_at(m, 114);
                s.weight = f32_at(m, 118);
                s.last_ms = time;
            }
            PLAYER_HEALTH if from_server && m.len() >= 74 => {
                s.part_health = (0..17).filter_map(|k| f32_at(m, 6 + 4 * k)).collect();
                // Overall health: measured to lose ~0.286 per point lost in the parts.
                let lost: f32 = s.part_health.iter().map(|h| (100.0 - h).max(0.0)).sum();
                s.health = Some((100.0 - lost * 0.286).clamp(0.0, 100.0));
                s.last_ms = time;
            }
            PLAYER_INJURIES if from_server && m.len() >= 26 => {
                s.injured_idle = f32_at(m, 6);
                s.walk_speed = f32_at(m, 18);
                s.run_speed = f32_at(m, 22);
            }
            PLAYER_EFFECTS if from_server && m.len() >= 42 => {
                let names = [("sleep", 6), ("beta", 18), ("depress", 26), ("painkiller", 34)];
                for (n, off) in names {
                    if let Some(v) = f32_at(m, off) {
                        s.effects.insert(n, v);
                    }
                }
                s.sleeping_pills = m.get(14..18).and_then(|b| b.try_into().ok()).map(i32::from_be_bytes);
            }
            PLAYER_DAMAGE if from_server => {
                let _ = decode_damage(m, s);
            }
            PLAYER_UPDATE_RELIABLE if to_server && m.len() >= 28 => {
                // PlayerID (3), then Prediction: type, x, y, z (i8), direction, moveDirection, speed.
                if let (Some(x), Some(y)) = (f32_at(m, 7), f32_at(m, 11)) {
                    s.position = Some((x, y, m[15] as i8));
                }
                s.direction = f32_at(m, 16);
                s.speed = f32_at(m, 24);
            }
            _ => {}
        }
    }
}
