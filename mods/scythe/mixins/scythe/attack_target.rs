use std::sync::atomic::{AtomicUsize, Ordering};

static SCYTHE_LAST_TARGET: AtomicUsize = AtomicUsize::new(usize::MAX);

fn scythe_attack_target(game: &mut crate::game::Game<'_>, index: usize) {
    use crate::game::scythe as m;
    if !game.native_on(m::ID) || !game.native_playing() {
        return;
    }
    SCYTHE_LAST_TARGET.store(index, Ordering::Relaxed);
}
