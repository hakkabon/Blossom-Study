use std::process::Command;

fn run(arguments: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_blossom_study"))
        .args(arguments)
        .output()
        .expect("CLI should start")
}

#[test]
fn documented_cli_workflow_succeeds() {
    let graph_path =
        std::env::temp_dir().join(format!("blossom-study-cli-{}.json", std::process::id()));
    let graph_path = graph_path.to_str().expect("UTF-8 temporary path");

    let generated = run(&["generate", "-n", "6", "-o", graph_path]);
    assert!(
        generated.status.success(),
        "{}",
        String::from_utf8_lossy(&generated.stderr)
    );

    let cardinality = run(&["max-cardinality", "-i", graph_path]);
    assert!(
        cardinality.status.success(),
        "{}",
        String::from_utf8_lossy(&cardinality.stderr)
    );
    assert!(String::from_utf8_lossy(&cardinality.stdout).contains("is_perfect=true"));

    let weighted = run(&[
        "--algorithm",
        "fast-blossom",
        "min-weight-perfect",
        "-i",
        graph_path,
    ]);
    assert!(
        weighted.status.success(),
        "{}",
        String::from_utf8_lossy(&weighted.stderr)
    );
    assert!(String::from_utf8_lossy(&weighted.stdout).contains("is_perfect=true"));

    let blossom_engine = run(&[
        "min-weight-perfect",
        "--solver",
        "blossom-engine",
        "-i",
        graph_path,
    ]);
    assert!(
        blossom_engine.status.success(),
        "{}",
        String::from_utf8_lossy(&blossom_engine.stderr)
    );
    assert!(String::from_utf8_lossy(&blossom_engine.stdout).contains("is_perfect=true"));

    for solver in ["sparse-blossom", "blossom-tree", "cherry-tree-experimental"] {
        let output = run(&["min-weight-perfect", "--solver", solver, "-i", graph_path]);
        assert!(
            output.status.success(),
            "solver {solver}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("is_perfect=true"));
    }

    std::fs::remove_file(graph_path).expect("temporary graph should be removable");
}
