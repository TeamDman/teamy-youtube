use arbitrary::Arbitrary;
use eyre::Context;
use facet::Facet;
use facet_pretty::ColorMode;
use facet_pretty::PrettyPrinter;
use std::io::IsTerminal;
use std::io::Write;
use std::io::{self};

#[derive(Arbitrary, Facet, Clone, Copy, Debug, Default, PartialEq, Eq)]
#[facet(rename_all = "kebab-case")]
#[repr(u8)]
pub enum OutputFormat {
    #[default]
    Text,
    Json,
    Csv,
}

pub struct CliOutput {
    value: Option<Box<dyn CliOutputValue>>,
    pub elapsed_duration_warn_threshold: Option<std::time::Duration>,
}

impl core::fmt::Debug for CliOutput {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("CliOutput")
            .field("has_value", &self.value.is_some())
            .field(
                "elapsed_duration_warn_threshold",
                &self.elapsed_duration_warn_threshold,
            )
            .finish()
    }
}

trait CliOutputValue {
    fn render(
        &self,
        format: OutputFormat,
        stdout_is_terminal: bool,
    ) -> eyre::Result<Option<String>>;
}

struct FacetCliOutput<T> {
    value: T,
}

impl CliOutput {
    #[must_use]
    pub const fn none(elapsed_duration_warn_threshold: Option<std::time::Duration>) -> Self {
        Self {
            value: None,
            elapsed_duration_warn_threshold,
        }
    }

    #[must_use]
    pub fn facet<T>(value: T, elapsed_duration_warn_threshold: Option<std::time::Duration>) -> Self
    where
        T: Facet<'static> + 'static,
    {
        Self {
            value: Some(Box::new(FacetCliOutput { value })),
            elapsed_duration_warn_threshold,
        }
    }

    /// # Errors
    ///
    /// This function will return an error if the selected output format cannot be rendered
    /// or if the rendered output cannot be written to stdout.
    pub fn emit(self, requested_format: Option<OutputFormat>) -> eyre::Result<()> {
        let Some(output) = self.value else {
            return Ok(());
        };

        let stdout_is_terminal = io::stdout().is_terminal();
        let format = requested_format.unwrap_or(if stdout_is_terminal {
            OutputFormat::Text
        } else {
            OutputFormat::Json
        });
        let Some(rendered) = output.render(format, stdout_is_terminal)? else {
            return Ok(());
        };

        let mut stdout = io::stdout().lock();
        stdout
            .write_all(rendered.as_bytes())
            .wrap_err("failed to write command output")?;
        if !rendered.ends_with('\n') {
            stdout
                .write_all(b"\n")
                .wrap_err("failed to terminate command output")?;
        }
        stdout.flush().wrap_err("failed to flush command output")?;
        Ok(())
    }
}

impl<T> CliOutputValue for FacetCliOutput<T>
where
    T: Facet<'static> + 'static,
{
    fn render(
        &self,
        format: OutputFormat,
        stdout_is_terminal: bool,
    ) -> eyre::Result<Option<String>> {
        let rendered = match format {
            OutputFormat::Text => PrettyPrinter::new()
                .with_colors(if stdout_is_terminal {
                    ColorMode::Always
                } else {
                    ColorMode::Never
                })
                .format(&self.value),
            OutputFormat::Json => facet_json::to_string_pretty(&self.value)
                .wrap_err("failed to serialize command output as JSON")?,
            OutputFormat::Csv => facet_csv::to_string(&self.value)
                .wrap_err("failed to serialize command output as CSV")?,
        };
        Ok(Some(rendered))
    }
}

struct FacetRowsCliOutput<T, R> {
    value: T,
    rows: fn(&T) -> Vec<R>,
}

impl CliOutput {
    /// Preserve structured reports in text/JSON, and emit flat rows for CSV.
    #[must_use]
    pub fn facet_rows<T, R>(
        value: T,
        rows: fn(&T) -> Vec<R>,
        elapsed_duration_warn_threshold: Option<std::time::Duration>,
    ) -> Self
    where
        T: Facet<'static> + 'static,
        R: Facet<'static> + 'static,
    {
        Self {
            value: Some(Box::new(FacetRowsCliOutput { value, rows })),
            elapsed_duration_warn_threshold,
        }
    }
}

impl<T, R> CliOutputValue for FacetRowsCliOutput<T, R>
where
    T: Facet<'static> + 'static,
    R: Facet<'static> + 'static,
{
    fn render(
        &self,
        format: OutputFormat,
        stdout_is_terminal: bool,
    ) -> eyre::Result<Option<String>> {
        let rendered = match format {
            OutputFormat::Text => PrettyPrinter::new()
                .with_colors(if stdout_is_terminal {
                    ColorMode::Always
                } else {
                    ColorMode::Never
                })
                .format(&self.value),
            OutputFormat::Json => facet_json::to_string_pretty(&self.value)
                .wrap_err("failed to serialize report as JSON")?,
            OutputFormat::Csv => {
                let fields = crate::cli::facet_shape::shape_struct_fields(R::SHAPE)
                    .ok_or_else(|| eyre::eyre!("CSV rows must be flat structs"))?;
                let mut csv = fields
                    .iter()
                    .map(|field| format!("\"{}\"", field.effective_name().replace('"', "\"\"")))
                    .collect::<Vec<_>>()
                    .join(",");
                csv.push('\n');
                for row in (self.rows)(&self.value) {
                    csv.push_str(
                        &facet_csv::to_string(&row)
                            .wrap_err("failed to serialize report row as CSV")?,
                    );
                }
                csv
            }
        };
        Ok(Some(rendered))
    }
}

#[cfg(test)]
mod budget_tests {
    use super::*;
    #[derive(Facet)]
    struct Report {
        count: u64,
    }
    #[test]
    fn budget_is_receipt_metadata_only() {
        for budget in [None, Some(std::time::Duration::from_secs(7))] {
            let output = CliOutput::facet(Report { count: 2 }, budget);
            assert_eq!(output.elapsed_duration_warn_threshold, budget);
            let value = output.value.unwrap();
            let json = value.render(OutputFormat::Json, false).unwrap().unwrap();
            let decoded: Report = facet_json::from_str(&json).unwrap();
            assert_eq!(decoded.count, 2);
            assert!(!json.contains("elapsed_duration"));
            let csv = value.render(OutputFormat::Csv, false).unwrap().unwrap();
            assert!(!csv.contains("elapsed_duration"));
        }
        assert!(CliOutput::none(None).value.is_none());
    }
}

#[cfg(test)]
mod rows_budget_tests {
    use super::*;
    #[derive(Facet, Clone)]
    struct Row {
        count: u64,
    }
    #[derive(Facet)]
    struct Report {
        rows: Vec<Row>,
        label: String,
    }
    #[test]
    fn rows_keep_structured_json_and_flat_csv_with_budget_metadata_separate() {
        let receipt = CliOutput::facet_rows(
            Report {
                rows: vec![Row { count: 2 }],
                label: "synthetic".to_owned(),
            },
            |report| report.rows.clone(),
            Some(std::time::Duration::from_secs(5)),
        );
        assert_eq!(
            receipt.elapsed_duration_warn_threshold,
            Some(std::time::Duration::from_secs(5))
        );
        let value = receipt.value.unwrap();
        let json = value.render(OutputFormat::Json, false).unwrap().unwrap();
        assert!(json.contains("synthetic"));
        assert!(!json.contains("elapsed_duration"));
        let csv = value.render(OutputFormat::Csv, false).unwrap().unwrap();
        assert!(csv.starts_with("\"count\"\n"));
        assert!(csv.contains('2'));
        assert!(!csv.contains("synthetic"));
        assert!(!csv.contains("elapsed_duration"));
    }
}
