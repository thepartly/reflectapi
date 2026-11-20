use anyhow::Context;
use chrono::NaiveDate;
use reflectapi::diff::{Acknowledgement, Change, Date, Kind, Options, Severity};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

#[derive(clap::Args)]
pub struct Args {
    /// The schema clients were built against
    old: PathBuf,

    /// The schema the server will serve
    new: PathBuf,

    /// Print the changes as JSON
    #[arg(long)]
    json: bool,

    /// Also print changes that don't break clients
    #[arg(short, long)]
    all: bool,

    /// The date removals are judged on, YYYY-MM-DD. Removing a deprecated
    /// item is compatible once its sunset is on or before it. Defaults to
    /// today in UTC
    #[arg(long, value_name = "DATE")]
    today: Option<NaiveDate>,

    /// Also require a removed item to have been deprecated, by its
    /// `deprecated_since` date, at least this many days before the date
    #[arg(long, value_name = "DAYS")]
    min_notice_days: Option<u32>,

    /// The schema as it was a notice period ago, such as the default branch a week back.
    /// A deprecated item may be removed only if it was already deprecated in it
    #[arg(long, value_name = "SCHEMA")]
    deprecated_in: Option<PathBuf>,

    /// Compare only functions with at least one of these tags
    #[arg(short, long, value_delimiter = ',')]
    include_tags: Vec<String>,

    /// Leave out functions with any of these tags, such as those left out of a published spec
    #[arg(short, long, value_delimiter = ',')]
    exclude_tags: Vec<String>,

    /// Report breaking changes of these kinds as warnings, which don't fail the diff
    #[arg(
        long,
        value_name = "KIND",
        value_delimiter = ',',
        value_parser = clap::builder::TypedValueParser::try_map(
            clap::builder::PossibleValuesParser::new(Kind::ALL.iter().map(|kind| kind.as_str())),
            |kind: String| kind.parse::<Kind>(),
        ),
    )]
    warn: Vec<Kind>,

    /// A file of accepted breaking changes, or a directory of them (its `.md` files).
    /// Each line `- <function> <location> [<kind>]: <reason>` accepts one change.
    /// May be given more than once
    #[arg(long, value_name = "PATH")]
    acknowledged: Vec<PathBuf>,
}

pub enum Breaks {
    Yes,
    No,
}

/// Whether a breaking change is left that no acknowledgement accepts.
pub fn run(args: Args) -> anyhow::Result<Breaks> {
    let read = |path: &Path| -> anyhow::Result<reflectapi::Schema> {
        let json = std::fs::read_to_string(path)
            .context(format!("Failed to read schema file: {path:?}"))?;
        crate::read_schema(&json).context(format!("Failed to read {path:?}"))
    };
    let today = args
        .today
        .unwrap_or_else(|| chrono::Utc::now().date_naive());
    let notice_cutoff = args
        .min_notice_days
        .map(|days| {
            today
                .checked_sub_days(chrono::Days::new(days.into()))
                .context(format!("{days} days before {today} is out of range"))
        })
        .transpose()?;
    let mut options = Options::default();
    options.today = Some(date(today)?);
    options.notice_cutoff = notice_cutoff.map(date).transpose()?;
    options.include_tags = BTreeSet::from_iter(args.include_tags);
    options.exclude_tags = BTreeSet::from_iter(args.exclude_tags);
    options.warn = BTreeSet::from_iter(args.warn);
    options.deprecated_in = args.deprecated_in.as_deref().map(read).transpose()?;
    let acknowledgements = read_acknowledgements(&args.acknowledged)?;
    let mut changes = reflectapi::diff::diff(&read(&args.old)?, &read(&args.new)?, &options);
    for unused in reflectapi::diff::acknowledge(&mut changes, &acknowledgements) {
        eprintln!(
            "warning: {}: acknowledges no breaking change: {}",
            unused.source, unused.key
        );
    }

    if args.json {
        let written = serde_json::to_writer_pretty(std::io::stdout().lock(), &changes);
        match written {
            Err(e) if e.io_error_kind() == Some(std::io::ErrorKind::BrokenPipe) => {}
            written => {
                written?;
                println!();
            }
        }
    } else {
        print_changes(&changes, args.all);
    }

    let unaccepted = changes
        .iter()
        .any(|c| c.severity == Severity::Breaking && c.acknowledged.is_none());
    Ok(if unaccepted { Breaks::Yes } else { Breaks::No })
}

fn date(date: NaiveDate) -> anyhow::Result<Date> {
    date.format("%Y-%m-%d")
        .to_string()
        .parse()
        .map_err(anyhow::Error::msg)
}

fn read_acknowledgements(paths: &[PathBuf]) -> anyhow::Result<Vec<Acknowledgement>> {
    let mut files = Vec::new();
    for path in paths {
        if path.is_dir() {
            let mut entries = std::fs::read_dir(path)
                .context(format!(
                    "Failed to read acknowledgements directory {path:?}"
                ))?
                .map(|entry| entry.map(|e| e.path()))
                .collect::<Result<Vec<_>, _>>()?;
            entries.retain(|p| p.extension().is_some_and(|e| e == "md"));
            entries.sort();
            files.extend(entries);
        } else {
            files.push(path.clone());
        }
    }
    let mut acknowledgements = Vec::new();
    for file in files {
        let text = std::fs::read_to_string(&file)
            .context(format!("Failed to read acknowledgements file {file:?}"))?;
        acknowledgements.extend(reflectapi::diff::parse_acknowledgements(
            &text,
            &file.display().to_string(),
        )?);
    }
    Ok(acknowledgements)
}

fn print_changes(changes: &[Change], all: bool) {
    let section = |title: &str, selected: Vec<String>| {
        if !selected.is_empty() {
            println!("{} {title}:", selected.len());
            for line in selected {
                println!("  {line}");
            }
            println!();
        }
    };
    let (acknowledged, blocking): (Vec<_>, Vec<_>) = changes
        .iter()
        .filter(|c| c.severity == Severity::Breaking)
        .partition(|c| c.acknowledged.is_some());

    section(
        "breaking change(s)",
        blocking.iter().map(|c| c.to_string()).collect(),
    );
    section(
        "warning(s)",
        changes
            .iter()
            .filter(|c| c.severity == Severity::Warning)
            .map(|c| format!("{c} [{}]", c.kind))
            .collect(),
    );
    section(
        "acknowledged breaking change(s)",
        acknowledged
            .iter()
            .map(|c| {
                format!(
                    "{c} (acknowledged: {})",
                    c.acknowledged.as_deref().unwrap_or_default()
                )
            })
            .collect(),
    );
    if all {
        section(
            "compatible change(s)",
            changes
                .iter()
                .filter(|c| c.severity == Severity::Compatible)
                .map(|c| c.to_string())
                .collect(),
        );
    }
    if blocking.is_empty() && acknowledged.is_empty() {
        println!("no breaking changes");
    } else if blocking.is_empty() {
        println!("no unacknowledged breaking changes");
    } else {
        println!(
            "To accept these, add the lines below to an acknowledgement file passed with \
             --acknowledged, replacing each <...> with why it is safe, which consumers you \
             checked, and the ticket:"
        );
        println!();
        for change in blocking {
            println!("- {}: <reason, consumers checked, ticket>", change.key);
        }
    }
}
