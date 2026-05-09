#![cfg_attr(test, allow(clippy::unwrap_used, clippy::expect_used))]
use clap::Parser;
use streeem_domain::command_spec::{CommandSpec, CommandSpecError};
use streeem_domain::rows_hint::RowsHint;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "streeem",
    version,
    about = "Host multiple terminals in a staggered grid"
)]
pub struct Cli {
    #[arg(long)]
    pub columns: Option<u16>,
    #[arg(long)]
    pub scrollback: Option<usize>,
    #[arg(long)]
    pub min_tile_width: Option<u16>,
    #[arg(long)]
    pub rows: Vec<u16>,
    #[arg(long = "name")]
    pub names: Vec<String>,
    #[arg(value_name = "COMMAND", num_args = 1..)]
    pub commands: Vec<String>,
}

impl Cli {
    pub fn into_specs(self) -> Result<Vec<CommandSpec>, CliError> {
        let default_rows = RowsHint::default();
        let mut rows_iter = self.rows.into_iter();
        let mut names_iter = self.names.into_iter();
        let mut specs = Vec::with_capacity(self.commands.len());
        for cmd in self.commands {
            let rh = match rows_iter.next() {
                Some(n) => RowsHint::new(n).map_err(|_| CliError::BadRows(n))?,
                None => default_rows,
            };
            let name = names_iter.next();
            specs.push(CommandSpec::new_with_name(cmd, name, rh).map_err(CliError::Spec)?);
        }
        Ok(specs)
    }
}

#[derive(Debug)]
pub enum CliError {
    BadRows(u16),
    Spec(CommandSpecError),
}

impl std::fmt::Display for CliError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CliError::BadRows(n) => write!(f, "invalid --rows value: {n}"),
            CliError::Spec(e) => write!(f, "invalid command spec: {e:?}"),
        }
    }
}

impl std::error::Error for CliError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Cli {
        let mut full = vec!["streeem"];
        full.extend_from_slice(args);
        Cli::try_parse_from(full).expect("parse failed")
    }

    #[test]
    fn parses_a_single_command_with_default_rows() {
        let cli = parse(&["echo hi"]);
        let specs = cli.into_specs().unwrap();
        assert_eq!(specs.len(), 1);
        assert_eq!(specs[0].command, "echo hi");
        assert_eq!(specs[0].rows_hint, RowsHint::default());
    }

    #[test]
    fn applies_rows_in_order() {
        let cli = parse(&["--rows", "20", "--rows", "8", "a", "b"]);
        let specs = cli.into_specs().unwrap();
        assert_eq!(specs[0].rows_hint.value(), 20);
        assert_eq!(specs[1].rows_hint.value(), 8);
    }

    #[test]
    fn parses_columns_override() {
        let cli = parse(&["--columns", "4", "a"]);
        assert_eq!(cli.columns, Some(4));
    }

    #[test]
    fn applies_names_in_order() {
        let cli = parse(&["--name", "alpha", "--name", "beta", "a", "b"]);
        let specs = cli.into_specs().unwrap();
        assert_eq!(specs[0].name, Some("alpha".to_string()));
        assert_eq!(specs[1].name, Some("beta".to_string()));
    }

    #[test]
    fn name_defaults_to_none_when_not_provided() {
        let cli = parse(&["echo hi"]);
        let specs = cli.into_specs().unwrap();
        assert_eq!(specs[0].name, None);
    }
}
