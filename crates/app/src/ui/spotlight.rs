//! The title-bar search as a Spotlight-style panel: with an empty field it
//! shows the conversations you opened recently and where else to search;
//! typing lists matching conversations, then message and public room search.
use gpui_kit::base::TestSupportExt;
use gpui_kit::{
    component::{
        button::{Button, ButtonVariants},
        *,
    },
    *,
};

use crate::app::MatrixApp;
use crate::backend::Room;
use crate::theme::scaled_px as px;
use gpui_kit::assets::IconName;

/// Conversations remembered as recently viewed.
pub(crate) const RECENT_LIMIT: usize = 9;
/// Matching conversations listed while typing.
const MATCH_LIMIT: usize = 6;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum SpotlightItem {
    Room(String),
    SearchMessages(String),
    Explore(String),
    People,
}

impl MatrixApp {
    /// What Return and the arrow keys walk through, in display order.
    pub(crate) fn spotlight_items(&self, query: &str) -> Vec<SpotlightItem> {
        let query = query.trim();
        if query.is_empty() {
            let mut items: Vec<_> = self
                .recent_rooms()
                .into_iter()
                .map(|room| SpotlightItem::Room(room.id.clone()))
                .collect();
            items.push(SpotlightItem::Explore(String::new()));
            items.push(SpotlightItem::People);
            return items;
        }
        let mut items: Vec<_> = self
            .spotlight_matches(query)
            .into_iter()
            .map(|room| SpotlightItem::Room(room.id.clone()))
            .collect();
        items.push(SpotlightItem::SearchMessages(query.to_owned()));
        items.push(SpotlightItem::Explore(query.to_owned()));
        items
    }

    /// Recently opened conversations still joined, most recent first; before
    /// any were opened, the most active ones.
    fn recent_rooms(&self) -> Vec<&Room> {
        let mut rooms: Vec<&Room> = self
            .recent_room_ids
            .iter()
            .filter_map(|id| self.snapshot.rooms.iter().find(|room| &room.id == id))
            .filter(|room| !room.is_space)
            .collect();
        if rooms.is_empty() {
            let mut active: Vec<&Room> = self
                .snapshot
                .rooms
                .iter()
                .filter(|room| !room.is_space && !room.archived)
                .collect();
            active.sort_by_key(|room| std::cmp::Reverse(room.recency));
            rooms = active;
        }
        rooms.truncate(RECENT_LIMIT);
        rooms
    }

    fn spotlight_matches(&self, query: &str) -> Vec<&Room> {
        let query = query.to_lowercase();
        let mut rooms: Vec<(bool, &Room)> = self
            .snapshot
            .rooms
            .iter()
            .filter(|room| !room.is_space)
            .filter_map(|room| {
                let name = room.name.to_lowercase();
                name.contains(&query)
                    .then_some((name.starts_with(&query), room))
            })
            .collect();
        rooms.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.recency.cmp(&a.1.recency)));
        rooms
            .into_iter()
            .take(MATCH_LIMIT)
            .map(|(_, room)| room)
            .collect()
    }

    /// Open while the search field has focus, or after an explicit open.
    pub(crate) fn spotlight_visible(&self, window: &Window, cx: &App) -> bool {
        self.spotlight_open || self.search.read(cx).focus_handle(cx).is_focused(window)
    }

    /// Remember a conversation as recently viewed.
    pub(crate) fn note_recent_room(&mut self, id: &str) {
        self.recent_room_ids.retain(|room| room != id);
        self.recent_room_ids.insert(0, id.to_owned());
        self.recent_room_ids.truncate(RECENT_LIMIT);
    }

    pub(crate) fn close_spotlight(
        &mut self,
        clear: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.spotlight_open = false;
        self.spotlight_pointer = false;
        self.spotlight_selected = 0;
        if clear {
            self.search
                .update(cx, |state, cx| state.set_value("", window, cx));
        }
        self.focus.focus(window, cx);
        cx.notify();
    }

    pub(crate) fn activate_spotlight(
        &mut self,
        item: SpotlightItem,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        match item {
            SpotlightItem::Room(id) => {
                self.close_spotlight(true, window, cx);
                self.select_room(id, window, cx);
            }
            SpotlightItem::SearchMessages(_) => {
                self.spotlight_open = false;
                self.spotlight_pointer = false;
                // The query stays in the field; the results take focus.
                self.focus.focus(window, cx);
                self.run_message_search(window, cx);
            }
            SpotlightItem::Explore(query) => {
                self.close_spotlight(true, window, cx);
                self.open_explore(window, cx);
                if !query.is_empty()
                    && let Some(page) = &self.management
                {
                    page.name
                        .update(cx, |state, cx| state.set_value(query.clone(), window, cx));
                }
            }
            SpotlightItem::People => {
                self.close_spotlight(true, window, cx);
                self.open_management(crate::ui::spaces::ManageKind::Dm, window, cx);
            }
        }
    }

    /// Return in the search field: the selected item, or message search.
    pub(crate) fn spotlight_enter(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let query = self.search.read(cx).value().to_string();
        let items = self.spotlight_items(&query);
        let item = items
            .get(self.spotlight_selected)
            .cloned()
            .unwrap_or(SpotlightItem::SearchMessages(query));
        if let SpotlightItem::SearchMessages(text) = &item
            && text.trim().is_empty()
        {
            return;
        }
        self.activate_spotlight(item, window, cx);
    }

    /// ↑ ↓ and Escape while the panel is open; true when handled.
    pub(crate) fn spotlight_key(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        if !self.spotlight_visible(window, cx) {
            return false;
        }
        let query = self.search.read(cx).value().to_string();
        let count = self.spotlight_items(&query).len();
        match event.keystroke.key.as_str() {
            "escape" => self.close_spotlight(true, window, cx),
            "down" | "up" if count > 0 => {
                let step: isize = if event.keystroke.key == "up" { -1 } else { 1 };
                self.spotlight_selected =
                    (self.spotlight_selected as isize + step).rem_euclid(count as isize) as usize;
            }
            _ => return false,
        }
        cx.stop_propagation();
        window.prevent_default();
        cx.notify();
        true
    }

    pub(crate) fn spotlight_view(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        if !self.spotlight_visible(window, cx) {
            return None;
        }
        let muted = cx.theme().muted_foreground;
        let query = self.search.read(cx).value().trim().to_owned();
        let items = self.spotlight_items(&query);
        let selected = self.spotlight_selected.min(items.len().saturating_sub(1));
        let heading = |text: &'static str| {
            div()
                .px(px(10.))
                .pt(px(8.))
                .pb(px(4.))
                .text_size(px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(muted)
                .child(text)
        };
        let row = |index: usize, icon: AnyElement, title: String, detail: Option<String>| {
            let item = items[index].clone();
            // Message search keeps its long-standing ID for tests and focus.
            let id: SharedString = if matches!(item, SpotlightItem::SearchMessages(_)) {
                "search-messages".into()
            } else {
                format!("spotlight-{index}").into()
            };
            Button::new(id)
                .ghost()
                .w_full()
                .h(px(36.))
                .px(px(10.))
                .justify_start()
                .rounded(px(6.))
                .tab_stop(false)
                .selected(index == selected)
                .accessibility_label(match &detail {
                    Some(detail) => format!("{title}, {detail}"),
                    None => title.clone(),
                })
                .child(
                    h_flex()
                        .w_full()
                        .gap(px(10.))
                        .child(icon)
                        .child(div().flex_none().max_w(px(300.)).truncate().child(title))
                        .children(detail.map(|detail| {
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(px(12.))
                                .text_color(muted)
                                .child(detail)
                        })),
                )
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.activate_spotlight(item.clone(), window, cx)
                }))
        };
        let icon = |name: IconName| {
            div()
                .size(px(22.))
                .flex()
                .items_center()
                .justify_center()
                .child(Icon::new(name).size(px(16.)).text_color(muted))
                .into_any_element()
        };
        let room_index = |id: &str| {
            items
                .iter()
                .position(|item| matches!(item, SpotlightItem::Room(room) if room == id))
        };
        let mut body = v_flex().p(px(6.)).gap(px(1.));
        if query.is_empty() {
            let recent = self.recent_rooms();
            if !recent.is_empty() {
                body = body.child(heading("Recently viewed")).child(
                    h_flex()
                        .id("spotlight-recent")
                        .px(px(4.))
                        .pb(px(4.))
                        .gap(px(2.))
                        .children(recent.into_iter().filter_map(|room| {
                            let index = room_index(&room.id)?;
                            let item = items[index].clone();
                            Some(
                                Button::new(SharedString::from(format!("spotlight-{index}")))
                                    .ghost()
                                    .w(px(64.))
                                    .h(px(72.))
                                    .p(px(4.))
                                    .rounded(px(8.))
                                    .tab_stop(false)
                                    .selected(index == selected)
                                    .accessibility_label(room.name.clone())
                                    .child(
                                        v_flex()
                                            .items_center()
                                            .gap(px(4.))
                                            .child(self.avatar_image(
                                                room.avatar.as_deref(),
                                                &room.id,
                                                &room.name,
                                                36.,
                                                18.,
                                                cx,
                                            ))
                                            .child(
                                                div()
                                                    .w(px(56.))
                                                    .truncate()
                                                    .text_center()
                                                    .text_size(px(11.))
                                                    .child(room.name.clone()),
                                            ),
                                    )
                                    .on_click(cx.listener(move |this, _, window, cx| {
                                        this.activate_spotlight(item.clone(), window, cx)
                                    })),
                            )
                        }))
                        .test_support(),
                );
            }
            body = body.child(heading("Search for"));
            for (index, item) in items.iter().enumerate() {
                match item {
                    SpotlightItem::Explore(_) => {
                        body = body.child(row(
                            index,
                            icon(IconName::Compass),
                            "Public rooms".into(),
                            None,
                        ))
                    }
                    SpotlightItem::People => {
                        body = body.child(row(
                            index,
                            icon(IconName::CircleUser),
                            "People".into(),
                            None,
                        ))
                    }
                    _ => {}
                }
            }
            body = body.child(
                h_flex()
                    .h(px(36.))
                    .px(px(10.))
                    .gap(px(10.))
                    .text_color(muted)
                    .child(icon(IconName::MessageSquareText))
                    .child("Messages: type to search"),
            );
        } else {
            let matches = self.spotlight_matches(&query);
            if !matches.is_empty() {
                body = body.child(heading("Conversations"));
                for room in matches {
                    if let Some(index) = room_index(&room.id) {
                        let picture = self
                            .avatar_image(
                                room.avatar.as_deref(),
                                &room.id,
                                &room.name,
                                22.,
                                11.,
                                cx,
                            )
                            .into_any_element();
                        body = body.child(row(
                            index,
                            picture,
                            room.name.clone(),
                            room.topic.clone().or_else(|| room.partner.clone()),
                        ));
                    }
                }
            }
            body = body.child(heading("Search for"));
            for (index, item) in items.iter().enumerate() {
                match item {
                    SpotlightItem::SearchMessages(text) => {
                        body = body.child(row(
                            index,
                            icon(IconName::MessageSquareText),
                            format!("Messages containing “{text}”"),
                            None,
                        ))
                    }
                    SpotlightItem::Explore(text) => {
                        body = body.child(row(
                            index,
                            icon(IconName::Compass),
                            format!("Public rooms matching “{text}”"),
                            None,
                        ))
                    }
                    _ => {}
                }
            }
        }
        Some(
            div()
                .absolute()
                .top(px(crate::ui::TOOLBAR_HEIGHT - 6.))
                .left_0()
                .right_0()
                .flex()
                .justify_center()
                .child(
                    v_flex()
                        .id("spotlight")
                        .role(Role::Dialog)
                        .aria_label("Search")
                        .w(px(560.))
                        .max_w(relative(0.9))
                        .max_h(px(520.))
                        .overflow_y_scroll()
                        .rounded(px(12.))
                        .border_1()
                        .border_color(cx.theme().border)
                        .bg(cx.theme().popover)
                        .shadow_lg()
                        .occlude()
                        // A press inside must not count as leaving the field.
                        .capture_any_mouse_down(cx.listener(|this, _, _, _| {
                            this.spotlight_pointer = true;
                        }))
                        .child(body)
                        .child(
                            div()
                                .px(px(16.))
                                .py(px(6.))
                                .border_t_1()
                                .border_color(cx.theme().border)
                                .text_size(px(10.5))
                                .text_color(muted)
                                .child("↑ ↓ to choose · Return to open · Escape to close"),
                        )
                        .test_support(),
                )
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn recent_rooms_keep_the_latest_first_without_duplicates() {
        let mut ids: Vec<String> = Vec::new();
        let mut note = |id: &str| {
            ids.retain(|room| room != id);
            ids.insert(0, id.to_owned());
            ids.truncate(super::RECENT_LIMIT);
        };
        for id in ["a", "b", "a", "c"] {
            note(id);
        }
        assert_eq!(ids, ["c", "a", "b"]);
    }
}
