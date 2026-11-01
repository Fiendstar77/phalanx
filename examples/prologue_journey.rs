//! Real Theus encounters with bounded retries. No forced outcome or invented history.
use phalanx::{learned::Controller, progression::Campaign, terrain::Arena};
fn main() {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "outputs/prologue-journey".into());
    let path = std::path::Path::new(&path);
    std::fs::create_dir_all(path).unwrap();
    let mut c = Campaign::new_prologue();
    let mut seed = 42;
    c.save(&path.join("chapter-1.json")).unwrap();
    while c.prologue_active() {
        let chapter = c.prologue.as_ref().unwrap().chapter;
        let mut won = false;
        for _ in 0..24 {
            let (id, mut b) = c
                .begin_battle(seed, Arena::OpenField, Controller::Utility)
                .unwrap();
            let mut recorder = phalanx::study::Recorder::start(&path.join("studies"), &b).unwrap();
            while !b.finished {
                recorder.step(&mut b).unwrap();
            }
            c.settle(id, &b).unwrap();
            recorder
                .save_learning(c.last_report.as_ref().unwrap())
                .unwrap();
            println!(
                "chapter={} battle={} seed={} tick={} winner={:?}",
                chapter + 1,
                id + 1,
                seed,
                b.tick,
                b.winner()
            );
            seed += 1;
            if b.winner() == Some(0) {
                won = true;
                break;
            }
        }
        assert!(won, "No witnessed victory in the retry budget");
        c.save(&path.join(format!("chapter-{}-won.json", chapter + 1)))
            .unwrap();
        c.advance_prologue().unwrap();
        c.save(&path.join(format!("chapter-{}.json", chapter + 2)))
            .unwrap();
    }
    std::fs::write(
        path.join("opening-receipts.json"),
        serde_json::to_string_pretty(c.prologue.as_ref().unwrap()).unwrap(),
    )
    .unwrap();
}
