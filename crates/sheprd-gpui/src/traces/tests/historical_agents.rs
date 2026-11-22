use super::*;

#[test]
fn a_full_recent_table_does_not_hide_an_older_agents_activity_or_filter() {
    let recent: Vec<_> = (0..sheprd_traces::MAX_LIMIT)
        .map(|i| session(&format!("recent-{i}"), AgentKind::Claude, 0, 1))
        .collect();
    let activity = vec![
        Activity {
            day: "2026-01-01".into(),
            agent: AgentKind::Codex,
            sessions: 1,
            messages: 4,
        },
        Activity {
            day: "2026-11-17".into(),
            agent: AgentKind::Claude,
            sessions: 1,
            messages: 2,
        },
    ];
    let agents = model::agents(&recent, &activity);
    assert!(agents.contains(&AgentKind::Claude));
    assert!(agents.contains(&AgentKind::Codex));
    let filter = Filter {
        range: Range::Year,
        ..Filter::default()
    };
    assert_eq!(model::chart(&activity, &agents, &filter, today()).total, 6);
    let filter = Filter {
        agent: Some(AgentKind::Codex),
        ..filter
    };
    assert!(model::shown(&recent, &filter, NOW, &[]).is_empty());
    let chart = model::chart(&activity, &agents, &filter, today());
    assert_eq!(chart.agents, [AgentKind::Codex]);
    assert_eq!(chart.total, 4);
}
