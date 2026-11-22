use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
fn main() {
    let seed: u64 = std::env::args().nth(1).unwrap().parse().unwrap();
    let mut rng = StdRng::seed_from_u64(seed);
    for _ in 0..4 {
        let u1: f64 = rng.gen();
        let u2: f64 = rng.gen();
        println!("{:.17} {:.17}", u1, u2);
    }
}
