//! Joined-space navigation projected from the SDK's space hierarchy filters.

use std::collections::{HashMap, HashSet};

use futures_util::{Stream, StreamExt};
use matrix_sdk_ui::spaces::{SpaceFilter, SpaceService};

use crate::auth::Account;

#[derive(Clone, Default)]
pub struct SpaceSummary {
    pub id: String,
    pub name: String,
    pub level: u8,
    pub avatar: Option<String>,
    pub can_add_rooms: bool,
    pub room_ids: HashSet<String>,
}

#[derive(Clone)]
pub struct SpaceSnapshot {
    pub account_instance: u128,
    pub spaces: Vec<SpaceSummary>,
}

// Expand the SDK-provided filter relationships for inclusive parent navigation.
// Identity is the room ID; cycles and shared descendants must not duplicate rooms.
fn descendants(root: &str, edges: &HashMap<String, Vec<String>>) -> HashSet<String> {
    let mut visited = HashSet::new();
    let mut pending = vec![root.to_owned()];
    while let Some(id) = pending.pop() {
        if !visited.insert(id.clone()) {
            continue;
        }
        if let Some(children) = edges.get(&id) {
            pending.extend(children.iter().cloned());
        }
    }
    visited.remove(root);
    visited
}

fn summaries(
    filters: impl Iterator<Item = SpaceFilter>,
    editable: &HashSet<String>,
    joined: &HashSet<String>,
    left_edges: &HashMap<String, Vec<String>>,
) -> Vec<SpaceSummary> {
    let filters: Vec<_> = filters.collect();
    let mut edges: HashMap<_, _> = filters
        .iter()
        .map(|filter| {
            (
                filter.space_room.room_id.to_string(),
                filter.descendants.iter().map(ToString::to_string).collect(),
            )
        })
        .collect();
    // Leaving a container doesn't remove its child links. The SDK's joined
    // graph omits those outgoing edges, but its cached state still knows them.
    for (id, children) in left_edges {
        if !joined.contains(id) {
            edges.insert(id.clone(), children.clone());
        }
    }
    let mut seen = HashSet::new();
    filters
        .into_iter()
        // Hierarchy links outlive membership. The SDK can include known left
        // child spaces, so use current client membership rather than cached
        // membership on a SpaceFilter when projecting navigation tiles.
        .filter(|filter| joined.contains(filter.space_room.room_id.as_str()))
        .filter(|filter| seen.insert(filter.space_room.room_id.clone()))
        .map(|filter| {
            let id = filter.space_room.room_id.to_string();
            SpaceSummary {
                room_ids: descendants(&id, &edges),
                can_add_rooms: editable.contains(&id),
                id,
                avatar: filter.space_room.avatar_url.map(|url| url.to_string()),
                name: filter.space_room.display_name,
                level: filter.level,
            }
        })
        .collect()
}

impl Account {
    /// Project only locally cached, reachable left-space links. This does not
    /// join spaces, fetch their state, or restore their navigation tiles.
    async fn cached_left_space_edges(
        &self,
        filters: &[SpaceFilter],
    ) -> HashMap<String, Vec<String>> {
        use matrix_sdk::{
            RoomState,
            deserialized_responses::SyncOrStrippedState,
            ruma::events::{SyncStateEvent, space::child::SpaceChildEventContent},
        };
        let mut pending: Vec<_> = filters
            .iter()
            .flat_map(|filter| filter.descendants.iter().cloned())
            .collect();
        let mut seen = HashSet::new();
        let mut edges = HashMap::new();
        while let Some(id) = pending.pop() {
            let Some(room) = self
                .client
                .get_room(&id)
                .filter(|room| room.is_space() && room.state() == RoomState::Left)
            else {
                continue;
            };
            if !seen.insert(id.clone()) {
                continue;
            }
            // Bound work on malicious or unusually large cached hierarchies.
            if seen.len() > 128 {
                break;
            }
            let Ok(events) = room
                .get_state_events_static::<SpaceChildEventContent>()
                .await
            else {
                continue;
            };
            let children: Vec<_> = events
                .into_iter()
                .filter_map(|raw| match raw.deserialize().ok()? {
                    SyncOrStrippedState::Sync(SyncStateEvent::Original(event))
                        if !event.content.via.is_empty() =>
                    {
                        Some(event.state_key)
                    }
                    SyncOrStrippedState::Stripped(event)
                        if event
                            .content
                            .via
                            .as_ref()
                            .is_some_and(|via| !via.is_empty()) =>
                    {
                        Some(event.state_key)
                    }
                    _ => None,
                })
                .collect();
            pending.extend(children.iter().cloned());
            edges.insert(
                id.to_string(),
                children.into_iter().map(|id| id.to_string()).collect(),
            );
        }
        edges
    }

    pub fn space_updates(&self) -> impl Stream<Item = SpaceSnapshot> + Send + 'static + use<> {
        let account = self.clone();
        async_stream::stream! {
            let service = SpaceService::new(account.client.clone()).await;
            let (mut filters, updates) = service.subscribe_to_space_filters().await;
            let mut updates = Box::pin(updates);
            let mut room_updates = account.client.subscribe_to_all_room_updates();
            let mut left_edges = HashMap::new();
            loop {
                let editable = service.editable_spaces().await.into_iter().map(|r| r.room_id.to_string()).collect();
                let joined = account.client.joined_space_rooms().iter().map(|room| room.room_id().to_string()).collect();
                if let Ok(edges) = tokio::time::timeout(std::time::Duration::from_secs(2), account.cached_left_space_edges(&filters.iter().cloned().collect::<Vec<_>>())).await {
                    left_edges = edges;
                }
                yield SpaceSnapshot {
                    account_instance: account.instance_id(),
                    spaces: summaries(filters.iter().cloned(), &editable, &joined, &left_edges),
                };
                tokio::select! {
                    next = updates.next() => { let Some(diffs) = next else { break; }; for diff in diffs { diff.apply(&mut filters); } }
                    update = room_updates.recv() => { if matches!(update, Err(tokio::sync::broadcast::error::RecvError::Closed)) { break; } }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn space_filter(id: &str, level: u8, children: &[&str]) -> SpaceFilter {
        use matrix_sdk::{RoomState, ruma::RoomId};
        SpaceFilter {
            level,
            descendants: children
                .iter()
                .map(|id| RoomId::parse(id).unwrap())
                .collect(),
            space_room: matrix_sdk_ui::spaces::SpaceRoom {
                room_id: RoomId::parse(id).unwrap(),
                display_name: "Test space".into(),
                canonical_alias: None,
                name: None,
                topic: None,
                avatar_url: None,
                room_type: Some(matrix_sdk::ruma::room::RoomType::Space),
                num_joined_members: 1,
                join_rule: None,
                world_readable: None,
                guest_can_join: false,
                is_direct: None,
                children_count: children.len() as u64,
                // Deliberately stale: current membership must win.
                state: Some(RoomState::Joined),
                heroes: None,
                via: Vec::new(),
                suggested: false,
                is_dm: None,
            },
        }
    }

    #[test]
    fn left_subspaces_disappear_even_when_hierarchy_filters_still_include_them() {
        let parent = "!parent:example.org";
        let child = "!child:example.org";
        let room = "!room:example.org";
        let filters = vec![
            space_filter(parent, 0, &[child]),
            space_filter(child, 1, &[room]),
        ];
        let editable = HashSet::new();
        let mut joined = HashSet::from([parent.to_owned(), child.to_owned()]);
        assert_eq!(
            summaries(
                filters.clone().into_iter(),
                &editable,
                &joined,
                &HashMap::new()
            )
            .len(),
            2
        );
        joined.remove(child);
        let remaining = summaries(
            filters.clone().into_iter(),
            &editable,
            &joined,
            &HashMap::new(),
        );
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].id, parent);
        // Leaving doesn't delete the space link or hide still-joined rooms
        // that belong under the parent through the left subspace.
        assert!(remaining[0].room_ids.contains(room));
        joined.insert(child.to_owned());
        assert_eq!(
            summaries(
                filters.clone().into_iter(),
                &editable,
                &joined,
                &HashMap::new()
            )
            .len(),
            2
        );
        assert!(
            summaries(
                filters.into_iter(),
                &editable,
                &HashSet::new(),
                &HashMap::new()
            )
            .is_empty()
        );
    }

    #[test]
    fn parent_keeps_rooms_through_a_left_space_with_an_empty_sdk_filter() {
        let parent = "!parent:example.org";
        let child = "!child:example.org";
        let room = "!room:example.org";
        // SDK rebuilds its joined graph after leaving: the child filter is
        // still advertised, but its descendants have become empty.
        let filters = vec![
            space_filter(parent, 0, &[child]),
            space_filter(child, 1, &[]),
        ];
        let joined = HashSet::from([parent.to_owned()]);
        let cached = HashMap::from([(child.to_owned(), vec![room.to_owned(), parent.to_owned()])]);
        let result = summaries(filters.into_iter(), &HashSet::new(), &joined, &cached);
        assert_eq!(result.len(), 1);
        assert_eq!(
            result[0].room_ids,
            HashSet::from([child.to_owned(), room.to_owned()])
        );
    }

    #[test]
    fn nested_spaces_expand_shared_rooms_without_cycles_or_duplicates() {
        let edges = HashMap::from([
            ("parent".into(), vec!["nested".into(), "shared".into()]),
            (
                "nested".into(),
                vec!["parent".into(), "child".into(), "shared".into()],
            ),
        ]);
        assert_eq!(
            descendants("parent", &edges),
            HashSet::from(["nested".into(), "shared".into(), "child".into(),])
        );
        assert!(descendants("unknown", &edges).is_empty());
    }
}
