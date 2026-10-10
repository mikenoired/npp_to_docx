use anyhow::{Context, Result};
use encoding_rs::WINDOWS_1251;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

pub const DATABASES: [&str; 2] = ["PLS_ANA_CONF", "PLS_BIN_CONF"];

pub fn normalize(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_matches(['\'', '"'])
        .trim()
        .to_string()
}

#[derive(Default)]
pub struct DescriptionIndex {
    exact: HashMap<String, String>,
    base: HashMap<String, (i32, String)>,
}

pub fn prepare(input: &Path) -> Result<Vec<PathBuf>> {
    DATABASES
        .iter()
        .map(|name| {
            let csv = input.join(format!("{name}.csv"));
            if csv.is_file() {
                return Ok(csv);
            }
            let dump = input.join(format!("{name}.dmp"));
            let bytes = fs::read(&dump)
                .with_context(|| format!("Не найден исходный файл БД: {}", dump.display()))?;
            let (text, _, _) = WINDOWS_1251.decode(&bytes);
            let normalized = text.replace("\r\n", "\n").replace('\r', "\n");
            let mut lines = normalized.lines();
            let header = lines
                .find(|line| line.starts_with("*#"))
                .context("Не найдена строка заголовков *#...")?;
            let headers: Vec<&str> = header[2..].split('|').collect();
            let mut writer = csv::WriterBuilder::new().from_writer(Vec::new());
            writer.write_record(&headers)?;
            for line in lines.filter(|line| !line.is_empty() && !line.starts_with('*')) {
                let mut fields: Vec<&str> = line.split('|').collect();
                fields.resize(headers.len(), "");
                writer.write_record(fields)?;
            }
            let mut output = vec![0xef, 0xbb, 0xbf];
            output.extend(writer.into_inner()?);
            fs::write(&csv, output)?;
            Ok(csv)
        })
        .collect()
}

fn score(source: &str, pvid: &str, base: &str) -> i32 {
    if pvid == base {
        return 1000;
    }
    let prefix = format!("{base}_");
    let suffix = if pvid.starts_with(&prefix) {
        &pvid[base.len()..]
    } else {
        ""
    };
    match suffix {
        "_XQ01" => return if source == "PLS_ANA_CONF" { 950 } else { 900 },
        "_F0" => return 930,
        "_Z0" => return 920,
        "_OU" => return 910,
        "_B0" => return 900,
        _ => {}
    }
    let upper = suffix.to_ascii_uppercase();
    for (prefix, score) in [
        ("_XA", 860),
        ("_XG", 840),
        ("_ZV", 820),
        ("_XB", 300),
        ("_XM", 250),
        ("_ST", 200),
        ("_ER", 150),
    ] {
        if let Some(rest) = upper.strip_prefix(prefix)
            && !rest.is_empty()
            && rest.bytes().all(|c| c.is_ascii_digit())
        {
            return score;
        }
    }
    (700 - suffix.len() as i32).max(100)
}

impl DescriptionIndex {
    pub fn load(input: &Path) -> Result<Self> {
        let mut index = Self::default();
        for path in prepare(input)? {
            let source = path.file_stem().unwrap().to_string_lossy();
            let mut reader = csv::ReaderBuilder::new().flexible(true).from_path(&path)?;
            let headers = reader.headers()?.clone();
            let columns: Vec<usize> = ["PVID", "PVDESCRIPTION", "PVTEXT", "PLC_ITEMID"]
                .iter()
                .map(|name| {
                    headers
                        .iter()
                        .position(|v| v.trim_start_matches('\u{feff}') == *name)
                        .with_context(|| format!("В {} отсутствует колонка {name}", path.display()))
                })
                .collect::<Result<_>>()?;
            for row in reader.records() {
                let row = row?;
                let values: Vec<String> = columns
                    .iter()
                    .map(|i| normalize(row.get(*i).unwrap_or("")))
                    .collect();
                let pvid = &values[0];
                let description = if values[1].is_empty() {
                    &values[2]
                } else {
                    &values[1]
                };
                let base = &values[3];
                if pvid.is_empty() || description.is_empty() {
                    continue;
                }
                index.exact.insert(pvid.clone(), description.clone());
                if !base.is_empty() {
                    let score = score(&source, pvid, base);
                    let bucket = index
                        .base
                        .entry(base.clone())
                        .or_insert((i32::MIN, String::new()));
                    if score > bucket.0 {
                        *bucket = (score, description.clone());
                    }
                }
            }
        }
        Ok(index)
    }

    pub fn lookup(&self, kks: Option<&str>) -> Option<String> {
        let value = normalize(kks?);
        self.exact
            .get(&value)
            .cloned()
            .or_else(|| self.base.get(&value).map(|v| v.1.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cp1251_dump_and_ranked_fallback() -> Result<()> {
        let tmp = tempfile::tempdir()?;
        let dump = "*#PVID|PVDESCRIPTION|PVTEXT|PLC_ITEMID\nK_XB01|Состояние||K\nK_XQ01|Насос||K\nE||Резерв|E\n";
        let (encoded, _, _) = WINDOWS_1251.encode(dump);
        for name in DATABASES {
            fs::write(tmp.path().join(format!("{name}.dmp")), &encoded)?;
        }
        let db = DescriptionIndex::load(tmp.path())?;
        assert_eq!(db.lookup(Some("K")).as_deref(), Some("Насос"));
        assert_eq!(db.lookup(Some("K_XB01")).as_deref(), Some("Состояние"));
        assert_eq!(db.lookup(Some(" E ")).as_deref(), Some("Резерв"));
        assert_eq!(db.lookup(Some("absent")), None);
        Ok(())
    }
}
