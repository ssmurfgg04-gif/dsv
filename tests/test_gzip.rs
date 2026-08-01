use crate::workdir::Workdir;
use crate::{Csv, CsvData};

fn create_csv(wrk: &Workdir, name: &str) {
    wrk.create(
        name,
        vec![svec!["h1", "h2"], svec!["a", "b"], svec!["y", "z"]],
    );
}

#[test]
fn gzip_roundtrip_count() {
    let wrk = Workdir::new("gzip_roundtrip_count");
    create_csv(&wrk, "in.csv");
    wrk.run(
        &mut wrk
            .command("cat")
            .arg("rows")
            .arg("in.csv")
            .arg("-o")
            .arg("out.csv.gz"),
    );

    let mut cmd = wrk.command("count");
    cmd.arg("out.csv.gz");
    let got: usize = wrk.stdout(&mut cmd);
    assert_eq!(got, 2);
}

#[test]
fn gzip_roundtrip_content() {
    let wrk = Workdir::new("gzip_roundtrip_content");
    create_csv(&wrk, "in.csv");
    wrk.run(
        &mut wrk
            .command("cat")
            .arg("rows")
            .arg("in.csv")
            .arg("-o")
            .arg("out.csv.gz"),
    );

    let got: CsvData = wrk.read_stdout(&mut wrk.command("cat").arg("rows").arg("out.csv.gz"));
    let expected: CsvData =
        CsvData::from_vecs(vec![svec!["h1", "h2"], svec!["a", "b"], svec!["y", "z"]]);
    assert_eq!(got, expected);
}

#[test]
fn gzip_convert_to_jsonl() {
    let wrk = Workdir::new("gzip_convert_to_jsonl");
    create_csv(&wrk, "in.csv");
    wrk.run(&mut wrk.command("convert").arg("in.csv").arg("out.jsonl.gz"));

    let mut cmd = wrk.command("count");
    cmd.arg("out.jsonl.gz");
    let got: usize = wrk.stdout(&mut cmd);
    assert_eq!(got, 2);
}
