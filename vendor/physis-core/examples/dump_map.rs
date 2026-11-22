//! Dumps build_map output for the parity corpus — the oracle for
//! cphysis's C port. Usage: dump_map
use physis_core::embed::RandomProjectionEmbedder;
use physis_core::embed::VectorEmbed;
use physis_core::map::build_map;

fn main() {
    let mut docs: Vec<(String, String)> = Vec::new();
    for i in 0..6 {
        docs.push((
            format!("a{i}.md"),
            format!("Pump maintenance required: replace the seal on line {i}, vibration rising after bearing wear. Schedule downtime tonight."),
        ));
    }
    for i in 0..5 {
        docs.push((
            format!("b{i}.md"),
            format!("Invoice {i}: payment terms net 30, purchase order approved, vendor shipping confirmation received by the office."),
        ));
    }
    docs.push((
        "x.md".into(),
        "Quantum tessellation of the mandelbrot choir sings beneath the ice.".into(),
    ));
    let e = RandomProjectionEmbedder::new(64);
    let r = build_map(&docs, &e, None).unwrap();
    println!("docs={} threshold={}", r.docs, r.threshold_used);
    println!("structure_hash={}", r.structure_hash);
    for c in &r.repeats {
        println!("repeat {} name={} count={} hints={}", c.id, c.name, c.count, c.hints.join(","));
        for s in &c.instances { println!("  inst {}", s.path); }
    }
    for d in &r.differences {
        println!("diff path={} best_sim={:.9} separation={:.9} cluster={:?}", d.path, d.best_sim, d.separation, d.nearest_cluster);
    }
    for c in &r.contradictions {
        println!("contra a={} b={} cluster={} lex={:.9} sim={:.9}", c.a.path, c.b.path, c.cluster, c.lexical_overlap, c.embedding_sim);
    }
}
