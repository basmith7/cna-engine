//! `cna-play` end to end: new, apply, log, replay, show.

use std::path::Path;
use std::process::Command;

fn play(dir: &Path, args: &[&str]) -> (bool, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_cna-play"))
        .current_dir(dir)
        .args(args)
        .output()
        .unwrap();
    (
        out.status.success(),
        String::from_utf8(out.stdout).unwrap(),
        String::from_utf8(out.stderr).unwrap(),
    )
}

#[test]
fn new_apply_log_replay_show() {
    let dir = std::env::temp_dir().join(format!("cna-play-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let (ok, _, err) = play(
        &dir,
        &[
            "new",
            "grazianis-offensive",
            "--seed",
            "7",
            "-o",
            "state.json",
        ],
    );
    assert!(ok, "{err}");
    let state = std::fs::read_to_string(dir.join("state.json")).unwrap();
    assert!(state.contains("\"scenario\": \"grazianis-offensive\""));

    let (ok, out, _) = play(&dir, &["show", "state.json"]);
    assert!(ok);
    assert!(out.contains("GT1 stage 1 phase A"), "{out}");
    assert!(out.contains("waiting on axis"), "{out}");
    assert!(out.contains("pending: 59"), "{out}");
    assert!(out.contains("C4218: 14"), "{out}");

    std::fs::write(
        dir.join("orders.json"),
        r#"[{"player": "axis", "order": {"place": {"piece": "unit:it:saharan-detachment-hq", "hex": "C0716"}}}]"#,
    )
    .unwrap();
    let (ok, out, _) = play(&dir, &["apply", "state.json", "orders.json"]);
    assert!(ok);
    assert!(out.contains("placed"), "{out}");
    let (ok, out, _) = play(&dir, &["show", "state.json"]);
    assert!(ok);
    assert!(out.contains("pending: 58"), "{out}");

    // An illegal order fails, names its case, and leaves the file alone.
    let before = std::fs::read_to_string(dir.join("state.json")).unwrap();
    std::fs::write(
        dir.join("bad.json"),
        r#"[{"player": "cw", "order": "pass"}]"#,
    )
    .unwrap();
    let (ok, _, err) = play(&dir, &["apply", "state.json", "bad.json"]);
    assert!(!ok);
    assert!(err.contains("5.2"), "{err}");
    assert_eq!(
        std::fs::read_to_string(dir.join("state.json")).unwrap(),
        before
    );

    let (ok, _, _) = play(&dir, &["log", "state.json", "-o", "log.json"]);
    assert!(ok);
    let (ok, _, err) = play(&dir, &["replay", "log.json", "-o", "replayed.json"]);
    assert!(ok, "{err}");
    assert_eq!(
        std::fs::read_to_string(dir.join("replayed.json")).unwrap(),
        before
    );
    std::fs::remove_dir_all(&dir).unwrap();
}
