//! Deterministic terminal-table rendering for milestone 0.1 reports.

use std::fmt::Write as _;

use clap::builder::styling::{AnsiColor, Style as AnsiStyle};

use urollup_core::query::{
    DailyDocument, ReportDocument, RequestCounts, SessionsDocument, TokenCounts,
};

const STYLE_HEADING: AnsiStyle = AnsiColor::Cyan.on_default().bold();
const STYLE_COMPLETE: AnsiStyle = AnsiColor::Green.on_default().bold();
const STYLE_PARTIAL: AnsiStyle = AnsiColor::Yellow.on_default().bold();

pub(crate) fn report(document: &ReportDocument, color: bool) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "{}", paint("urollup report", STYLE_HEADING, color));
    write_query(&mut output, &document.query);
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", paint("TOTALS", STYLE_HEADING, color));
    let _ = writeln!(output, "Requests  {}", number(request_total(document.totals.requests)));
    let _ = writeln!(output, "Owned     {}", number(document.totals.requests.owned));
    let _ = writeln!(output, "Ambiguous {}", number(document.totals.requests.ambiguous));
    let _ = writeln!(output, "Unknown   {}", number(document.totals.requests.unknown));
    write_tokens(&mut output, &document.totals.tokens);
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", paint("COVERAGE", STYLE_HEADING, color));
    let coverage = if document.coverage.complete {
        paint("complete", STYLE_COMPLETE, color)
    } else {
        paint("partial", STYLE_PARTIAL, color)
    };
    let _ = writeln!(
        output,
        "Status {}  Copies excluded {}  Limit observations {}",
        coverage,
        number(document.coverage.copies_excluded),
        number(document.coverage.limit_observations),
    );
    let _ = writeln!(
        output,
        "Unresolved {}  Possible {}  Requests without usage {}",
        number(document.totals.unresolved.requests),
        number(document.totals.possible.requests),
        number(document.coverage.requests_without_usage),
    );
    let _ = writeln!(output);
    let _ = writeln!(
        output,
        "{}",
        paint("REQUEST SIZES (inclusive input tokens)", STYLE_HEADING, color)
    );
    let _ = writeln!(
        output,
        "Count {}  p50 {}  p90 {}  p99 {}  max {}",
        number(document.sizes.count),
        optional_number(document.sizes.p50),
        optional_number(document.sizes.p90),
        optional_number(document.sizes.p99),
        optional_number(document.sizes.max),
    );
    for (group, rows) in &document.breakdowns {
        let _ = writeln!(output);
        let heading = format!("{} BREAKDOWN", group.to_uppercase());
        let _ = writeln!(output, "{}", paint(&heading, STYLE_HEADING, color));
        let _ = writeln!(output, "VALUE | REQUESTS | INPUT | OUTPUT | TOTAL");
        for row in rows {
            let _ = writeln!(
                output,
                "{} | {} | {} | {} | {}",
                row.value,
                number(request_total(row.requests)),
                input(&row.tokens),
                optional_number(row.tokens.output),
                optional_number(row.tokens.total),
            );
        }
    }
    write_diagnostics(&mut output, &document.diagnostics, color);
    output
}

pub(crate) fn daily(document: &DailyDocument, color: bool) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "{}", paint("urollup daily", STYLE_HEADING, color));
    write_query(&mut output, &document.query);
    let _ = writeln!(output);
    let _ =
        writeln!(output, "DATE | REQUESTS | UNCACHED | CACHE READ | CACHE WRITE | OUTPUT | TOTAL");
    for row in &document.rows {
        let _ = writeln!(
            output,
            "{} | {} | {} | {} | {} | {} | {}",
            row.date.as_deref().unwrap_or("unknown"),
            number(request_total(row.requests)),
            optional_number(row.tokens.uncached_input),
            optional_number(row.tokens.cache_read),
            optional_number(row.tokens.cache_write),
            optional_number(row.tokens.output),
            optional_number(row.tokens.total),
        );
    }
    write_diagnostics(&mut output, &document.diagnostics, color);
    output
}

pub(crate) fn sessions(document: &SessionsDocument, color: bool) -> String {
    let mut output = String::new();
    let _ = writeln!(output, "{}", paint("urollup sessions", STYLE_HEADING, color));
    write_query(&mut output, &document.query);
    let _ = writeln!(output);
    let _ = writeln!(output, "THREAD | AGENT | PROJECT | REQUESTS | INPUT | OUTPUT | TOTAL");
    for row in &document.rows {
        let _ = writeln!(
            output,
            "{} | {} | {} | {} | {} | {} | {}",
            row.thread.as_deref().unwrap_or("unowned"),
            row.agent,
            row.project.as_deref().unwrap_or("unknown"),
            number(request_total(row.requests)),
            input(&row.tokens),
            optional_number(row.tokens.output),
            optional_number(row.tokens.total),
        );
    }
    write_diagnostics(&mut output, &document.diagnostics, color);
    output
}

fn write_query(output: &mut String, query: &urollup_core::query::QueryMetadata) {
    let _ = writeln!(
        output,
        "Selection {}  Scope {}  Timezone {}",
        query.selection, query.scope, query.timezone
    );
}

fn write_tokens(output: &mut String, tokens: &TokenCounts) {
    let _ = writeln!(output, "Uncached input {}", optional_number(tokens.uncached_input));
    let _ = writeln!(output, "Cache read     {}", optional_number(tokens.cache_read));
    let _ = writeln!(output, "Cache write    {}", optional_number(tokens.cache_write));
    let _ = writeln!(output, "Output         {}", optional_number(tokens.output));
    let _ = writeln!(output, "Reasoning      {}", optional_number(tokens.reasoning));
    let _ = writeln!(output, "Total tokens   {}", optional_number(tokens.total));
}

fn write_diagnostics(
    output: &mut String,
    diagnostics: &[urollup_core::query::DiagnosticSummary],
    color: bool,
) {
    if diagnostics.is_empty() {
        return;
    }
    let _ = writeln!(output);
    let _ = writeln!(output, "{}", paint("DIAGNOSTICS", STYLE_HEADING, color));
    for diagnostic in diagnostics {
        let _ =
            writeln!(output, "{} x{}: {}", diagnostic.code, diagnostic.count, diagnostic.detail);
    }
}

fn request_total(counts: RequestCounts) -> u64 {
    counts.owned.saturating_add(counts.ambiguous).saturating_add(counts.unknown)
}

fn input(tokens: &TokenCounts) -> String {
    let values = [
        tokens.uncached_input,
        tokens.cache_read,
        tokens.cache_write_5m,
        tokens.cache_write_1h,
        tokens.cache_write_unspecified,
    ];
    let any = values.iter().any(Option::is_some);
    if !any {
        return "-".to_owned();
    }
    number(values.into_iter().flatten().fold(0_u64, u64::saturating_add))
}

fn optional_number(value: Option<u64>) -> String {
    value.map_or_else(|| "-".to_owned(), number)
}

fn number(value: u64) -> String {
    let digits = value.to_string();
    let mut output = String::with_capacity(digits.len().saturating_add(digits.len() / 3));
    let first = digits.len() % 3;
    for (index, byte) in digits.bytes().enumerate() {
        if index > 0 && index % 3 == first {
            output.push(',');
        }
        output.push(char::from(byte));
    }
    output
}

fn paint(text: &str, style: AnsiStyle, color: bool) -> String {
    if color { format!("{style}{text}{style:#}") } else { text.to_owned() }
}

#[cfg(test)]
mod tests {
    use super::number;

    #[test]
    fn formats_counts_without_locale_state() {
        assert_eq!(number(0), "0");
        assert_eq!(number(999), "999");
        assert_eq!(number(1_000), "1,000");
        assert_eq!(number(12_345_678), "12,345,678");
    }
}
