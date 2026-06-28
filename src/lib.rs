use anyhow::{Context, Result};
#[cfg(feature = "python")]
use pyo3::prelude::*;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::fs;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub const SCRIPT_SHEBANG: &str = "#!/usr/bin/env -S uv run --script";

#[derive(Clone, Debug, Default, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub ignore: BTreeSet<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct Pyproject {
    tool: Option<Tool>,
}

#[derive(Clone, Debug, Deserialize)]
struct Tool {
    ndol: Option<Config>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Diagnostic {
    pub path: Option<PathBuf>,
    pub line: usize,
    pub column: usize,
    pub code: String,
    pub message: String,
    pub replacement: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LintResult {
    pub diagnostics: Vec<Diagnostic>,
    pub fixed_text: String,
    pub changed: bool,
    pub executable_required: bool,
}

struct PendingDiagnostic {
    code: &'static str,
    line_number: usize,
    column: usize,
    message: &'static str,
    replacement: String,
}

pub fn load_config(start: &Path) -> Result<Config> {
    let mut directory = if start.is_dir() {
        start.to_path_buf()
    } else {
        start
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf()
    };

    loop {
        let candidate = directory.join("pyproject.toml");
        if candidate.exists() {
            let raw = fs::read_to_string(&candidate)
                .with_context(|| format!("failed to read {}", candidate.display()))?;
            let parsed: Pyproject = toml::from_str(&raw)
                .with_context(|| format!("failed to parse {}", candidate.display()))?;
            return Ok(parsed.tool.and_then(|tool| tool.ndol).unwrap_or_default());
        }
        if !directory.pop() {
            return Ok(Config::default());
        }
    }
}

pub fn check_text(source: &str, config: &Config) -> LintResult {
    let file_ignores = file_ignores(source);
    let mut diagnostics = Vec::new();
    let mut fixed_lines = Vec::new();
    let mut changed = false;

    for (index, line) in source.lines().enumerate() {
        let line_number = index + 1;
        let mut fixed = line.to_string();

        if let Some(replacement) = datetime_replacement(line) {
            push_diagnostic(
                &mut diagnostics,
                config,
                &file_ignores,
                line,
                PendingDiagnostic {
                    code: "ND001",
                    line_number,
                    column: 1,
                    message: "use `import datetime as dt` instead of importing datetime members",
                    replacement,
                },
                &mut fixed,
                &mut changed,
            );
        }

        if let Some(replacement) = structlog_replacement(line) {
            push_diagnostic(
                &mut diagnostics,
                config,
                &file_ignores,
                line,
                PendingDiagnostic {
                    code: "ND002",
                    line_number,
                    column: 1,
                    message: "import get_logger from structlog.stdlib",
                    replacement,
                },
                &mut fixed,
                &mut changed,
            );
        }

        if let Some(replacement) = http_replacement(line) {
            push_diagnostic(
                &mut diagnostics,
                config,
                &file_ignores,
                line,
                PendingDiagnostic {
                    code: "ND003",
                    line_number,
                    column: 1,
                    message: "import HTTP client libraries with the `http` alias",
                    replacement,
                },
                &mut fixed,
                &mut changed,
            );
        }

        fixed_lines.push(fixed);
    }

    let mut fixed_text = fixed_lines.join("\n");
    if source.ends_with('\n') {
        fixed_text.push('\n');
    }

    let executable_required = source.contains("/// script");
    if executable_required
        && !source.starts_with(SCRIPT_SHEBANG)
        && !is_ignored(config, &file_ignores, "", "ND004")
    {
        diagnostics.push(Diagnostic {
            path: None,
            line: 1,
            column: 1,
            code: "ND004".to_string(),
            message: format!("files containing `/// script` must start with `{SCRIPT_SHEBANG}`"),
            replacement: Some(format!("{SCRIPT_SHEBANG}\n")),
        });
        fixed_text = format!("{SCRIPT_SHEBANG}\n{fixed_text}");
        changed = true;
    }

    LintResult {
        diagnostics,
        fixed_text,
        changed,
        executable_required,
    }
}

pub fn collect_python_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_file() {
            if is_python_file(path) {
                files.push(path.clone());
            }
            continue;
        }
        for entry in WalkDir::new(path).into_iter().filter_entry(|entry| {
            let name = entry.file_name().to_string_lossy();
            !matches!(name.as_ref(), ".git" | ".venv" | "__pycache__" | "target")
        }) {
            let entry = entry.with_context(|| format!("failed to walk {}", path.display()))?;
            let candidate = entry.path();
            if candidate.is_file() && is_python_file(candidate) {
                files.push(candidate.to_path_buf());
            }
        }
    }
    files.sort();
    Ok(files)
}

pub fn check_paths(paths: &[PathBuf], config: &Config) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    for path in collect_python_files(paths)? {
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut result = check_text(&source, config);
        let executable_missing = result.executable_required && !is_executable(&path)?;
        if executable_missing && !config.ignore.contains("ND004") {
            result.diagnostics.push(Diagnostic {
                path: Some(path.clone()),
                line: 1,
                column: 1,
                code: "ND004".to_string(),
                message: "files containing `/// script` must be executable".to_string(),
                replacement: Some("chmod +x".to_string()),
            });
        }
        for diagnostic in &mut result.diagnostics {
            diagnostic.path = Some(path.clone());
        }
        diagnostics.extend(result.diagnostics);
    }
    Ok(diagnostics)
}

pub fn apply_paths(paths: &[PathBuf], config: &Config) -> Result<Vec<Diagnostic>> {
    let mut diagnostics = Vec::new();
    for path in collect_python_files(paths)? {
        let source = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        let mut result = check_text(&source, config);
        if result.changed {
            fs::write(&path, result.fixed_text.as_bytes())
                .with_context(|| format!("failed to write {}", path.display()))?;
        }
        if result.executable_required && !config.ignore.contains("ND004") {
            make_executable(&path)?;
        }
        for diagnostic in &mut result.diagnostics {
            diagnostic.path = Some(path.clone());
        }
        diagnostics.extend(result.diagnostics);
    }
    Ok(diagnostics)
}

pub fn format_diagnostics(diagnostics: &[Diagnostic]) -> String {
    let mut output = String::new();
    for diagnostic in diagnostics {
        let path = diagnostic
            .path
            .as_ref()
            .map_or_else(|| "<text>".to_string(), |path| path.display().to_string());
        let _ = writeln!(
            output,
            "{}:{}:{} {} {}",
            path, diagnostic.line, diagnostic.column, diagnostic.code, diagnostic.message
        );
    }
    output
}

fn push_diagnostic(
    diagnostics: &mut Vec<Diagnostic>,
    config: &Config,
    file_ignores: &BTreeSet<String>,
    line: &str,
    pending: PendingDiagnostic,
    fixed: &mut String,
    changed: &mut bool,
) {
    let PendingDiagnostic {
        code,
        line_number,
        column,
        message,
        replacement,
    } = pending;

    if is_ignored(config, file_ignores, line, code) {
        return;
    }
    diagnostics.push(Diagnostic {
        path: None,
        line: line_number,
        column,
        code: code.to_string(),
        message: message.to_string(),
        replacement: Some(replacement.clone()),
    });
    *fixed = replacement;
    *changed = true;
}

fn datetime_replacement(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if !trimmed.starts_with("from datetime import ") {
        return None;
    }
    let indent = &line[..line.len() - trimmed.len()];
    Some(format!("{indent}import datetime as dt"))
}

fn structlog_replacement(line: &str) -> Option<String> {
    let trimmed = line.trim_start();
    if trimmed != "from structlog import get_logger" {
        return None;
    }
    let indent = &line[..line.len() - trimmed.len()];
    Some(format!("{indent}from structlog.stdlib import get_logger"))
}

fn http_replacement(line: &str) -> Option<String> {
    const HTTP_LIBRARIES: &[&str] = &["niquests", "requests", "httpx", "aiohttp", "urllib3"];
    let trimmed = line.trim_start();
    if !trimmed.starts_with("import ") {
        return None;
    }
    let import_target = trimmed.strip_prefix("import ")?.trim();
    if import_target.contains(',') || import_target.contains('.') {
        return None;
    }
    let mut parts = import_target.split_whitespace();
    let module = parts.next()?;
    if !HTTP_LIBRARIES.contains(&module) {
        return None;
    }
    match (parts.next(), parts.next(), parts.next()) {
        (None, None, None) => {}
        (Some("as"), Some("http"), None) => return None,
        (Some("as"), Some(_), None) => {}
        _ => return None,
    }
    let indent = &line[..line.len() - trimmed.len()];
    Some(format!("{indent}import {module} as http"))
}

fn file_ignores(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .take(10)
        .filter_map(|line| line.trim().strip_prefix("# ndol: noqa"))
        .flat_map(parse_noqa_codes)
        .collect()
}

fn line_ignores(line: &str) -> BTreeSet<String> {
    let Some((_, comment)) = line.split_once('#') else {
        return BTreeSet::new();
    };
    let comment = comment.trim_start();
    if !comment.starts_with("noqa") {
        return BTreeSet::new();
    }
    parse_noqa_codes(comment.strip_prefix("noqa").unwrap_or_default()).collect()
}

fn parse_noqa_codes(raw: &str) -> impl Iterator<Item = String> + '_ {
    raw.trim_start_matches(':')
        .split(',')
        .map(str::trim)
        .filter(|code| !code.is_empty())
        .map(ToString::to_string)
}

fn is_ignored(config: &Config, file_ignores: &BTreeSet<String>, line: &str, code: &str) -> bool {
    config.ignore.contains(code)
        || config.ignore.contains("ALL")
        || file_ignores.contains(code)
        || file_ignores.contains("ALL")
        || line_ignores(line).contains(code)
        || line_ignores(line).contains("ALL")
}

fn is_python_file(path: &Path) -> bool {
    path.extension().is_some_and(|extension| extension == "py")
}

fn is_executable(path: &Path) -> Result<bool> {
    #[cfg(unix)]
    {
        let mode = fs::metadata(path)
            .with_context(|| format!("failed to stat {}", path.display()))?
            .permissions()
            .mode();
        Ok(mode & 0o111 != 0)
    }

    #[cfg(not(unix))]
    {
        let _ = path;
        Ok(true)
    }
}

fn make_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        let metadata =
            fs::metadata(path).with_context(|| format!("failed to stat {}", path.display()))?;
        let mut permissions = metadata.permissions();
        permissions.set_mode(permissions.mode() | 0o755);
        fs::set_permissions(path, permissions)
            .with_context(|| format!("failed to chmod +x {}", path.display()))?;
    }

    #[cfg(not(unix))]
    {
        let _ = path;
    }

    Ok(())
}

#[cfg(feature = "python")]
#[pyfunction(name = "check_text")]
fn check_text_py(source: &str) -> Vec<(usize, String, String)> {
    check_text(source, &Config::default())
        .diagnostics
        .into_iter()
        .map(|diagnostic| (diagnostic.line, diagnostic.code, diagnostic.message))
        .collect()
}

#[cfg(feature = "python")]
#[pyfunction(name = "apply_text")]
fn apply_text_py(source: &str) -> String {
    check_text(source, &Config::default()).fixed_text
}

#[cfg(feature = "python")]
#[pymodule]
fn _native(module: &Bound<'_, PyModule>) -> PyResult<()> {
    module.add_function(wrap_pyfunction!(check_text_py, module)?)?;
    module.add_function(wrap_pyfunction!(apply_text_py, module)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> Config {
        Config::default()
    }

    #[test]
    fn fixes_datetime_imports() {
        let result = check_text("from datetime import datetime\n", &default_config());
        assert!(result.changed);
        assert_eq!(result.fixed_text, "import datetime as dt\n");
        assert_eq!(result.diagnostics[0].code, "ND001");
    }

    #[test]
    fn fixes_structlog_imports() {
        let result = check_text("from structlog import get_logger\n", &default_config());
        assert_eq!(
            result.fixed_text,
            "from structlog.stdlib import get_logger\n"
        );
    }

    #[test]
    fn fixes_http_aliases() {
        let result = check_text(
            "import requests\nimport httpx as client\n",
            &default_config(),
        );
        assert_eq!(
            result.fixed_text,
            "import requests as http\nimport httpx as http\n"
        );
    }

    #[test]
    fn respects_inline_noqa() {
        let result = check_text(
            "from datetime import datetime  # noqa: ND001\n",
            &default_config(),
        );
        assert!(!result.changed);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn respects_file_noqa() {
        let result = check_text("# ndol: noqa: ND003\nimport requests\n", &default_config());
        assert!(!result.changed);
        assert!(result.diagnostics.is_empty());
    }

    #[test]
    fn inserts_uv_script_shebang() {
        let result = check_text("/// script\n# dependencies = []\n", &default_config());
        assert!(result.fixed_text.starts_with(SCRIPT_SHEBANG));
        assert!(result.executable_required);
    }
}
