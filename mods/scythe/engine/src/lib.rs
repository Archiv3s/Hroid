use std::sync::Mutex;
use std::time::{Duration, Instant};

use horizon_client::mod_api::{
    CriticalHitEvent, EquipmentSlot, Flow, Host, Hooks, KeyEvent, NetFromClientEvent,
    ServerTickEvent, Uuid,
};

const CHANNEL: &str = "scythe:chain";
const SCYTHE: &str = "scythe:black_scythe";
const CHAIN_KEY: &str = "key.keyboard.p";
const RANGE: f64 = 16.0;
const CONE_COS: f64 = 0.966;
const COOLDOWN: Duration = Duration::from_secs(4);
const CHAIN_TIME: Duration = Duration::from_secs(2);
const SOURCE_PLAYER: usize = 7;
const SOUND_CHAIN: &str = "minecraft:block.chain.place";
const SOUND_CRIT: &str = "minecraft:entity.player.attack.crit";

struct Chain {
    player: Uuid,
    dimension: String,
    mob: i32,
    until: Instant,
}

static CHAINS: Mutex<Vec<Chain>> = Mutex::new(Vec::new());
static LAST_USE: Mutex<Vec<(Uuid, Instant)>> = Mutex::new(Vec::new());

fn look(yaw: f64, pitch: f64) -> [f64; 3] {
    let (yaw, pitch) = (yaw.to_radians(), pitch.to_radians());
    [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()]
}

fn sub(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn scale(a: [f64; 3], k: f64) -> [f64; 3] {
    [a[0] * k, a[1] * k, a[2] * k]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn len(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

fn cooling_down(uuid: Uuid) -> bool {
    LAST_USE
        .lock()
        .unwrap()
        .iter()
        .any(|(u, t)| *u == uuid && t.elapsed() < COOLDOWN)
}

fn mark_used(uuid: Uuid) {
    let mut last = LAST_USE.lock().unwrap();
    last.retain(|(u, _)| *u != uuid);
    last.push((uuid, Instant::now()));
}

pub struct Scythe;

impl Hooks for Scythe {
    fn key(&self, host: &mut dyn Host, event: &mut KeyEvent<'_>) -> Flow {
        if event.pressed && event.key == host.mod_key("scythe", "chain", CHAIN_KEY) {
            let _ = host.net_send_to_server(CHANNEL, &[]);
        }
        Flow::Continue
    }

    fn net_from_client(&self, host: &mut dyn Host, event: &mut NetFromClientEvent<'_>) -> Flow {
        if event.channel != CHANNEL {
            return Flow::Continue;
        }
        let Some(me) = host.server_players().into_iter().find(|p| p.uuid == event.player) else {
            return Flow::Continue;
        };
        if cooling_down(me.uuid) {
            return Flow::Continue;
        }
        let held = host.player_equipment(me.uuid, EquipmentSlot::Mainhand);
        if !held.is_some_and(|s| s.item == SCYTHE) {
            return Flow::Continue;
        }

        let eye = [me.position[0], me.position[1] + 1.62, me.position[2]];
        let aim = look(me.yaw, me.pitch);
        let mut best: Option<(f64, i32)> = None;
        for mob in host.mobs(&me.dimension) {
            let d = sub(mob.position, eye);
            let dist = len(d);
            if dist < 0.5 || dist > RANGE || dot(d, aim) / dist < CONE_COS {
                continue;
            }
            if !host.has_line_of_sight(me.entity_id, mob.id) {
                continue;
            }
            if best.map_or(true, |(b, _)| dist < b) {
                best = Some((dist, mob.id));
            }
        }

        let Some((_, mob)) = best else {
            host.action_bar(me.uuid, "No mob in sight to chain");
            return Flow::Continue;
        };
        CHAINS.lock().unwrap().push(Chain {
            player: me.uuid,
            dimension: me.dimension.clone(),
            mob,
            until: Instant::now() + CHAIN_TIME,
        });
        mark_used(me.uuid);
        let _ = host.play_sound(SOUND_CHAIN, eye, SOURCE_PLAYER, 0.8, 1.2);
        host.action_bar(me.uuid, "Chained");
        Flow::Continue
    }

    fn server_tick(&self, host: &mut dyn Host, _event: &mut ServerTickEvent<'_>) {
        let players = host.server_players();
        let now = Instant::now();
        let mut chains = CHAINS.lock().unwrap();
        chains.retain(|c| {
            c.until > now
                && players.iter().any(|p| p.uuid == c.player)
                && host.mob(&c.dimension, c.mob).is_some()
        });
        for c in chains.iter() {
            let Some(p) = players.iter().find(|p| p.uuid == c.player) else {
                continue;
            };
            let Some(mob) = host.mob(&c.dimension, c.mob) else {
                continue;
            };
            let hand = [p.position[0], p.position[1] + 1.0, p.position[2]];
            let d = sub(hand, mob.position);
            let dist = len(d);
            if dist < 1.5 {
                continue;
            }
            let speed = (dist * 0.15).min(1.2);
            host.push_mob(&c.dimension, c.mob, scale(d, speed / dist));
        }
    }

    fn critical_hit(&self, host: &mut dyn Host, event: &mut CriticalHitEvent<'_>) -> Flow {
        let held = host.player_equipment(event.attacker, EquipmentSlot::Mainhand);
        if !held.is_some_and(|s| s.item == SCYTHE) {
            return Flow::Continue;
        }
        let at = event.target_position;
        let _ = host.play_sound(SOUND_CRIT, at, SOURCE_PLAYER, 1.0, 0.8);
        host.kill_mob(&event.dimension, event.target_id);
        Flow::Continue
    }
}

pub fn hooks() -> Box<dyn Hooks> {
    Box::new(Scythe)
}

#[cfg(test)]
mod tests {
    #[test]
    fn zero_yaw_looks_south() {
        let v = super::look(0.0, 0.0);
        assert!(v[2] > 0.99 && v[0].abs() < 1e-9);
    }
}
