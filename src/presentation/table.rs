use crate::domain::stats::{AuthorMeta, CommitStats};
use crate::utils::fmt_date;
use comfy_table::{Table, presets::UTF8_HORIZONTAL_ONLY};

fn sorted_entries<'a>(stats: &'a CommitStats, desc: bool)
    -> Vec<(&'a String, &'a AuthorMeta)>
{
    let mut entries: Vec<_> = stats.data.iter().collect();
    entries.sort_by(|a, b| {
        if desc { b.1.count.cmp(&a.1.count) } else { a.1.count.cmp(&b.1.count) }
    });
    entries
}

/// Only top N authors.
pub fn author_stats_top(stats: &CommitStats, desc: bool, n: usize) -> String {
    let entries = sorted_entries(stats, desc);
    render(entries_to_rows(entries.into_iter().take(n).collect()))
}

fn entries_to_rows(entries: Vec<(&String, &AuthorMeta)>) -> Vec<[String; 4]> {
    entries.into_iter().map(|(email, m)| {
        [
            email.clone(),
            m.count.to_string(),
            fmt_date(m.first),
            fmt_date(m.last),
        ]
    }).collect()
}

fn render(rows: Vec<[String; 4]>) -> String {
    let mut t = Table::new();
    t.load_preset(UTF8_HORIZONTAL_ONLY)
        .set_header(vec!["Author", "Commits", "First", "Last"]);
    for r in rows {
        t.add_row(vec![r[0].clone(), r[1].clone(), r[2].clone(), r[3].clone()]);
    }
    t.to_string()
}

