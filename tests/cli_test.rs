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

    let experimental_mv = run(&[
        "--algorithm",
        "micali-vazirani",
        "max-cardinality",
        "-i",
        graph_path,
    ]);
    assert!(
        String::from_utf8_lossy(&experimental_mv.stderr)
            .contains("Micali–Vazirani is experimental")
    );

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

    for solver in ["sparse-blossom", "blossom-tree", "cherry-tree"] {
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

#[test]
fn generic_benchmark_runner_excludes_experimental_mv() {
    let temp_root =
        std::env::temp_dir().join(format!("blossom-study-mv-benchmark-{}", std::process::id()));
    std::fs::create_dir(&temp_root).expect("temporary benchmark directory should be created");
    let config_path = temp_root.join("config.json");
    let output_path = temp_root.join("results");
    std::fs::write(
        &config_path,
        r#"{
            "graph_sizes": [4],
            "densities": [1.0],
            "weight_ranges": [[1.0, 2.0]],
            "runs_per_config": 1,
            "algorithms": ["micali_vazirani"]
        }"#,
    )
    .expect("benchmark config should be written");

    let output = run(&[
        "bench",
        "--config",
        config_path.to_str().expect("UTF-8 config path"),
        "--output",
        output_path.to_str().expect("UTF-8 output path"),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("Micali–Vazirani is experimental and excluded")
    );
    assert_eq!(
        std::fs::read_to_string(output_path.join("bench_results.json"))
            .expect("benchmark results should exist")
            .trim(),
        "[]"
    );

    std::fs::remove_file(output_path.join("bench_results.json")).unwrap();
    std::fs::remove_dir(&output_path).unwrap();
    std::fs::remove_file(config_path).unwrap();
    std::fs::remove_dir(temp_root).unwrap();
}
