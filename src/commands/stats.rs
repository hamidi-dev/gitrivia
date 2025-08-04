use anyhow::Result;
use clap::Args;
use serde_json::json;

use crate::{
    commands::Global,
    domain::{git::RepoExt, stats as d},
    presentation::table,
    utils::fmt_date,
};

/// Summarise repository health and highlight top contributors.
///
/// The command scans commit history to produce high‑level metrics such as
/// active period, momentum and merge/revert rates. It also prints the top
/// five authors overall and for the last 12 months.
#[derive(Debug, Args)]
pub struct Stats {
    /// Path to the Git repository to inspect.
    /// Defaults to the current working directory.
    #[arg(short, long, default_value = ".")]
    pub path: String,

    /// Only analyse the most recent `N` commits.
    /// If omitted, the entire history is scanned.
    #[arg(short, long)]
    pub limit: Option<usize>,

    /// Force descending sort order for contributor tables.
    /// The command defaults to descending, but this flag overrides any
    /// global `--desc` setting.
    #[arg(long)]
    pub sort_desc: bool,
}

impl super::Runnable for Stats {
    fn run(self, g: &Global) -> Result<()> {
        let repo = RepoExt::open(&self.path)?;
        let scan = d::scan_repo(repo.repo(), self.limit);

        if g.json {
            // Build top-5 authors sorted desc by count
            let mut top_vec = scan
                .stats
                .data
                .iter()
                .map(|(email, m)| (email.clone(), m.count, m.first, m.last))
                .collect::<Vec<_>>();
            top_vec.sort_by(|a, b| b.1.cmp(&a.1));
            let top_vec = top_vec.into_iter().take(5).map(|(email, count, first, last)| {
                json!({"email": email, "count": count, "first": fmt_date(first), "last": fmt_date(last)})
            }).collect::<Vec<_>>();

            let mut top12 = scan
                .recent12
                .data
                .iter()
                .map(|(email, m)| (email.clone(), m.count, m.first, m.last))
                .collect::<Vec<_>>();
            top12.sort_by(|a, b| b.1.cmp(&a.1));
            let top12 = top12.into_iter().take(5).map(|(email, count, first, last)| {
                json!({"email": email, "count": count, "first": fmt_date(first), "last": fmt_date(last)})
            }).collect::<Vec<_>>();

            let s = &scan.summary;
            let payload = json!({
                "summary": {
                    "first_commit": { "date": fmt_date(s.first_date), "author": s.first_author },
                    "last_commit":  { "date": fmt_date(s.last_date),  "author": s.last_author  },
                    "total_commits": s.total_commits,
                    "contributors_total": s.contributors_total,
                    "active_days": s.active_days,
                    "avg_commits_per_day": s.avg_commits_per_day,
                    "peak_day": s.peak_day.as_ref().map(|(d,c)| json!({"date": d.to_string(), "commits": c})),
                    "longest_idle_gap_days": s.longest_idle_gap_days,
                    "momentum_90d_pct": s.momentum_90d_pct,
                    "active_authors_last_90d": s.active_authors_last_90d,

                    "contributors": {
                        "drive_by_ratio_pct": s.drive_by_ratio,
                        "core_size_80pct": s.core_size_80pct,
                        "concentration_hhi": s.hhi,
                        "concentration_gini": s.gini
                    },

                    "activity_patterns": {
                        "weekday_counts_mon_sun": s.weekday_counts,
                        "work_hours_pct_9_18": s.work_hours_pct
                    },

                    "merge_revert": {
                        "merge_rate_pct": s.merge_rate,
                        "revert_rate_pct": s.revert_rate
                    },

                    "messages": {
                        "median_subject_len": s.msg_median_len,
                        "body_present_pct": s.msg_body_pct,
                        "conventional_commit_pct": s.conv_commit_pct
                    },

                    "top_recent_30d": s.top_recent_30d.as_ref()
                        .map(|(a,c)| json!({"author": a, "commits": c}))
                },
                    "top_5_authors_last_12m": top12,
                    "top_5_authors": top_vec
            });
            println!("{}", serde_json::to_string_pretty(&payload)?);
            return Ok(());
        }

        // Human-friendly with quick explanations
        let s = &scan.summary;

        println!("✨ Repo summary");
        println!(
            "  First commit:     {} by {}",
            fmt_date(s.first_date),
            s.first_author
        );
        println!(
            "  Last commit:      {} by {}",
            fmt_date(s.last_date),
            s.last_author
        );
        println!("  Total commits:    {}", s.total_commits);
        println!("  Contributors:     {}", s.contributors_total);
        println!("  Active period:    {} days", s.active_days);
        println!("  Avg commits/day:  {:.2}", s.avg_commits_per_day);
        if let Some((d, c)) = s.peak_day {
            println!("  Peak day:         {} ({} commits)", d, c);
        }
        println!(
            "  Longest idle gap: {} days (largest pause between commits)",
            s.longest_idle_gap_days
        );
        println!(
            "  Momentum (90d):   {:.1}% of all commits, {} authors active",
            s.momentum_90d_pct, s.active_authors_last_90d
        );
        if let Some((a, c)) = &s.top_recent_30d {
            println!("  Top last 30d:     {} ({} commits)", a, c);
        }

        println!();
        println!("👥 Contributors");
        println!("  Drive-by ratio:   {:.0}%  (share of authors with ≤2 commits; many = lots of one-offs)", s.drive_by_ratio);
        println!(
            "  Core size (80%):  {}     (few = concentrated, many = distributed)",
            s.core_size_80pct
        );
        println!(
            "  Concentration:    HHI {:.2}  |  Gini {:.2}  (higher = more concentrated)",
            s.hhi, s.gini
        );

        println!();
        let wc = s.weekday_counts;
        let wc_total = wc.iter().sum::<usize>().max(1) as f64;
        let pct = |n: usize| 100.0 * (n as f64) / wc_total;
        println!("⏰ Activity patterns");
        println!("  Weekdays: Mon {:>4.1}% Tue {:>4.1}% Wed {:>4.1}% Thu {:>4.1}% Fri {:>4.1}% Sat {:>4.1}% Sun {:>4.1}%",
            pct(wc[0]), pct(wc[1]), pct(wc[2]), pct(wc[3]), pct(wc[4]), pct(wc[5]), pct(wc[6]));
        println!("  Work-hours (09–18): {:.0}%", s.work_hours_pct);

        println!();
        println!("🔀 Merge/Revert");
        println!(
            "  Merge rate:  {:.0}%   Revert rate: {:.1}%",
            s.merge_rate, s.revert_rate
        );

        println!();
        println!("📝 Messages");
        println!("  Median subject length: {} chars", s.msg_median_len);
        println!("  With body:             {:.0}%", s.msg_body_pct);
        println!("  Conventional commits:  {:.0}%", s.conv_commit_pct);

        println!();
        println!("🔥 Top 5 authors (last 12 months):");
        if scan.recent12.data.is_empty() {
            println!("(no commits in the last 12 months)");
        } else {
            println!("{}", table::author_stats_top(&scan.recent12, true, 5));
        }

        println!();
        println!("🔥 Top 5 authors (all time):");
        // Force DESC for “Top 5”
        println!("{}", table::author_stats_top(&scan.stats, true, 5));

        // Tiny legend
        println!("\nLegend:");
        println!(
            "  Drive-by ratio = Authors with ≤2 commits (higher → many one-off contributors)."
        );
        println!("  Core size (80%) = Minimal number of authors covering 80% of commits.");
        println!("  HHI/Gini = Contribution concentration (higher → more concentrated).");

        Ok(())
    }
}
