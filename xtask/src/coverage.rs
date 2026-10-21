//! Offline consistency gate for the reviewed, commit-pinned syntax inventory.
use crate::{Manifest, Stage};
use gdfmt_parser::{LanguageVersion, LexOptions, ParseOptions, lex, parse, parse_expression};
use serde_json::Value;
use std::{collections::BTreeSet, error::Error, fs, path::Path};

type Result<T> = std::result::Result<T, Box<dyn Error>>;

fn array<'a>(value: &'a Value, field: &str) -> Result<&'a Vec<Value>> {
    value[field]
        .as_array()
        .ok_or_else(|| format!("coverage: missing array {field}").into())
}

fn string<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value[field]
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("coverage: missing text {field}").into())
}

pub fn run(root: &Path) -> Result<()> {
    let directory = root.join("tests/compatibility");
    let coverage: Value =
        serde_json::from_str(&fs::read_to_string(directory.join("syntax-coverage.json"))?)?;
    validate(&directory, &coverage)?;
    println!(
        "syntax coverage: {} grammar rows, {} source dispositions, {} official test classifications; {}",
        array(&coverage, "rows")?.len(),
        array(&coverage, "symbolDisposition")?.len(),
        array(&coverage, "officialTestInventory")?.len(),
        string(&coverage["acceptance"], "status")?
    );
    Ok(())
}

fn validate(directory: &Path, coverage: &Value) -> Result<()> {
    if coverage["schemaVersion"] != 1 {
        return Err("unsupported syntax coverage schema".into());
    }
    string(coverage, "scope")?;
    if string(&coverage["acceptance"], "status")? != "accepted-with-exclusions" {
        return Err("P3 syntax audit is not accepted; resolve pending evidence first".into());
    }
    let engine: Value = serde_json::from_str(&fs::read_to_string(directory.join("engine.json"))?)?;
    let pins: BTreeSet<_> = array(&engine, "engines")?
        .iter()
        .map(|pin| {
            (
                pin["languageVersion"].clone().to_string(),
                pin["commit"].clone().to_string(),
            )
        })
        .collect();
    let audited_pins = array(coverage, "enginePins")?;
    let actual: BTreeSet<_> = audited_pins
        .iter()
        .map(|pin| {
            (
                pin["languageVersion"].clone().to_string(),
                pin["commit"].clone().to_string(),
            )
        })
        .collect();
    if actual != pins || actual.len() != audited_pins.len() {
        return Err("syntax audit engine pins differ from engine.json".into());
    }
    let manifest: Manifest =
        serde_json::from_str(&fs::read_to_string(directory.join("manifest.json"))?)?;
    crate::validate_fixture_inventory(directory, &manifest)?;
    let rows = array(coverage, "rows")?;
    if rows.is_empty() {
        return Err("empty syntax coverage".into());
    }
    let mut ids = BTreeSet::new();
    for row in rows {
        let id = string(row, "id")?;
        if !ids.insert(id) {
            return Err(format!("duplicate grammar row {id}").into());
        }
        string(row, "family")?;
        string(row, "semanticBoundary")?;
        let since = string(row, "since")?;
        if since != "4.3-or-earlier"
            && !LanguageVersion::ALL
                .iter()
                .any(|version| version.to_string() == since)
        {
            return Err(format!("{id}: unknown introduction version {since}").into());
        }
        let excluded = match string(row, "status")? {
            "covered" => false,
            "excluded" => true,
            status => return Err(format!("{id}: unclassified coverage status {status}").into()),
        };
        let exclusions = array(row, "exclusions")?;
        if excluded && exclusions.is_empty() {
            return Err(format!("{id}: exclusion requires a reason").into());
        }
        for exclusion in exclusions {
            string(exclusion, "scope")?;
            string(exclusion, "reason")?;
        }
        array(row, "followUp")?;
        if array(row, "upstream")?.is_empty() || array(row, "officialTests")?.is_empty() {
            return Err(format!("{id}: missing upstream source/test evidence").into());
        }
        for reference in array(row, "officialTests")? {
            let path = string(reference, "path")?;
            let commit = string(reference, "commit")?;
            let blob = string(reference, "blob")?;
            if !path.starts_with("modules/gdscript/tests/")
                || blob.len() != 40
                || !array(coverage, "enginePins")?
                    .iter()
                    .any(|pin| pin["commit"] == commit)
            {
                return Err(format!("{id}: unpinned official test {path}").into());
            }
        }
        for (field, rejected) in [("positiveFixtures", false), ("negativeFixtures", true)] {
            let evidence = array(row, field)?;
            if evidence.is_empty() && !(excluded && !rejected) {
                return Err(format!("{id}: missing {field}").into());
            }
            for file in evidence {
                let file = file.as_str().ok_or("fixture reference must be text")?;
                let fixture = manifest
                    .fixtures
                    .iter()
                    .find(|fixture| fixture.file == file)
                    .ok_or_else(|| format!("{id}: unregistered fixture {file}"))?;
                let source = fs::read_to_string(directory.join(file))?;
                let mut matches = false;
                for version in LanguageVersion::ALL {
                    if !rejected
                        && since != "4.3-or-earlier"
                        && version.to_string().as_str() < since
                    {
                        continue;
                    }
                    let options = ParseOptions {
                        language_version: version,
                        ..Default::default()
                    };
                    let failed = match fixture.stage {
                        Stage::Lexer => lex(&source, &LexOptions::default()).has_errors(),
                        Stage::Expression => parse_expression(&source, &options).has_errors(),
                        Stage::Script => parse(&source, &options).has_errors(),
                    };
                    matches |= failed == rejected;
                }
                if !matches {
                    return Err(format!("{id}: {file} does not provide {field} evidence").into());
                }
            }
        }
    }
    validate_sources(coverage, &ids)?;
    Ok(())
}

fn validate_sources(coverage: &Value, ids: &BTreeSet<&str>) -> Result<()> {
    let mut files = BTreeSet::new();
    let mut expected = BTreeSet::new();
    let mut symbols = BTreeSet::new();
    for file in array(coverage, "sourceFiles")? {
        let version = string(file, "languageVersion")?;
        let path = string(file, "file")?;
        let commit = string(file, "commit")?;
        if !array(coverage, "enginePins")?
            .iter()
            .any(|pin| pin["languageVersion"] == version && pin["commit"] == commit)
            || !files.insert((version, path))
            || string(file, "sha256")?.len() != 64
            || string(file, "url")?
                != format!("https://github.com/godotengine/godot/blob/{commit}/{path}")
        {
            return Err(format!("invalid source pin {version} {path}").into());
        }
        for entry in array(file, "symbols")? {
            let symbol = string(entry, "symbol")?;
            let line = entry["line"]
                .as_u64()
                .filter(|line| *line > 0)
                .ok_or("source symbol requires a line")?;
            if !expected.insert((version, path, symbol, line)) {
                return Err("duplicate source symbol entry".into());
            }
            symbols.insert((path, symbol));
        }
    }
    // All three audited source files must remain represented for all five pins.
    for pin in array(coverage, "enginePins")? {
        let version = string(pin, "languageVersion")?;
        for path in [
            "modules/gdscript/gdscript_parser.cpp",
            "modules/gdscript/gdscript_tokenizer.cpp",
            "modules/gdscript/gdscript_tokenizer.h",
        ] {
            if !files.contains(&(version, path)) {
                return Err(format!("missing source audit {version} {path}").into());
            }
        }
    }
    let mut actual = BTreeSet::new();
    for entry in array(coverage, "symbolDisposition")? {
        let key = (
            string(entry, "languageVersion")?,
            string(entry, "file")?,
            string(entry, "symbol")?,
            entry["line"].as_u64().ok_or("missing disposition line")?,
        );
        if !actual.insert(key) {
            return Err("duplicate symbol disposition".into());
        }
        string(entry, "reason")?;
        let references = array(entry, "rowIds")?;
        match string(entry, "status")? {
            "covered" if !references.is_empty() => {}
            "excluded" => {}
            _ => return Err(format!("unclassified source symbol {}", key.2).into()),
        }
        for id in references {
            if !ids.contains(id.as_str().ok_or("row id must be text")?) {
                return Err(format!("dangling grammar row {id}").into());
            }
        }
    }
    if actual != expected {
        return Err("source symbols and dispositions differ".into());
    }
    let mut annotations = BTreeSet::new();
    for entry in array(coverage, "annotationRegistry")? {
        let version = string(entry, "languageVersion")?;
        let path = string(entry, "file")?;
        let name = string(entry, "name")?;
        let callback = string(entry, "callback")?;
        if !files.contains(&(version, path))
            || !annotations.insert((version, name))
            || !symbols.contains(&(path, callback))
        {
            return Err(format!("invalid annotation registry entry {version} {name}").into());
        }
        string(entry, "syntaxContract")?;
        string(entry, "valueContract")?;
        if array(entry, "targets")?.is_empty() || array(entry, "rowIds")?.is_empty() {
            return Err(format!("{name}: missing annotation targets/evidence").into());
        }
        for id in array(entry, "rowIds")? {
            if !ids.contains(id.as_str().ok_or("row id must be text")?) {
                return Err(format!("{name}: unknown grammar row {id}").into());
            }
        }
    }
    for version in LanguageVersion::ALL {
        if !annotations
            .iter()
            .any(|(profile, _)| *profile == version.to_string())
        {
            return Err(format!("missing {version} annotation registry").into());
        }
    }
    for row in array(coverage, "rows")? {
        for reference in array(row, "upstream")? {
            if !symbols.contains(&(string(reference, "file")?, string(reference, "symbol")?)) {
                return Err(format!("{}: unknown upstream symbol {reference}", row["id"]).into());
            }
        }
    }
    let mut tests = BTreeSet::new();
    for entry in array(coverage, "officialTestInventory")? {
        let path = string(entry, "path")?;
        let commit = string(entry, "commit")?;
        let blob = string(entry, "blob")?;
        string(entry, "category")?;
        string(entry, "reason")?;
        if !path.starts_with("modules/gdscript/tests/scripts/")
            || blob.len() != 40
            || !array(coverage, "enginePins")?
                .iter()
                .any(|pin| pin["commit"] == commit)
            || !tests.insert((path, commit, blob))
        {
            return Err(format!("invalid or duplicate official test {path}").into());
        }
        for id in array(entry, "rowIds")? {
            if !ids.contains(id.as_str().ok_or("row id must be text")?) {
                return Err(format!("{path}: unknown row {id}").into());
            }
        }
        match string(entry, "disposition")? {
            "syntax-probe" => {
                let probe = &entry["probe"];
                if probe["languageVersion"] != "4.7" || probe["lossless"] != true {
                    return Err(format!("{path}: missing lossless probe").into());
                }
                let accepted = probe["rustAccepted"]
                    .as_bool()
                    .ok_or("missing official test probe verdict")?;
                match string(probe, "expected")? {
                    "accepted" if accepted => {}
                    "rejected" if !accepted => {}
                    "boundary" => {}
                    _ => return Err(format!("{path}: unresolved official test discrepancy").into()),
                }
            }
            "semantic-excluded" | "warning-excluded" | "editor-excluded" | "runtime-excluded"
            | "support-excluded" | "syntax-excluded" => {}
            _ => return Err(format!("{path}: unclassified official test").into()),
        }
    }
    if tests.is_empty() {
        return Err("official test inventory is empty".into());
    }
    for row in array(coverage, "rows")? {
        for reference in array(row, "officialTests")? {
            if !tests.contains(&(
                string(reference, "path")?,
                string(reference, "commit")?,
                string(reference, "blob")?,
            )) {
                return Err(format!("{}: official test missing from inventory", row["id"]).into());
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_pin_drift_dangling_evidence_and_unclassified_symbols() {
        let directory = Path::new(env!("CARGO_MANIFEST_DIR")).join("../tests/compatibility");
        let baseline: Value = serde_json::from_str(
            &fs::read_to_string(directory.join("syntax-coverage.json")).unwrap(),
        )
        .unwrap();
        validate(&directory, &baseline).unwrap();
        let mut changed = baseline.clone();
        changed["enginePins"][0]["commit"] = Value::String("0".repeat(40));
        assert!(
            validate(&directory, &changed)
                .unwrap_err()
                .to_string()
                .contains("pins differ")
        );
        let mut changed = baseline.clone();
        changed["rows"][0]["positiveFixtures"][0] = Value::String("fixtures/missing.gd".into());
        assert!(
            validate(&directory, &changed)
                .unwrap_err()
                .to_string()
                .contains("unregistered fixture")
        );
        let mut changed = baseline;
        changed["symbolDisposition"].as_array_mut().unwrap().pop();
        assert!(
            validate(&directory, &changed)
                .unwrap_err()
                .to_string()
                .contains("dispositions differ")
        );
    }
}
