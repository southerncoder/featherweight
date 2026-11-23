//! Port of the TypeScript CLI (`src/cli.ts`, `src/cli/*.ts`): same arguments,
//! messages and exit codes.
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE_ERROR: &str = "featherweight: Invalid arguments. Run 'featherweight --help' for usage.\n";
const READ_ERROR: &str = "featherweight: Unable to read input PDF.\n";
const EXTRACTION_ERROR: &str = "featherweight: Unable to extract native PDF text.\n";
const CONVERSION_ERROR: &str = "featherweight: Unable to convert screenplay.\n";
const SAME_PATH_ERROR: &str = "featherweight: Input and output paths must be different.\n";
const WRITE_ERROR: &str = "featherweight: Unable to write output.\n";
const HELP: &str = "Usage: featherweight <input.pdf> [--format fountain|json|fdx] [--output <path>]\n\
\n\
Convert a PDF screenplay using native embedded text.\n\
\n\
Options:\n\
\x20 --format <fountain|json|fdx>  Output format; Fountain is the default.\n\
\x20 --output <path>          Write output to a file instead of stdout.\n\
\x20 --help                   Show help.\n\
\x20 --version                Show version.\n";

#[derive(Debug, PartialEq)]
enum Command {
    Help,
    Version,
    Convert {
        input: PathBuf,
        format: Format,
        output: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Format {
    Fountain,
    Json,
    Fdx,
}

/// Port of `parseArguments`; `None` is a usage error.
fn parse_arguments(arguments: &[String]) -> Option<Command> {
    match arguments {
        [only] if only == "--help" => return Some(Command::Help),
        [only] if only == "--version" => return Some(Command::Version),
        _ => {}
    }

    let mut format = None;
    let mut output = None;
    let mut inputs = Vec::new();
    let mut remaining = arguments.iter();

    while let Some(argument) = remaining.next() {
        match argument.as_str() {
            "--format" => {
                if format.is_some() {
                    return None;
                }
                format = Some(match remaining.next().map(String::as_str) {
                    Some("fountain") => Format::Fountain,
                    Some("json") => Format::Json,
                    Some("fdx") => Format::Fdx,
                    _ => return None,
                });
            }
            "--output" => {
                if output.is_some() {
                    return None;
                }
                match remaining.next() {
                    Some(value) if !value.starts_with('-') => output = Some(PathBuf::from(value)),
                    _ => return None,
                }
            }
            flag if flag.starts_with('-') => return None,
            input => inputs.push(PathBuf::from(input)),
        }
    }

    match inputs.as_slice() {
        [input] => Some(Command::Convert {
            input: input.clone(),
            format: format.unwrap_or(Format::Fountain),
            output,
        }),
        _ => None,
    }
}

fn fail(message: &str, code: u8) -> ExitCode {
    let _ = std::io::stderr().write_all(message.as_bytes());
    ExitCode::from(code)
}

/// Whether two paths name the same file, including through symlinks and (on
/// Windows) different letter case; on Unix also hard links.
fn same_file(input: &Path, output: &Path) -> bool {
    let (Ok(input_path), Ok(output_path)) = (input.canonicalize(), output.canonicalize()) else {
        return false;
    };
    if input_path == output_path {
        return true;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        if let (Ok(a), Ok(b)) = (std::fs::metadata(input), std::fs::metadata(output)) {
            return a.dev() == b.dev() && a.ino() == b.ino();
        }
    }
    false
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let (input, format, output) = match parse_arguments(&arguments) {
        None => return fail(USAGE_ERROR, 2),
        Some(Command::Help) => return write_stdout(HELP),
        Some(Command::Version) => return write_stdout(&format!("{}\n", env!("CARGO_PKG_VERSION"))),
        Some(Command::Convert { input, format, output }) => (input, format, output),
    };

    if output
        .as_deref()
        .is_some_and(|output| output == input || same_file(&input, output))
    {
        return fail(SAME_PATH_ERROR, 1);
    }

    let Ok(pdf) = std::fs::read(&input) else {
        return fail(READ_ERROR, 1);
    };
    let Ok(inspection) = featherweight::inspect_screenplay_pdf(&pdf) else {
        return fail(EXTRACTION_ERROR, 1);
    };

    if !inspection.pages_needing_ocr.is_empty() {
        let pages: Vec<String> = inspection.pages_needing_ocr.iter().map(usize::to_string).collect();
        let warning = format!(
            "featherweight: OCR is needed for pages {}; converting native text only.\n",
            pages.join(", ")
        );
        if std::io::stderr().write_all(warning.as_bytes()).is_err() {
            return fail(WRITE_ERROR, 1);
        }
    }

    let pages: Vec<usize> = (0..inspection.page_count).collect();
    let converted = match format {
        Format::Fountain => featherweight::screenplay_to_fountain(&pdf, &pages, &[]),
        Format::Json => featherweight::screenplay_to_json(&pdf, &pages, &[]),
        Format::Fdx => featherweight::screenplay_to_fdx(&pdf, &pages, &[]),
    };
    let Ok(artifact) = converted else {
        return fail(CONVERSION_ERROR, 1);
    };

    match output {
        None => write_stdout(&artifact),
        Some(path) => match std::fs::write(path, artifact) {
            Ok(()) => ExitCode::SUCCESS,
            Err(_) => fail(WRITE_ERROR, 1),
        },
    }
}

fn write_stdout(value: &str) -> ExitCode {
    let mut stdout = std::io::stdout().lock();
    match stdout.write_all(value.as_bytes()).and_then(|()| stdout.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => fail(WRITE_ERROR, 1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(arguments: &[&str]) -> Option<Command> {
        parse_arguments(&arguments.iter().map(|a| a.to_string()).collect::<Vec<_>>())
    }

    #[test]
    fn parses_like_the_typescript_cli() {
        assert_eq!(parse(&["--help"]), Some(Command::Help));
        assert_eq!(parse(&["--version"]), Some(Command::Version));
        assert_eq!(
            parse(&["a.pdf", "--format", "fdx", "--output", "a.fdx"]),
            Some(Command::Convert {
                input: "a.pdf".into(),
                format: Format::Fdx,
                output: Some("a.fdx".into())
            })
        );
        assert_eq!(
            parse(&["a.pdf"]),
            Some(Command::Convert {
                input: "a.pdf".into(),
                format: Format::Fountain,
                output: None
            })
        );
        for invalid in [
            &[][..],
            &["a.pdf", "b.pdf"],
            &["a.pdf", "--format", "txt"],
            &["a.pdf", "--format", "json", "--format", "fdx"],
            &["a.pdf", "--output", "-x"],
            &["a.pdf", "--help"],
            &["-v"],
        ] {
            assert_eq!(parse(invalid), None, "{invalid:?}");
        }
    }
}
