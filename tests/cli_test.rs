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

    for solver in ["gabow-weighted-approach", "mehlhorn-nobahari"] {
        let output = run(&["--algorithm", solver, "max-cardinality", "-i", graph_path]);
        assert!(
            output.status.success(),
            "solver {solver}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("is_perfect=true"));
    }

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

    for solver in [
        "sparse-blossom",
        "blossom-tree",
        "cherry-tree",
        "duan-scaling",
    ] {
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
fn generic_benchmark_runner_includes_imported_solvers() {
    let temp_root = std::env::temp_dir().join(format!(
        "blossom-study-imported-benchmark-{}",
        std::process::id()
    ));
    std::fs::create_dir(&temp_root).unwrap();
    let config_path = temp_root.join("config.json");
    let output_path = temp_root.join("results");
    std::fs::write(
        &config_path,
        r#"{
            "graph_sizes": [4],
            "densities": [1.0],
            "weight_ranges": [[1.0, 2.0]],
            "runs_per_config": 1,
            "seed": 17,
            "algorithms": ["fast_blossom", "gabow-weighted-approach", "mehlhorn-nobahari", "primal-dual", "duan-scaling"]
        }"#,
    )
    .unwrap();
    let output = run(&[
        "bench",
        "--config",
        config_path.to_str().unwrap(),
        "--output",
        output_path.to_str().unwrap(),
    ]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let records: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(output_path.join("bench_results.json")).unwrap(),
    )
    .unwrap();
    let names: std::collections::BTreeSet<_> = records
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["algorithm"].as_str().unwrap())
        .collect();
    assert_eq!(
        names,
        [
            "fast-blossom",
            "gabow-weighted-approach",
            "mehlhorn-nobahari",
            "primal-dual",
            "duan-scaling"
        ]
        .into_iter()
        .collect()
    );
    std::fs::remove_dir_all(temp_root).unwrap();
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

#[test]
fn generic_benchmark_runner_labels_and_checks_cardinality_solvers() {
    let temp_root =
        std::env::temp_dir().join(format!("blossom-study-comparison-{}", std::process::id()));
    std::fs::create_dir(&temp_root).expect("temporary benchmark directory should be created");
    let config_path = temp_root.join("config.json");
    let output_path = temp_root.join("results");
    std::fs::write(
        &config_path,
        r#"{
            "graph_sizes": [6],
            "densities": [1.0],
            "weight_ranges": [[1.0, 2.0], [-5.0, -4.0]],
            "runs_per_config": 2,
            "seed": 12345,
            "algorithms": ["fast_blossom", "gabow", "primal-dual"]
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
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("maximum-cardinality"));
    assert!(stdout.contains("fast-blossom"));
    assert!(stdout.contains("gabow-heuristic"));
    assert!(stdout.contains("minimum-weight-perfect"));

    let results_path = output_path.join("bench_results.json");
    let records: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&results_path).expect("benchmark results should exist"),
    )
    .expect("benchmark results should be valid JSON");
    let records = records.as_array().unwrap();
    assert_eq!(records.len(), 12);
    let mut signatures = std::collections::BTreeMap::new();
    for record in records {
        assert_eq!(record["seed"], 12345);
        assert_eq!(record["m"], 15);
        let run = record["run"].as_u64().unwrap();
        assert!((1..=2).contains(&run));
        let range = record["weight_range"].as_array().unwrap();
        let minimum = range[0].as_f64().unwrap();
        let maximum = range[1].as_f64().unwrap();
        let observed_minimum = record["observed_weight_min"].as_f64().unwrap();
        let observed_maximum = record["observed_weight_max"].as_f64().unwrap();
        assert!(observed_minimum >= minimum && observed_maximum < maximum);

        let key = (
            minimum.to_bits(),
            run,
            record["algorithm"].as_str().unwrap().to_string(),
        );
        signatures.insert(key, record["graph_signature"].as_str().unwrap().to_string());
    }
    assert_eq!(signatures.len(), 12);
    for range_minimum in [1.0_f64, -5.0] {
        for run in 1..=2 {
            let fast = signatures
                .get(&(range_minimum.to_bits(), run, "fast-blossom".to_string()))
                .unwrap();
            let gabow = signatures
                .get(&(range_minimum.to_bits(), run, "gabow-heuristic".to_string()))
                .unwrap();
            let weighted = signatures
                .get(&(range_minimum.to_bits(), run, "primal-dual".to_string()))
                .unwrap();
            assert_eq!(fast, gabow);
            assert_eq!(fast, weighted);
        }
        let run_one = signatures
            .get(&(range_minimum.to_bits(), 1, "fast-blossom".to_string()))
            .unwrap();
        let run_two = signatures
            .get(&(range_minimum.to_bits(), 2, "fast-blossom".to_string()))
            .unwrap();
        assert_ne!(run_one, run_two, "each run should use a fresh graph");
    }

    let first_run_signatures: Vec<_> = records
        .iter()
        .map(|record| record["graph_signature"].as_str().unwrap())
        .collect();
    let repeated = run(&[
        "bench",
        "--config",
        config_path.to_str().expect("UTF-8 config path"),
        "--output",
        output_path.to_str().expect("UTF-8 output path"),
    ]);
    assert!(repeated.status.success());
    let repeated_records: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&results_path).expect("second benchmark results should exist"),
    )
    .expect("second benchmark results should be valid JSON");
    let repeated_signatures: Vec<_> = repeated_records
        .as_array()
        .unwrap()
        .iter()
        .map(|record| record["graph_signature"].as_str().unwrap())
        .collect();
    assert_eq!(first_run_signatures, repeated_signatures);

    std::fs::remove_file(results_path).unwrap();
    std::fs::remove_dir(&output_path).unwrap();
    std::fs::remove_file(config_path).unwrap();
    std::fs::remove_dir(temp_root).unwrap();
}

#[test]
fn generic_benchmark_runner_rejects_zero_runs() {
    let temp_root = std::env::temp_dir().join(format!(
        "blossom-study-invalid-benchmark-{}",
        std::process::id()
    ));
    std::fs::create_dir(&temp_root).expect("temporary benchmark directory should be created");
    let config_path = temp_root.join("config.json");
    std::fs::write(
        &config_path,
        r#"{
            "graph_sizes": [4],
            "densities": [1.0],
            "weight_ranges": [[1.0, 2.0]],
            "runs_per_config": 0,
            "algorithms": ["fast_blossom"]
        }"#,
    )
    .expect("benchmark config should be written");

    let output = run(&[
        "bench",
        "--config",
        config_path.to_str().expect("UTF-8 config path"),
        "--output",
        temp_root
            .join("results")
            .to_str()
            .expect("UTF-8 output path"),
    ]);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr)
            .contains("runs_per_config must be greater than zero")
    );

    std::fs::remove_file(config_path).unwrap();
    std::fs::remove_dir(temp_root).unwrap();
}
