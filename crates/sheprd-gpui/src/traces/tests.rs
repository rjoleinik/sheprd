#![allow(clippy::unwrap_used)]

use super::model::{self, Filter, Range};
use chrono::NaiveDate;
use sheprd_client::herdr::AgentKind;
use sheprd_traces::{Activity, Session, Visibility};

mod activity_refresh;
mod historical_agents;
mod host_filter;
mod layout;

const DAY: i64 = 24 * 60 * 60 * 1000;
/// 2026-11-17T12:00:00Z.
const NOW: i64 = 1_794_916_800_000;

fn session(id: &str, agent: AgentKind, days_ago: i64, messages: i64) -> Session {
    Session {
        id: id.into(),
        host_id: format!("host-{}", id.len() % 2),
        host: "box".into(),
        agent,
        native_id: id.into(),
        user: "me@box".into(),
        title: Some(format!("Fix {id}")),
        model: Some("claude-opus-5-5".into()),
        cwd: Some("/home/me/code/app".into()),
        branch: Some("main".into()),
        agent_version: None,
        started_at: Some(NOW - days_ago * DAY),
        updated_at: Some(NOW - days_ago * DAY),
        messages,
        user_messages: 1,
        tool_calls: 2,
        errors: 1,
        tokens_in: 100,
        tokens_out: 10,
        visibility: Visibility::Private,
    }
}

fn today() -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, 17).unwrap()
}

#[test]
fn the_filter_keeps_the_range_agent_and_query() {
    let sessions = [
        session("login", AgentKind::Claude, 1, 5),
        session("router", AgentKind::Codex, 3, 7),
        session("old", AgentKind::Claude, 40, 2),
    ];
    let ids = |filter: &Filter| -> Vec<String> {
        model::shown(&sessions, filter, NOW, &[])
            .iter()
            .map(|session| session.id.clone())
            .collect()
    };
    assert_eq!(ids(&Filter::default()), ["login", "router"]);
    let year = Filter {
        range: Range::Year,
        ..Filter::default()
    };
    assert_eq!(ids(&year), ["login", "router", "old"]);
    let codex = Filter {
        agent: Some(AgentKind::Codex),
        ..Filter::default()
    };
    assert_eq!(ids(&codex), ["router"]);
    let query = Filter {
        query: " ROUTER ".into(),
        ..Filter::default()
    };
    assert_eq!(ids(&query), ["router"]);
    // The query also matches the agent's name.
    let by_agent = Filter {
        query: "codex".into(),
        ..Filter::default()
    };
    assert_eq!(ids(&by_agent), ["router"]);
}

#[test]
fn totals_add_up_the_sessions_shown() {
    let sessions = [
        session("ab", AgentKind::Claude, 1, 5),
        session("abc", AgentKind::Codex, 1, 7),
    ];
    let shown: Vec<&Session> = sessions.iter().collect();
    let totals = model::totals(&shown);
    assert_eq!(
        (
            totals.sessions,
            totals.messages,
            totals.tool_calls,
            totals.errors
        ),
        (2, 12, 4, 2)
    );
    assert_eq!((totals.tokens, totals.hosts), (220, 2));
}

#[test]
fn agents_are_ordered_by_volume() {
    let sessions = [
        session("a", AgentKind::Claude, 1, 5),
        session("b", AgentKind::Pi, 1, 50),
        session("c", AgentKind::Claude, 1, 10),
    ];
    assert_eq!(
        model::agents(&sessions, &[]),
        [AgentKind::Pi, AgentKind::Claude]
    );
}

#[test]
fn the_chart_stacks_each_day_by_agent_and_ends_today() {
    let activity = [
        Activity {
            day: "2026-11-17".into(),
            agent: AgentKind::Claude,
            sessions: 1,
            messages: 4,
        },
        Activity {
            day: "2026-11-17".into(),
            agent: AgentKind::Codex,
            sessions: 1,
            messages: 6,
        },
        Activity {
            day: "2026-11-11".into(),
            agent: AgentKind::Claude,
            sessions: 1,
            messages: 3,
        },
        // Before the week: left out.
        Activity {
            day: "2026-11-01".into(),
            agent: AgentKind::Claude,
            sessions: 1,
            messages: 99,
        },
    ];
    let agents = [AgentKind::Codex, AgentKind::Claude];
    let week = Filter {
        range: Range::Week,
        ..Filter::default()
    };
    let chart = model::chart(&activity, &agents, &week, today());
    assert_eq!(chart.days.len(), 7);
    assert_eq!(
        chart.days.first(),
        NaiveDate::from_ymd_opt(2026, 11, 11).as_ref()
    );
    assert_eq!(chart.days.last(), Some(&today()));
    assert_eq!(chart.counts[6], [6, 4]);
    assert_eq!(chart.counts[0], [0, 3]);
    assert_eq!((chart.peak, chart.total), (10, 13));

    // One agent: only its column.
    let claude = Filter {
        agent: Some(AgentKind::Claude),
        ..week
    };
    let chart = model::chart(&activity, &agents, &claude, today());
    assert_eq!(chart.agents, [AgentKind::Claude]);
    assert_eq!(chart.counts[6], [4]);
}

#[test]
fn counts_are_compact() {
    assert_eq!(model::compact(999), "999");
    assert_eq!(model::compact(12_345), "12.3k");
    assert_eq!(model::compact(4_500_000), "4.5M");
}
