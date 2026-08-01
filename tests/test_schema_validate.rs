use std::io::Write;

use crate::workdir::Workdir;
use crate::{Csv, CsvData};

#[test]
fn schema_basic_types() {
    let wrk = Workdir::new("schema_basic_types");
    wrk.create(
        "in.csv",
        vec![
            svec!["name", "age", "score"],
            svec!["Alice", "30", "9.5"],
            svec!["Bob", "25", "10.0"],
        ],
    );
    let got: CsvData = wrk.read_stdout(&mut wrk.command("schema").arg("in.csv"));
    let expected = vec![
        svec!["name", "string"],
        svec!["age", "integer"],
        svec!["score", "float"],
    ];
    assert_eq!(got, CsvData::from_vecs(expected));
}

#[test]
fn schema_mixed_types_degrade_to_string() {
    let wrk = Workdir::new("schema_mixed_types");
    wrk.create(
        "in.csv",
        vec![svec!["a", "b"], svec!["1", "x"], svec!["2", "hello world"]],
    );
    let got: CsvData = wrk.read_stdout(&mut wrk.command("schema").arg("in.csv"));
    let expected = vec![svec!["a", "integer"], svec!["b", "string"]];
    assert_eq!(got, CsvData::from_vecs(expected));
}

#[test]
fn validate_good_file() {
    let wrk = Workdir::new("validate_good_file");
    wrk.create(
        "in.csv",
        vec![svec!["a", "b"], svec!["1", "x"], svec!["2", "y"]],
    );
    wrk.run(&mut wrk.command("validate").arg("in.csv"));
}

#[test]
fn validate_bad_file_exits_nonzero() {
    let wrk = Workdir::new("validate_bad_file");
    let path = wrk.path("in.csv");
    let mut f = std::fs::File::create(&path).unwrap();
    writeln!(f, "a,b").unwrap();
    writeln!(f, "1,x,extra").unwrap();
    f.flush().unwrap();

    let mut cmd = wrk.command("validate");
    cmd.arg("in.csv");
    let out = cmd.output().unwrap();
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("length mismatch"));
}
