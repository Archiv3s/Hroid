use std::cell::Cell;

pub const ID: &str = "scythe";
pub const VERSION: &str = "0.1.0";
pub const CHAIN_COOLDOWN_TICKS: u32 = 100;
const CHAIN_RANGE: f64 = 16.0;
const CONE_COS: f64 = 0.966;
const CHAIN_HOLD_TICKS: u32 = 40;
const PULL_SPEED: f64 = 0.6;
const STOP_DISTANCE: f64 = 1.5;

thread_local! {
    static COOLDOWN: Cell<u32> = const { Cell::new(0) };
    static HOLD: Cell<Option<(u32, u32)>> = const { Cell::new(None) };
}

pub fn cooldown() -> u32 {
    COOLDOWN.with(|c| c.get())
}

pub fn start_cooldown() {
    COOLDOWN.with(|c| c.set(CHAIN_COOLDOWN_TICKS));
}

pub fn tick_cooldown() {
    COOLDOWN.with(|c| c.set(c.get().saturating_sub(1)));
}

pub fn start_chain(game: &mut crate::game::Game<'_>) -> bool {
    let Some(eye) = game.eye_position() else {
        return false;
    };
    let yaw = f64::from(game.yaw).to_radians();
    let pitch = f64::from(game.pitch).to_radians();
    let aim = [-yaw.sin() * pitch.cos(), -pitch.sin(), yaw.cos() * pitch.cos()];
    let mut best: Option<(f64, u32)> = None;
    for mob in game.mobs.iter().filter(|m| !m.removed) {
        let d = [
            f64::from(mob.position.x) - eye[0],
            f64::from(mob.position.y) - eye[1],
            f64::from(mob.position.z) - eye[2],
        ];
        let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
        if dist < 0.5 || dist > CHAIN_RANGE {
            continue;
        }
        let facing = (d[0] * aim[0] + d[1] * aim[1] + d[2] * aim[2]) / dist;
        if facing < CONE_COS {
            continue;
        }
        if best.map_or(true, |(b, _)| dist < b) {
            best = Some((dist, mob.id));
        }
    }
    let Some((_, id)) = best else {
        return false;
    };
    HOLD.with(|h| h.set(Some((id, CHAIN_HOLD_TICKS))));
    true
}

pub fn pull_chain(game: &mut crate::game::Game<'_>) {
    let Some((id, ticks)) = HOLD.with(|h| h.get()) else {
        return;
    };
    let Some(eye) = game.eye_position() else {
        clear_chain();
        return;
    };
    if ticks == 0 {
        clear_chain();
        return;
    }
    let Some(mob) = game.mobs.iter_mut().find(|m| m.id == id && !m.removed) else {
        clear_chain();
        return;
    };
    let d = [
        eye[0] - f64::from(mob.position.x),
        eye[1] - f64::from(mob.position.y),
        eye[2] - f64::from(mob.position.z),
    ];
    let dist = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if dist < STOP_DISTANCE {
        clear_chain();
        return;
    }
    let k = PULL_SPEED / dist;
    mob.velocity.x = (d[0] * k) as _;
    mob.velocity.y = (d[1] * k) as _;
    mob.velocity.z = (d[2] * k) as _;
    mob.needs_sync = true;
    HOLD.with(|h| h.set(Some((id, ticks - 1))));
}

pub fn clear_chain() {
    HOLD.with(|h| h.set(None));
}

#[cfg(test)]
mod tests {
    #[test]
    fn cooldown_counts_down_to_zero() {
        super::start_cooldown();
        for _ in 0..super::CHAIN_COOLDOWN_TICKS {
            super::tick_cooldown();
        }
        assert_eq!(super::cooldown(), 0);
    }
}
