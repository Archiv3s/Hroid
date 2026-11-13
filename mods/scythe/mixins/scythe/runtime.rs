use std::cell::Cell;

pub const ID: &str = "scythe";
pub const VERSION: &str = "0.1.0";
pub const CHAIN_COOLDOWN_TICKS: u32 = 100;

thread_local! {
    static COOLDOWN: Cell<u32> = const { Cell::new(0) };
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
