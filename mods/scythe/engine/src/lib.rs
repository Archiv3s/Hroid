use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use horizon_client::mod_api::{CriticalHitEvent, Flow, Host, Hooks, HurtEnemyEvent, KeyEvent, NetFromClientEvent, ServerTickEvent};

// Player ids are u128 in NativeServerPlayer.uuid.
type Uuid = u128;

const CHANNEL: &str = "scythe:chain";
const SCYTHE: &str = "scythe:black_scythe";
const CHAIN_KEY: &str = "key.keyboard.p";
const RANGE: f64 = 16.0;
const CONE_COS: f64 = 0.966;
const COOLDOWN: Duration = Duration::from_secs(4);
const CHAIN_TIME: Duration = Duration::from_secs(2);
// Index into SOUND_SOURCES for the player category (index 7 in the list read from the docs).
const SOURCE_PLAYER: usize = 7;
const SOUND_CHAIN: &str = "minecraft:block.chain.place";
const SOUND_CRIT: &str = "minecraft:entity.player.attack.crit";

// critical_hit runs before the hit lands; hurt_enemy reads and clears this for the same blow.
// A crit that does no damage leaves the flag set until the next hit, which is a known gap.
static CRIT_PENDING: AtomicBool = AtomicBool::new(false);

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
        Flow::Continue
    }

    fn critical_hit(&self, event: &mut CriticalHitEvent<'_>) -> Flow {
        CRIT_PENDING.store(event.critical, Ordering::SeqCst);
        Flow::Continue
    }

    fn hurt_enemy(&self, host: &mut dyn Host, event: &mut HurtEnemyEvent<'_>) -> Flow {
        let crit = CRIT_PENDING.swap(false, Ordering::SeqCst);
        if !crit || event.stack.item != SCYTHE {
            return Flow::Continue;
        }
        let dimension = host.dimension();
        if let Some(mob) = host.mob(&dimension, event.entity) {
            let _ = host.play_sound(SOUND_CRIT, mob.position, SOURCE_PLAYER, 1.0, 0.8);
        }
        host.kill_mob(&dimension, event.entity);
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
