use anyhow::{Result, bail};
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::Path;

const TMPL_PACKAGE_JSON: &str = include_str!("../template/package.json");
const TMPL_TSCONFIG: &str = include_str!("../template/tsconfig.json");
const TMPL_CONFIG: &str = include_str!("../template/uzumaki.config.json");
const TMPL_INDEX_TSX: &str = include_str!("../template/src/index.tsx");
const TMPL_LOGO_SVG: &[u8] = include_bytes!("../template/assets/logo.svg");
const TMPL_REACT_SVG: &[u8] = include_bytes!("../template/assets/react.svg");

struct TemplateEntry {
    path: &'static str,
    content: TemplateContent,
}

enum TemplateContent {
    Text(&'static str),
    Bytes(&'static [u8]),
}

const TEMPLATE_ENTRIES: &[TemplateEntry] = &[
    TemplateEntry {
        path: "package.json",
        content: TemplateContent::Text(TMPL_PACKAGE_JSON),
    },
    TemplateEntry {
        path: "tsconfig.json",
        content: TemplateContent::Text(TMPL_TSCONFIG),
    },
    TemplateEntry {
        path: "uzumaki.config.json",
        content: TemplateContent::Text(TMPL_CONFIG),
    },
    TemplateEntry {
        path: "src/index.tsx",
        content: TemplateContent::Text(TMPL_INDEX_TSX),
    },
    TemplateEntry {
        path: "assets/logo.svg",
        content: TemplateContent::Bytes(TMPL_LOGO_SVG),
    },
    TemplateEntry {
        path: "assets/react.svg",
        content: TemplateContent::Bytes(TMPL_REACT_SVG),
    },
];

const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";
const BLUE: &str = "\x1b[38;5;75m";
const GREEN: &str = "\x1b[32m";
const DIM: &str = "\x1b[2m";

fn prompt(label: &str, default: &str) -> Result<String> {
    if default.is_empty() {
        eprint!("{BLUE}?{RESET} {BOLD}{label}{RESET}: ");
    } else {
        eprint!("{BLUE}?{RESET} {BOLD}{label}{RESET} {DIM}({default}){RESET}: ");
    }
    io::stderr().flush()?;

    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;
    let trimmed = line.trim().to_string();

    if trimmed.is_empty() {
        if default.is_empty() {
            bail!("{label} is required");
        }
        Ok(default.to_string())
    } else {
        Ok(trimmed)
    }
}

fn sanitize_name(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_lowercase()
}

fn apply_vars(template: &str, vars: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in vars {
        out = out.replace(&format!("{{{{{key}}}}}"), value);
    }
    out
}

fn write_template_entry(base: &Path, entry: &TemplateEntry, vars: &[(&str, &str)]) -> Result<()> {
    let dest = base.join(entry.path);
    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent)?;
    }
    match entry.content {
        TemplateContent::Text(template) => fs::write(&dest, apply_vars(template, vars))?,
        TemplateContent::Bytes(bytes) => fs::write(&dest, bytes)?,
    }
    Ok(())
}

pub fn cmd_init(target_dir: Option<&str>) -> Result<()> {
    println!("\n{BOLD}{BLUE}Uzumaki{RESET} — Project Setup\n");

    let cwd = std::env::current_dir()?;

    // Project name
    let dir_hint = target_dir.map(String::from).unwrap_or_else(|| {
        cwd.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "my-app".to_string())
    });
    let project_name = prompt("Project name", &sanitize_name(&dir_hint))?;
    let project_name = sanitize_name(&project_name);

    if project_name.is_empty() {
        bail!("project name cannot be empty");
    }

    // Identifier
    let default_id = format!("com.example.{}", project_name.replace('-', "_"));
    let identifier = prompt("Bundle identifier", &default_id)?;

    // Resolve output directory
    let project_dir = match target_dir {
        Some(d) => cwd.join(d),
        None => cwd.join(&project_name),
    };

    // Check if dir exists and is non-empty
    if project_dir.is_dir() {
        if project_dir.join("package.json").is_file() {
            bail!(
                "{} already contains package.json; refusing to create a project inside an existing package",
                project_dir.display()
            );
        }

        let has_entries = fs::read_dir(&project_dir)?.next().is_some();
        if has_entries {
            bail!("directory {} is not empty", project_dir.display());
        }
    }

    // Write files
    let vars: Vec<(&str, &str)> =
        vec![("PROJECT_NAME", &project_name), ("IDENTIFIER", &identifier)];

    for entry in TEMPLATE_ENTRIES {
        write_template_entry(&project_dir, entry, &vars)?;
    }

    // Summary
    let rel = project_dir.strip_prefix(&cwd).unwrap_or(&project_dir);

    println!();
    for entry in TEMPLATE_ENTRIES {
        println!("  {GREEN}created{RESET} {}/{}", rel.display(), entry.path);
    }

    println!("\n{BOLD}Next steps:{RESET}\n");
    println!("  cd {project_name}");
    println!("  pnpm install");
    println!("  pnpm dev");
    println!();

    Ok(())
}
