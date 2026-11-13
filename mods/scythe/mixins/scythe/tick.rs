fn scythe_tick(game: &mut crate::game::Game<'_>) {
    use crate::game::scythe as m;
    if game.native_window() {
        m::tick_cooldown();
        if game.native_on(m::ID) && game.native_playing() {
            m::pull_chain(game);
        }
    }
}
