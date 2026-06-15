use std::fs;
use eyre::{Result, WrapErr};
use chrono::Local;

use crate::client::PlatformClient;
use crate::parser::Dashboard;

fn count_updates(dashboard: &Dashboard) -> usize {
    dashboard.pending_approval.len()
        + dashboard.open.len()
        + dashboard.awaiting_schedule.len()
        + dashboard.rate_limited.len()
        + dashboard.errored.len()
        + dashboard.pending_automerge.len()
        + dashboard.other.len()
}

pub fn generate_report(
    dashboards: &[(String, String, String, Dashboard)],
    client: &PlatformClient,
    output_path: &str,
) -> Result<()> {
    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

    let total_pending: usize = dashboards.iter().map(|(_, _, _, d)| count_updates(d)).sum();
    let total_pending_approval: usize = dashboards.iter().map(|(_, _, _, d)| d.pending_approval.len()).sum();
    let total_open: usize = dashboards.iter().map(|(_, _, _, d)| d.open.len()).sum();
    let total_awaiting: usize = dashboards.iter().map(|(_, _, _, d)| d.awaiting_schedule.len()).sum();
    let total_rate_limited: usize = dashboards.iter().map(|(_, _, _, d)| d.rate_limited.len()).sum();
    let total_errored: usize = dashboards.iter().map(|(_, _, _, d)| d.errored.len()).sum();
    let total_automerge: usize = dashboards.iter().map(|(_, _, _, d)| d.pending_automerge.len()).sum();
    let total_other: usize = dashboards.iter().map(|(_, _, _, d)| d.other.len()).sum();

    let mut sorted: Vec<_> = dashboards.to_vec();
    sorted.sort_by(|a, b| count_updates(&b.3).cmp(&count_updates(&a.3)));

    let mut html = format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Renovate Dashboard Report</title>
    <style>
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; background: #f5f5f5; color: #333; padding: 2rem; }}
        .container {{ max-width: 1200px; margin: 0 auto; }}
        h1 {{ margin-bottom: 0.5rem; color: #1a1a1a; }}
        .timestamp {{ color: #666; margin-bottom: 2rem; }}
        .summary {{ display: grid; grid-template-columns: repeat(auto-fit, minmax(150px, 1fr)); gap: 1rem; margin-bottom: 2rem; }}
        .card {{ background: white; border-radius: 8px; padding: 1.5rem; box-shadow: 0 1px 3px rgba(0,0,0,0.1); text-align: center; }}
        .card h3 {{ font-size: 2rem; margin-bottom: 0.5rem; }}
        .card p {{ color: #666; font-size: 0.9rem; }}
        .card.total h3 {{ color: #1a1a1a; }}
        .card.pending h3 {{ color: #f59e0b; }}
        .card.open h3 {{ color: #10b981; }}
        .card.awaiting h3 {{ color: #3b82f6; }}
        .card.ratelimited h3 {{ color: #8b5cf6; }}
        .card.error h3 {{ color: #ef4444; }}
        .card.automerge h3 {{ color: #10b981; }}
        .card.other h3 {{ color: #6b7280; }}
        .repo {{ background: white; border-radius: 8px; padding: 1.5rem; margin-bottom: 1rem; box-shadow: 0 1px 3px rgba(0,0,0,0.1); }}
        .repo h2 {{ font-size: 1.25rem; margin-bottom: 1rem; color: #1a1a1a; border-bottom: 2px solid #e5e7eb; padding-bottom: 0.5rem; }}
        .repo a {{ color: #3b82f6; text-decoration: none; }}
        .repo a:hover {{ text-decoration: underline; }}
        .section {{ margin-bottom: 1rem; }}
        .section h3 {{ font-size: 1rem; margin-bottom: 0.5rem; color: #374151; }}
        .section.empty {{ color: #9ca3af; font-style: italic; }}
        table {{ width: 100%; border-collapse: collapse; margin-bottom: 1rem; }}
        th, td {{ padding: 0.75rem; text-align: left; border-bottom: 1px solid #e5e7eb; }}
        th {{ background: #f9fafb; font-weight: 600; font-size: 0.85rem; text-transform: uppercase; color: #6b7280; }}
        tr:hover {{ background: #f9fafb; }}
        td a {{ color: #3b82f6; text-decoration: none; }}
        td a:hover {{ text-decoration: underline; }}
        .badge {{ display: inline-block; padding: 0.25rem 0.5rem; border-radius: 4px; font-size: 0.75rem; font-weight: 500; }}
        .badge.pending {{ background: #fef3c7; color: #92400e; }}
        .badge.open {{ background: #d1fae5; color: #065f46; }}
        .badge.awaiting {{ background: #dbeafe; color: #1e40af; }}
        .badge.ratelimited {{ background: #ede9fe; color: #5b21b6; }}
        .badge.error {{ background: #fee2e2; color: #991b1b; }}
        .badge.automerge {{ background: #d1fae5; color: #065f46; }}
        .badge.other {{ background: #f3f4f6; color: #374151; }}
        .no-dashboards {{ text-align: center; padding: 3rem; color: #6b7280; }}
    </style>
</head>
<body>
    <div class="container">
        <h1>Renovate Dashboard Report</h1>
        <p class="timestamp">Generated: {timestamp}</p>

        <div class="summary">
            <div class="card total">
                <h3>{total_pending}</h3>
                <p>Total Updates</p>
            </div>
            <div class="card pending">
                <h3>{total_pending_approval}</h3>
                <p>Pending Approval</p>
            </div>
            <div class="card open">
                <h3>{total_open}</h3>
                <p>Open PRs</p>
            </div>
            <div class="card awaiting">
                <h3>{total_awaiting}</h3>
                <p>Awaiting Schedule</p>
            </div>
            <div class="card ratelimited">
                <h3>{total_rate_limited}</h3>
                <p>Rate-Limited</p>
            </div>
            <div class="card error">
                <h3>{total_errored}</h3>
                <p>Errored</p>
            </div>
            <div class="card automerge">
                <h3>{total_automerge}</h3>
                <p>Pending Automerge</p>
            </div>
            <div class="card other">
                <h3>{total_other}</h3>
                <p>Other</p>
            </div>
        </div>
"#);

    if sorted.is_empty() {
        html.push_str(r#"<div class="no-dashboards"><p>No Renovate dashboards found.</p></div>"#);
    } else {
        for (full_name, _repo_name, dashboard_url, dashboard) in &sorted {
            html.push_str(&format!(
                r#"<div class="repo">
    <h2><a href="{}">{}</a></h2>"#,
                dashboard_url, full_name
            ));

            render_section(&mut html, "Pending Approval", &dashboard.pending_approval, "pending", full_name, client);
            render_section(&mut html, "Open PRs", &dashboard.open, "open", full_name, client);
            render_section(&mut html, "Awaiting Schedule", &dashboard.awaiting_schedule, "awaiting", full_name, client);
            render_section(&mut html, "Rate-Limited", &dashboard.rate_limited, "ratelimited", full_name, client);
            render_section(&mut html, "Errored", &dashboard.errored, "error", full_name, client);
            render_section(&mut html, "Pending Branch Automerge", &dashboard.pending_automerge, "automerge", full_name, client);
            render_section(&mut html, "Other Branches", &dashboard.other, "other", full_name, client);

            html.push_str("</div>\n");
        }
    }

    html.push_str(r#"</div>
</body>
</html>"#);

    fs::write(output_path, &html)
        .wrap_err_with(|| format!("Failed to write report to {}", output_path))?;

    Ok(())
}

fn render_section(
    html: &mut String,
    title: &str,
    updates: &[crate::parser::Update],
    badge_class: &str,
    full_name: &str,
    client: &PlatformClient,
) {
    if updates.is_empty() {
        html.push_str(&format!(
            r#"<div class="section empty"><h3>{}: None</h3></div>"#,
            title
        ));
        return;
    }

    html.push_str(&format!(
        r#"<div class="section">
    <h3><span class="badge {}">{}</span> ({})</h3>
    <table>
        <thead><tr><th>Description</th><th>Branch</th></tr></thead>
        <tbody>"#,
        badge_class,
        title,
        updates.len()
    ));

    for update in updates {
        let branch_display = if update.branch.is_empty() {
            "-".to_string()
        } else {
            let pr_url = client.pr_search_url(full_name, &update.branch);
            format!("<a href=\"{}\"><code>{}</code></a>", pr_url, update.branch)
        };

        html.push_str(&format!(
            "<tr><td>{}</td><td>{}</td></tr>",
            update.description, branch_display
        ));
    }

    html.push_str(r#"</tbody></table></div>"#);
}
