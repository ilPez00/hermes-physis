use physis_core::embed::RandomProjectionEmbedder;
use physis_core::map::compile_context_with_observer;
use physis_core::map::ObserverReserve;
fn main() {
    let embedder = RandomProjectionEmbedder::new(64);
    let big = (1..=30).map(|i| format!("word{} filler{} token{}", i, i, i)).collect::<Vec<_>>().join(" and ");
    let docs3 = vec![("big.md".to_string(), big.clone()), ("small.md".to_string(), "tiny note".to_string())];
    for budget in [64usize, 128, 256] {
        let r = compile_context_with_observer(&docs3, &embedder, "word7 filler7", budget, &[], 0.9, ObserverReserve::None).unwrap();
        println!("== budget {} ==", budget);
        println!("physis_tokens {} baseline_tokens {}", r.physis_tokens, r.baseline_tokens);
        let head: String = r.physis_context.chars().take(60).collect();
        println!("ctx head: {}", head);
    }
}
