use eyre::Result;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};

#[derive(Debug, Clone)]
pub struct Dashboard {
    pub pending_approval: Vec<Update>,
    pub open: Vec<Update>,
    pub awaiting_schedule: Vec<Update>,
    pub rate_limited: Vec<Update>,
    pub errored: Vec<Update>,
    pub pending_automerge: Vec<Update>,
    pub other: Vec<Update>,
}

#[derive(Debug, Clone)]
pub struct Update {
    pub branch: String,
    pub description: String,
    #[allow(dead_code)]
    pub checked: bool,
}

pub fn parse_dashboard(body: &str) -> Result<Dashboard> {
    let mut dashboard = Dashboard {
        pending_approval: Vec::new(),
        open: Vec::new(),
        awaiting_schedule: Vec::new(),
        rate_limited: Vec::new(),
        errored: Vec::new(),
        pending_automerge: Vec::new(),
        other: Vec::new(),
    };

    let options = Options::empty();
    let parser = Parser::new_ext(body, options);
    let events: Vec<Event> = parser.collect();

    let mut current_section = String::new();
    let mut i = 0;

    while i < events.len() {
        match &events[i] {
            Event::Start(Tag::Heading { level: HeadingLevel::H2, .. }) => {
                i += 1;
                if let Some(Event::Text(text)) = events.get(i) {
                    current_section = text.to_string();
                }
            }
            Event::Start(Tag::Item) => {
                i += 1;
                let mut branch = String::new();
                let mut description_parts: Vec<String> = Vec::new();
                let mut checked = false;
                let mut seen_bracket_open = false;
                let mut seen_bracket_close = false;
                let mut past_checkbox = false;

                while i < events.len() {
                    match &events[i] {
                        Event::End(TagEnd::Item) => break,
                        Event::InlineHtml(html) => {
                            if let Some(b) = extract_branch_from_comment(html) {
                                branch = b;
                            }
                            past_checkbox = true;
                        }
                        Event::Text(text) => {
                            let t = text.trim();
                            if !past_checkbox {
                                // Still in checkbox area
                                if t == "[" {
                                    seen_bracket_open = true;
                                    seen_bracket_close = false;
                                } else if t == "]" && seen_bracket_open {
                                    seen_bracket_close = true;
                                } else if seen_bracket_close && (t == "x" || t == "X") {
                                    checked = true;
                                    seen_bracket_open = false;
                                    seen_bracket_close = false;
                                } else if t == "-" || t.is_empty() || t == " " {
                                    // Ignore dashes and whitespace in checkbox
                                } else if seen_bracket_close && t != "x" && t != "X" {
                                    // After ] without x/X, checkbox is unchecked
                                    seen_bracket_open = false;
                                    seen_bracket_close = false;
                                }
                            } else {
                                // After HTML comment, this is description
                                if !t.is_empty() {
                                    description_parts.push(t.to_string());
                                }
                            }
                        }
                        Event::Code(code) => {
                            if past_checkbox {
                                description_parts.push(format!("`{}`", code));
                            }
                        }
                        Event::Start(Tag::Emphasis) | Event::Start(Tag::Strong) => {}
                        Event::End(TagEnd::Emphasis) | Event::End(TagEnd::Strong) => {}
                        Event::SoftBreak | Event::HardBreak => {}
                        _ => {}
                    }
                    i += 1;
                }

                let description = description_parts.join("");
                if !description.is_empty() || !branch.is_empty() {
                    let update = Update {
                        branch,
                        description,
                        checked,
                    };

                    match current_section.as_str() {
                        "Pending Approval" => dashboard.pending_approval.push(update),
                        "Open" => dashboard.open.push(update),
                        "Awaiting Schedule" => dashboard.awaiting_schedule.push(update),
                        "Rate-Limited" => dashboard.rate_limited.push(update),
                        "Errored" => dashboard.errored.push(update),
                        "Pending Branch Automerge" => dashboard.pending_automerge.push(update),
                        "Other Branches" => dashboard.other.push(update),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        i += 1;
    }

    Ok(dashboard)
}

fn extract_branch_from_comment(text: &str) -> Option<String> {
    let text = text.trim();
    if text.starts_with("<!-- ") && text.ends_with(" -->") {
        let inner = &text[5..text.len() - 4];
        if let Some(branch) = inner.strip_prefix("approve-branch=") {
            return Some(branch.to_string());
        }
        if let Some(branch) = inner.strip_prefix("approvePr-branch=") {
            return Some(branch.to_string());
        }
        if let Some(branch) = inner.strip_prefix("retry-branch=") {
            return Some(branch.to_string());
        }
        if let Some(branch) = inner.strip_prefix("unlimit-branch=") {
            return Some(branch.to_string());
        }
        if let Some(branch) = inner.strip_prefix("unschedule-branch=") {
            return Some(branch.to_string());
        }
        if let Some(branch) = inner.strip_prefix("other-branch=") {
            return Some(branch.to_string());
        }
        if let Some(branch) = inner.strip_prefix("rebase-branch=") {
            return Some(branch.to_string());
        }
    }
    None
}
