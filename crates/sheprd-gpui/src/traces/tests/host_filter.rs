use super::*;
use sheprd_client::device_tags::{ComputerTags, TaggedHost};
use sheprd_control_protocol::protocol::app::TraceFilter;

fn hosts() -> Vec<TaggedHost> {
    vec![
        TaggedHost {
            id: "home".into(),
            label: "same name".into(),
            tags: ComputerTags::new(["personal"], 1),
        },
        TaggedHost {
            id: "work".into(),
            label: "same name".into(),
            tags: ComputerTags::new(["Work"], 1),
        },
        TaggedHost {
            id: "both".into(),
            label: "studio".into(),
            tags: ComputerTags::new(["work", "personal"], 1),
        },
    ]
}

#[test]
fn host_ids_disambiguate_names_and_tags_match_offline_hosts_case_insensitively() {
    let hosts = hosts();
    let sessions: Vec<_> = hosts
        .iter()
        .map(|host| {
            let mut value = session(&host.id, AgentKind::Claude, 1, 5);
            value.host_id = host.id.clone();
            value.host = host.label.clone();
            value
        })
        .collect();
    let mut filter = Filter {
        scope: TraceFilter::Host { id: "home".into() },
        ..Filter::default()
    };
    let ids = |filter: &Filter| {
        model::shown(&sessions, filter, NOW, &hosts)
            .iter()
            .map(|s| s.id.as_str())
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&filter), ["home"]);
    filter.scope = TraceFilter::Tag { tag: "WORK".into() };
    assert_eq!(ids(&filter), ["work", "both"]);
    assert_eq!(
        model::totals(&model::shown(&sessions, &filter, NOW, &hosts)).hosts,
        2
    );
    filter.query = "studio".into();
    assert_eq!(ids(&filter), ["both"]);
    filter.scope = TraceFilter::All;
    filter.query.clear();
    assert_eq!(ids(&filter), ["home", "work", "both"]);
    assert_eq!(
        crate::traces::host_filter::tags(&hosts),
        ["personal", "Work"]
    );
}

#[test]
fn an_unknown_or_removed_tag_never_falls_back_to_every_host() {
    let filter = TraceFilter::Tag {
        tag: "removed".into(),
    };
    assert!(!filter.includes("home", &hosts()));
    let filter = TraceFilter::Tag {
        tag: "personal".into(),
    };
    let mut hosts = hosts();
    hosts[0].tags = ComputerTags::new(Vec::<String>::new(), 2);
    assert!(!filter.includes("home", &hosts));
    assert!(filter.includes("both", &hosts));
}
