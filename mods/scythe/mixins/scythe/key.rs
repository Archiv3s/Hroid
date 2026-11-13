fn scythe_key(game: &mut crate::game::Game<'_>, key: &str, pressed: bool) {
    use crate::game::scythe as m;
    if !pressed || !game.native_on(m::ID) || !game.native_playing() {
        return;
    }
    if key != game.native_mod_key(m::ID, "chain", "key.keyboard.p") {
        return;
    }
    if m::cooldown() > 0 {
        return;
    }
    m::start_cooldown();
}
