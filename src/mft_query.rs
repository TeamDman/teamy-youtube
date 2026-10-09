use std::ops::ControlFlow;
use teamy_mft::cancellation::CancellationToken;
use teamy_mft::cli::command::query::QueryArgs;
use teamy_mft::query::QueryResultRow;

/// Collect a bounded visitor query using the current MFT consumer API.
/// The default `QueryArgs` backend reads published indexes without daemon operations.
pub(crate) fn collect_rows(args: &QueryArgs) -> eyre::Result<Vec<QueryResultRow>> {
    let mut rows = Vec::new();
    args.visit_rows(&CancellationToken::new(), |row| {
        rows.push(row);
        Ok(ControlFlow::Continue(()))
    })?;
    Ok(rows)
}
