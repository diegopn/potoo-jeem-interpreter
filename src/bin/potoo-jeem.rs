use std::io::{self, Write};

use potoo_jeem_interpreter::{Circle, Host, Interpreter, check, parse_with_library_loader};

struct CliHost;

impl Host for CliHost {
    fn input(&mut self) -> Result<String, String> {
        let mut line = String::new();
        io::stdin()
            .read_line(&mut line)
            .map_err(|error| error.to_string())?;
        Ok(line.trim_end_matches(['\r', '\n']).to_owned())
    }

    fn output(&mut self, text: &str) -> Result<(), String> {
        print!("{text}");
        io::stdout().flush().map_err(|error| error.to_string())
    }

    fn draw_circle(&mut self, _circle: Circle) -> Result<(), String> {
        Err("drawing is unavailable in the command-line host".to_owned())
    }
}

fn main() {
    if let Err(message) = run() {
        eprintln!("{message}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| "usage: potoo-jeem <source-file>".to_owned())?;
    let source = std::fs::read_to_string(&path).map_err(|error| format!("{path}: {error}"))?;
    let source_directory = std::path::Path::new(&path)
        .parent()
        .filter(|directory| !directory.as_os_str().is_empty())
        .unwrap_or_else(|| std::path::Path::new("."))
        .to_owned();
    let executable_library_directory = std::env::current_exe()
        .ok()
        .and_then(|executable| executable.parent().map(|parent| parent.join("common libs")));
    let program = parse_with_library_loader(&source, |library| {
        if library.contains('/') || library.contains('\\') || library == "." || library == ".." {
            return Err("library name must be a file name without a path".to_owned());
        }
        let file_name = format!("{library}.jeem");
        for directory in std::iter::once(Some(source_directory.as_path()))
            .chain(std::iter::once(executable_library_directory.as_deref()))
            .flatten()
        {
            let candidate = directory.join(&file_name);
            match std::fs::read_to_string(&candidate) {
                Ok(source) => return Ok(Some(source)),
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(format!("{}: {error}", candidate.display())),
            }
        }
        Ok(None)
    })
    .map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    check(&program).map_err(|diagnostics| {
        diagnostics
            .into_iter()
            .map(|diagnostic| diagnostic.to_string())
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    Interpreter::new(CliHost)
        .run(&program)
        .map_err(|error| error.to_string())?;
    Ok(())
}
