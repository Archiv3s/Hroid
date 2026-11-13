use std::sync::atomic::{AtomicBool, Ordering};

static SCYTHE_LAST_CRIT: AtomicBool = AtomicBool::new(false);

fn scythe_melee_result(
    game: &mut crate::game::Game<'_>,
    value: (f32, f32, bool, bool),
) -> (f32, f32, bool, bool) {
    use crate::game::scythe as m;
    if game.native_on(m::ID) && game.native_playing() {
        SCYTHE_LAST_CRIT.store(value.2, Ordering::Relaxed);
    }
    value
}
