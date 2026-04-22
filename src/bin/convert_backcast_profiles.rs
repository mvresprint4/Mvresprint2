use quick_xml::events::{BytesStart, Event};
use quick_xml::escape::unescape;
use quick_xml::Reader;
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashMap};
use std::fs::{self, File};
use std::io::{Cursor, Read, Write};
use std::path::{Path, PathBuf};
use zip::read::ZipArchive;

#[derive(Debug, Serialize)]
struct SheetManifest {
    sheet_index: usize,
    sheet_name: String,
    sheet_xml_path: String,
    csv_path: String,
    rows: usize,
    max_columns: usize,
    sha256: String,
}

#[derive(Debug, Serialize)]
struct YearManifest {
    source_zip: String,
    source_xlsx: String,
    year: String,
    sheets: Vec<SheetManifest>,
}

#[derive(Debug, Serialize)]
struct SummaryManifest {
    generated_by: String,
    archives_processed: usize,
    manifests: Vec<YearManifest>,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo_root = std::env::current_dir()?;
    let grid_dir = repo_root.join("Grid and Market Conditions");
    let out_root = repo_root.join("artifacts").join("backcasted_load_profiles");
    fs::create_dir_all(&out_root)?;

    let mut archives: Vec<PathBuf> = fs::read_dir(&grid_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("ERCOT-Backcasted-Load-Profiles-") && n.ends_with(".zip"))
                    .unwrap_or(false)
        })
        .collect();
    archives.sort();

    if archives.is_empty() {
        return Err("No ERCOT backcasted profile archives found".into());
    }

    let mut manifests = Vec::new();
    for archive_path in &archives {
        println!("Converting {}", archive_path.display());
        let manifest = convert_archive(&repo_root, archive_path, &out_root)?;
        manifests.push(manifest);
    }

    let summary = SummaryManifest {
        generated_by: "src/bin/convert_backcast_profiles.rs".to_string(),
        archives_processed: manifests.len(),
        manifests,
    };
    let summary_path = out_root.join("summary_manifest.json");
    fs::write(summary_path, serde_json::to_vec_pretty(&summary)?)?;
    println!("Done");
    Ok(())
}

fn convert_archive(
    repo_root: &Path,
    zip_path: &Path,
    out_root: &Path,
) -> Result<YearManifest, Box<dyn std::error::Error>> {
    let year = detect_year(
        zip_path
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or("Invalid archive filename")?,
    )
    .unwrap_or_else(|| "unknown".to_string());

    let year_dir = out_root.join(&year);
    if year_dir.exists() {
        fs::remove_dir_all(&year_dir)?;
    }
    fs::create_dir_all(&year_dir)?;

    let source_zip_rel = relative_slash_path(repo_root, zip_path);

    let xlsx_bytes;
    let xlsx_name;
    {
        let file = File::open(zip_path)?;
        let mut outer = ZipArchive::new(file)?;
        let mut xlsx_entries: Vec<String> = (0..outer.len())
            .filter_map(|i| outer.by_index(i).ok().map(|f| f.name().to_string()))
            .filter(|n| n.to_ascii_lowercase().ends_with(".xlsx"))
            .collect();
        xlsx_entries.sort();
        let pick = xlsx_entries
            .first()
            .ok_or("No .xlsx entry found in backcast archive")?
            .to_string();
        let mut f = outer.by_name(&pick)?;
        let mut data = Vec::new();
        f.read_to_end(&mut data)?;
        xlsx_bytes = data;
        xlsx_name = pick;
    }

    let mut xlsx = ZipArchive::new(Cursor::new(xlsx_bytes))?;
    let shared_strings = load_shared_strings(&mut xlsx)?;
    let sheets = load_sheet_specs(&mut xlsx)?;

    let mut sheet_manifests = Vec::new();
    for (idx, sheet) in sheets.iter().enumerate() {
        let safe_name = sanitize_name(&sheet.name);
        let file_name = format!("{:02}_{}.csv", idx + 1, safe_name);
        let out_csv = year_dir.join(file_name);
        let stats = write_sheet_csv(&mut xlsx, &sheet.path, &shared_strings, &out_csv)?;

        sheet_manifests.push(SheetManifest {
            sheet_index: idx + 1,
            sheet_name: sheet.name.clone(),
            sheet_xml_path: sheet.path.clone(),
            csv_path: relative_slash_path(repo_root, &out_csv),
            rows: stats.0,
            max_columns: stats.1,
            sha256: sha256_path(&out_csv)?,
        });
    }

    let manifest = YearManifest {
        source_zip: source_zip_rel,
        source_xlsx: xlsx_name,
        year,
        sheets: sheet_manifests,
    };
    fs::write(
        year_dir.join("manifest.json"),
        serde_json::to_vec_pretty(&manifest)?,
    )?;
    Ok(manifest)
}

fn detect_year(name: &str) -> Option<String> {
    let bytes = name.as_bytes();
    for i in 0..bytes.len().saturating_sub(3) {
        let s = &name[i..i + 4];
        if s.chars().all(|c| c.is_ascii_digit()) {
            return Some(s.to_string());
        }
    }
    None
}

#[derive(Debug)]
struct SheetSpec {
    name: String,
    path: String,
}

fn load_sheet_specs<R: Read + std::io::Seek>(
    zip: &mut ZipArchive<R>,
) -> Result<Vec<SheetSpec>, Box<dyn std::error::Error>> {
    let workbook_xml = read_zip_entry(zip, "xl/workbook.xml")?;
    let rels_xml = read_zip_entry(zip, "xl/_rels/workbook.xml.rels")?;

    let mut rel_map: HashMap<String, String> = HashMap::new();
    {
        let mut reader = Reader::from_str(&rels_xml);
        reader.config_mut().trim_text(true);
        loop {
            match reader.read_event()? {
                Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"Relationship" => {
                    let mut rid = String::new();
                    let mut target = String::new();
                    for a in e.attributes().flatten() {
                        let key = a.key.as_ref();
                        if key == b"Id" {
                            rid = String::from_utf8_lossy(a.value.as_ref()).to_string();
                        } else if key == b"Target" {
                            target = String::from_utf8_lossy(a.value.as_ref())
                                .replace('\\', "/");
                        }
                    }
                    if !rid.is_empty() && !target.is_empty() {
                        rel_map.insert(rid, target);
                    }
                }
                Event::Eof => break,
                _ => {}
            }
        }
    }

    let mut sheets = Vec::new();
    let mut reader = Reader::from_str(&workbook_xml);
    reader.config_mut().trim_text(true);
    loop {
        match reader.read_event()? {
            Event::Start(e) | Event::Empty(e) if e.name().as_ref() == b"sheet" => {
                let mut name = String::new();
                let mut rid = String::new();
                for a in e.attributes().flatten() {
                    let key = a.key.as_ref();
                    let val = String::from_utf8_lossy(a.value.as_ref()).to_string();
                    if key == b"name" {
                        name = val;
                    } else if key == b"r:id" || key.ends_with(b":id") {
                        rid = val;
                    }
                }
                if let Some(target) = rel_map.get(&rid) {
                    if target.starts_with("worksheets/") {
                        sheets.push(SheetSpec {
                            name,
                            path: format!("xl/{}", target),
                        });
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(sheets)
}

fn load_shared_strings<R: Read + std::io::Seek>(
    zip: &mut ZipArchive<R>,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let xml = match read_zip_entry(zip, "xl/sharedStrings.xml") {
        Ok(v) => v,
        Err(_) => return Ok(Vec::new()),
    };
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(false);

    let mut strings = Vec::new();
    let mut in_si = false;
    let mut in_t = false;
    let mut current = String::new();
    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == b"si" => {
                in_si = true;
                current.clear();
            }
            Event::End(e) if e.name().as_ref() == b"si" => {
                in_si = false;
                strings.push(current.clone());
            }
            Event::Start(e) if in_si && e.name().as_ref() == b"t" => in_t = true,
            Event::End(e) if e.name().as_ref() == b"t" => in_t = false,
            Event::Text(t) if in_si && in_t => {
                current.push_str(&decode_xml_text(t.as_ref())?);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    Ok(strings)
}

fn write_sheet_csv<R: Read + std::io::Seek>(
    zip: &mut ZipArchive<R>,
    sheet_path: &str,
    shared: &[String],
    out_csv: &Path,
) -> Result<(usize, usize), Box<dyn std::error::Error>> {
    let xml = read_zip_entry(zip, sheet_path)?;
    let mut reader = Reader::from_str(&xml);
    reader.config_mut().trim_text(false);

    let mut file = File::create(out_csv)?;
    let mut row_map: BTreeMap<usize, String> = BTreeMap::new();
    let mut row_max = 0usize;
    let mut rows = 0usize;
    let mut max_cols = 0usize;

    let mut in_cell = false;
    let mut cell_col = 1usize;
    let mut cell_type = String::new();
    let mut capturing = false;
    let mut cell_raw = String::new();

    loop {
        match reader.read_event()? {
            Event::Start(e) if e.name().as_ref() == b"row" => {
                row_map.clear();
                row_max = 0;
            }
            Event::End(e) if e.name().as_ref() == b"row" => {
                if row_max > max_cols {
                    max_cols = row_max;
                }
                write_row_csv(&mut file, &row_map, row_max)?;
                rows += 1;
            }
            Event::Start(e) if e.name().as_ref() == b"c" => {
                let (col, ctype) = parse_cell_attrs(&e);
                in_cell = true;
                cell_col = col;
                cell_type = ctype;
                cell_raw.clear();
                if cell_col > row_max {
                    row_max = cell_col;
                }
            }
            Event::End(e) if e.name().as_ref() == b"c" && in_cell => {
                let value = if cell_type == "s" {
                    if let Ok(idx) = cell_raw.parse::<usize>() {
                        shared.get(idx).cloned().unwrap_or_default()
                    } else {
                        String::new()
                    }
                } else {
                    cell_raw.clone()
                };
                row_map.insert(cell_col, value);
                in_cell = false;
                capturing = false;
            }
            Event::Start(e) if in_cell && (e.name().as_ref() == b"v" || e.name().as_ref() == b"t") => {
                capturing = true;
            }
            Event::End(e) if e.name().as_ref() == b"v" || e.name().as_ref() == b"t" => {
                capturing = false;
            }
            Event::Text(t) if in_cell && capturing => {
                cell_raw.push_str(&decode_xml_text(t.as_ref())?);
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok((rows, max_cols))
}

fn parse_cell_attrs(cell: &BytesStart<'_>) -> (usize, String) {
    let mut col = 1usize;
    let mut ctype = String::new();
    for a in cell.attributes().flatten() {
        let key = a.key.as_ref();
        let val = String::from_utf8_lossy(a.value.as_ref()).to_string();
        if key == b"r" {
            col = col_index_from_ref(&val);
        } else if key == b"t" {
            ctype = val;
        }
    }
    (col, ctype)
}

fn decode_xml_text(bytes: &[u8]) -> Result<String, Box<dyn std::error::Error>> {
    let raw = String::from_utf8_lossy(bytes);
    Ok(unescape(&raw)?.into_owned())
}

fn write_row_csv(
    file: &mut File,
    row_map: &BTreeMap<usize, String>,
    max_col: usize,
) -> Result<(), Box<dyn std::error::Error>> {
    if max_col == 0 {
        file.write_all(b"\n")?;
        return Ok(());
    }
    for i in 1..=max_col {
        if i > 1 {
            file.write_all(b",")?;
        }
        if let Some(v) = row_map.get(&i) {
            file.write_all(escape_csv(v).as_bytes())?;
        }
    }
    file.write_all(b"\n")?;
    Ok(())
}

fn escape_csv(value: &str) -> String {
    let needs_quote =
        value.contains(',') || value.contains('"') || value.contains('\n') || value.contains('\r');
    if !needs_quote {
        return value.to_string();
    }
    let escaped = value.replace('"', "\"\"");
    format!("\"{}\"", escaped)
}

fn col_index_from_ref(cell_ref: &str) -> usize {
    let letters: String = cell_ref
        .chars()
        .take_while(|c| c.is_ascii_alphabetic())
        .collect();
    if letters.is_empty() {
        return 1;
    }
    let mut n = 0usize;
    for ch in letters.chars() {
        let upper = ch.to_ascii_uppercase() as u8;
        n = (n * 26) + (upper - b'A' + 1) as usize;
    }
    if n == 0 {
        1
    } else {
        n
    }
}

fn sanitize_name(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for ch in name.chars() {
        if ch.is_ascii_alphanumeric() || ch == '.' || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "sheet".to_string()
    } else {
        out
    }
}

fn sha256_path(path: &Path) -> Result<String, Box<dyn std::error::Error>> {
    let mut f = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = f.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex::encode(hasher.finalize()))
}

fn read_zip_entry<R: Read + std::io::Seek>(
    zip: &mut ZipArchive<R>,
    name: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut f = zip.by_name(name)?;
    let mut s = String::new();
    f.read_to_string(&mut s)?;
    Ok(s)
}

fn relative_slash_path(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
