use std::path::Path;

const MAX_LINES: usize = 500;

fn visit_rs_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, out);
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn src_rust_files_stay_under_line_limit() {
    let mut files = Vec::new();
    visit_rs_files(Path::new("src"), &mut files);
    files.sort();

    let mut violations = Vec::new();
    for path in files {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let line_count = content.lines().count();
        if line_count > MAX_LINES {
            violations.push((path, line_count));
        }
    }

    assert!(
        violations.is_empty(),
        "{} file(s) exceed {} lines:\n{}",
        violations.len(),
        MAX_LINES,
        violations
            .iter()
            .map(|(path, lines)| format!("- {} ({})", path.display(), lines))
            .collect::<Vec<_>>()
            .join("\n")
    );
}
