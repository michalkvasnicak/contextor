use std::fs;
use std::path::Path;

use anyhow::{Context, Result, ensure};

use super::{Annotation, Language};

// Add <case>.<language>.fixture and <case>.<language>.json under tests/fixtures/parser.
// The .fixture suffix prevents formatters from shifting the expected line ranges.
#[test]
fn parser_fixtures() -> Result<()> {
    let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/parser");
    let mut fixtures = Vec::new();
    for entry in fs::read_dir(&directory)? {
        let path = entry?.path();
        if path.extension().is_some_and(|suffix| suffix == "fixture") {
            fixtures.push(path);
        }
    }
    ensure!(!fixtures.is_empty(), "no parser fixtures in {}", directory.display());
    fixtures.sort_unstable();

    let mut failures = Vec::new();
    for fixture in fixtures {
        if let Err(error) = check_fixture(&fixture) {
            failures.push(format!("{}: {error:#}", fixture.display()));
        }
    }
    ensure!(failures.is_empty(), "\n{}", failures.join("\n"));
    Ok(())
}

fn check_fixture(fixture: &Path) -> Result<()> {
    let language_path = fixture.with_extension("");
    let language = Language::for_path(&language_path).context("unsupported fixture language")?;
    let expected_path = fixture.with_extension("json");
    let expected = fs::read_to_string(&expected_path)
        .with_context(|| format!("reading {}", expected_path.display()))?;
    let mut expected: Vec<Annotation> = serde_json::from_str(&expected)
        .with_context(|| format!("parsing {}", expected_path.display()))?;
    let source = fs::read_to_string(fixture)?;
    let mut actual = language.extract(&source)?;

    expected.sort_unstable();
    actual.sort_unstable();
    ensure!(actual == expected, "expected {expected:#?}\nactual {actual:#?}");
    Ok(())
}
