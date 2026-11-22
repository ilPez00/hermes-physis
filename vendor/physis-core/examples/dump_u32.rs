use rand::rngs::StdRng;
use rand::RngCore;
use rand::{Rng, SeedableRng};
fn main() {
    let seed: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut rng = StdRng::seed_from_u64(seed);
    for _ in 0..8 { print!("{} ", rng.next_u32()); }
    println!();
    let mut rng = StdRng::seed_from_u64(seed);
    print!("u64s:");
    for _ in 0..4 { print!(" {} ", rng.next_u64()); }
    println!();
}
