//! Sidebar ordering. Rooms and sections follow the shared order by default.
//! Holding the local-reorder key (Ctrl; Command on macOS, where Control-click
//! is a secondary click) lets anyone drag rooms and sections into a personal
//! order that only this device shows. Rooms moved that way carry a small
//! "differs from the server" mark. A plain drag changes the shared order and is
//! offered only to people who may edit the space, and only in the offline
//! preview until the backend can write the space's order.
use super::*;
use std::collections::{HashMap, HashSet};

/// The scope of the sidebar when no space is selected.
pub(crate) const HOME: &str = "home";
/// Sidebar sections, in their default order.
pub(crate) const SECTIONS: [&str; 3] = ["favourites", "rooms", "direct"];

/// The key held for a personal reorder.
pub(crate) fn local_key(modifiers: &Modifiers) -> bool {
    if cfg!(target_os = "macos") {
        modifiers.platform
    } else {
        modifiers.control
    }
}

/// The key's name, as people see it on their keyboard.
pub(crate) fn local_key_name() -> &'static str {
    if cfg!(target_os = "macos") {
        "Command (⌘)"
    } else {
        "Ctrl"
    }
}

/// Session ordering state. Personal orders live only in memory for now; see
/// PLAN.md for the account-scoped storage they still need.
#[derive(Default)]
pub(crate) struct SidebarOrder {
    /// Personal room order per (scope, section).
    local: HashMap<(String, String), Vec<String>>,
    /// Rooms placed by a personal move, per scope, while they differ.
    moved: HashSet<(String, String)>,
    /// Personal section order per scope.
    sections: HashMap<String, Vec<String>>,
    /// Sample stand-in for the space's shared order (offline preview only).
    server: HashMap<(String, String), Vec<String>>,
}

fn sort_by_list<T>(items: &mut [T], id: impl Fn(&T) -> &str, list: &[String]) {
    // Stable: rooms the list does not know yet keep their relative order, last.
    items.sort_by_key(|item| {
        list.iter()
            .position(|known| known == id(item))
            .unwrap_or(usize::MAX)
    });
}

/// `ids` with `from` taking `to`'s place.
fn moved_to(ids: &[String], from: &str, to: &str) -> Option<Vec<String>> {
    let from_ix = ids.iter().position(|id| id == from)?;
    let to_ix = ids.iter().position(|id| id == to)?;
    if from_ix == to_ix {
        return None;
    }
    let mut ids = ids.to_vec();
    let item = ids.remove(from_ix);
    ids.insert(to_ix, item);
    Some(ids)
}

impl SidebarOrder {
    fn key(scope: &str, section: &str) -> (String, String) {
        (scope.to_owned(), section.to_owned())
    }

    /// The shared order: the server's (sample) order, else the incoming order.
    fn shared<'a>(&self, scope: &str, section: &str, mut rooms: Vec<&'a Room>) -> Vec<&'a Room> {
        if let Some(list) = self.server.get(&Self::key(scope, section)) {
            sort_by_list(&mut rooms, |room| room.id.as_str(), list);
        }
        rooms
    }

    /// IDs of `rooms` in the shared order.
    pub fn shared_ids(&self, scope: &str, section: &str, rooms: &[&Room]) -> Vec<String> {
        self.shared(scope, section, rooms.to_vec())
            .iter()
            .map(|room| room.id.clone())
            .collect()
    }

    /// Rooms as shown: the shared order, then any personal order on top.
    pub fn arrange<'a>(&self, scope: &str, section: &str, rooms: Vec<&'a Room>) -> Vec<&'a Room> {
        let mut rooms = self.shared(scope, section, rooms);
        if let Some(list) = self.local.get(&Self::key(scope, section)) {
            sort_by_list(&mut rooms, |room| room.id.as_str(), list);
        }
        rooms
    }

    /// Moves `from` to `to`'s place for this person only. `shown` is the
    /// section as displayed; `shared` the same rooms in the shared order.
    pub fn move_local(
        &mut self,
        scope: &str,
        section: &str,
        shown: &[String],
        shared: &[String],
        from: &str,
        to: &str,
    ) -> bool {
        let Some(ids) = moved_to(shown, from, to) else {
            return false;
        };
        let key = (scope.to_owned(), from.to_owned());
        // A room moved back to its shared place no longer differs.
        if ids.iter().position(|id| id == from) == shared.iter().position(|id| id == from) {
            self.moved.remove(&key);
        } else {
            self.moved.insert(key);
        }
        if ids == shared {
            self.local.remove(&Self::key(scope, section));
            self.moved
                .retain(|(moved_scope, room)| moved_scope != scope || !shared.contains(room));
        } else {
            self.local.insert(Self::key(scope, section), ids);
        }
        true
    }

    /// Sample only: changes the shared order, as a moderator would.
    pub fn move_shared(
        &mut self,
        scope: &str,
        section: &str,
        shared: &[String],
        from: &str,
        to: &str,
    ) -> bool {
        let Some(ids) = moved_to(shared, from, to) else {
            return false;
        };
        self.server.insert(Self::key(scope, section), ids);
        true
    }

    pub fn is_moved(&self, scope: &str, room: &str) -> bool {
        self.moved.contains(&(scope.to_owned(), room.to_owned()))
    }

    pub fn sections(&self, scope: &str) -> Vec<&'static str> {
        let mut sections = SECTIONS.to_vec();
        if let Some(list) = self.sections.get(scope) {
            sort_by_list(&mut sections, |key| key, list);
        }
        sections
    }

    pub fn section_moved(&self, scope: &str, section: &str) -> bool {
        self.sections.get(scope).is_some_and(|list| {
            list.iter().position(|key| key == section)
                != SECTIONS.iter().position(|key| *key == section)
        })
    }

    pub fn move_section(&mut self, scope: &str, from: &str, to: &str) -> bool {
        let shown: Vec<String> = self
            .sections(scope)
            .iter()
            .map(|key| key.to_string())
            .collect();
        let Some(ids) = moved_to(&shown, from, to) else {
            return false;
        };
        if ids.iter().map(String::as_str).eq(SECTIONS) {
            self.sections.remove(scope);
        } else {
            self.sections.insert(scope.to_owned(), ids);
        }
        true
    }

    /// Anything here differs from the shared order.
    pub fn has_local(&self, scope: &str) -> bool {
        self.sections.contains_key(scope)
            || self
                .local
                .keys()
                .any(|(local_scope, _)| local_scope == scope)
    }

    /// Back to the shared order for rooms and sections in `scope`.
    pub fn reset(&mut self, scope: &str) {
        self.sections.remove(scope);
        self.local
            .retain(|(local_scope, _), _| local_scope != scope);
        self.moved.retain(|(moved_scope, _)| moved_scope != scope);
    }
}

/// Where a sidebar row sits, for the keyboard-reachable Move Up/Down items.
pub(crate) struct RowOrder {
    pub section: &'static str,
    pub shown: Vec<String>,
    pub shared: Vec<String>,
    pub up: Option<String>,
    pub down: Option<String>,
}

/// A room being dragged in the sidebar.
#[derive(Clone)]
pub(crate) struct RoomDrag {
    pub scope: String,
    pub section: &'static str,
    pub id: String,
    pub name: String,
    /// A personal reorder (the key was held when the drag began).
    pub local: bool,
}

/// A section header being dragged (personal reorder only).
#[derive(Clone)]
pub(crate) struct SectionDrag {
    pub scope: String,
    pub section: &'static str,
    pub label: &'static str,
}

/// What follows the pointer during a drag.
pub(crate) struct DragGhost {
    label: SharedString,
    local: bool,
}

impl DragGhost {
    pub fn new(label: impl Into<SharedString>, local: bool, cx: &mut App) -> Entity<Self> {
        cx.new(|_| Self {
            label: label.into(),
            local,
        })
    }
}

impl Render for DragGhost {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .h(px(26.))
            .px(px(10.))
            .gap(px(6.))
            .rounded(px(6.))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().popover)
            .shadow_md()
            .text_size(px(12.))
            .text_color(cx.theme().foreground)
            .child(self.label.clone())
            .when(self.local, |ghost| {
                ghost.child(
                    div()
                        .text_color(cx.theme().muted_foreground)
                        .child("· just for you"),
                )
            })
    }
}

impl MatrixApp {
    /// The sidebar scope: the selected space, or Home.
    pub(crate) fn order_scope(&self) -> String {
        self.navigation
            .selected_space
            .clone()
            .unwrap_or_else(|| HOME.to_owned())
    }

    /// The personal-reorder key is held over the sidebar.
    pub(crate) fn local_reorder_active(&self) -> bool {
        self.reorder_key && self.sidebar_hovered
    }

    /// A plain drag may change the shared order here: you can edit the space,
    /// and (until the backend writes it) only in the offline preview.
    pub(crate) fn can_reorder_shared(&self, scope: &str, section: &str) -> bool {
        self.preview
            && section == "rooms"
            && self
                .snapshot
                .spaces
                .iter()
                .any(|space| space.id == scope && space.can_add_rooms)
    }

    pub(crate) fn drop_room(
        &mut self,
        drag: &RoomDrag,
        target: &str,
        shown: &[String],
        shared: &[String],
        cx: &mut Context<Self>,
    ) {
        let changed = if drag.local {
            self.sidebar_order.move_local(
                &drag.scope,
                drag.section,
                shown,
                shared,
                &drag.id,
                target,
            )
        } else if self.can_reorder_shared(&drag.scope, drag.section) {
            let changed =
                self.sidebar_order
                    .move_shared(&drag.scope, drag.section, shared, &drag.id, target);
            if changed {
                self.status = "Sample data · The order changed for everyone in a real space; nothing was sent.".into();
            }
            changed
        } else {
            false
        };
        if changed {
            cx.notify();
        }
    }

    pub(crate) fn drop_section(
        &mut self,
        drag: &SectionDrag,
        target: &str,
        cx: &mut Context<Self>,
    ) {
        if self
            .sidebar_order
            .move_section(&drag.scope, drag.section, target)
        {
            cx.notify();
        }
    }

    pub(crate) fn reset_local_order(&mut self, cx: &mut Context<Self>) {
        let scope = self.order_scope();
        self.sidebar_order.reset(&scope);
        cx.notify();
    }

    /// The small "!" in a circle beside something arranged just for you.
    pub(crate) fn local_order_mark(&self, id: SharedString, cx: &App) -> AnyElement {
        div()
            .id(id)
            .flex_none()
            .aria_label("Locally different from the server's ordering")
            .child(
                Icon::new(IconName::CircleAlert)
                    .size(px(12.))
                    .text_color(theme::palette(cx).warning),
            )
            .tooltip(|window, cx| {
                gpui_kit::component::tooltip::Tooltip::new(
                    "Locally different from the server's ordering",
                )
                .build(window, cx)
            })
            .into_any_element()
    }

    /// Shown while the key is held: the next drag is personal.
    pub(crate) fn local_reorder_banner(&self, cx: &mut Context<Self>) -> AnyElement {
        let palette = theme::palette(cx);
        motion::rise(
            v_flex()
                .id("local-reorder-banner")
                .role(Role::Alert)
                .aria_label("Reordering just for you. Others keep the server's order.")
                .mx(px(8.))
                .mb(px(4.))
                .px(px(10.))
                .py(px(7.))
                .gap(px(2.))
                .rounded(px(7.))
                .bg(palette.warning_background)
                .text_color(palette.warning)
                .child(
                    h_flex()
                        .gap(px(6.))
                        .text_size(px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(Icon::new(IconName::CircleAlert).size(px(13.)))
                        .child("Reordering just for you"),
                )
                .child(div().text_size(px(11.5)).child(format!(
                    "Drag rooms or sections. Others keep the server's order. Release {} to stop.",
                    local_key_name()
                ))),
            "local-reorder-banner-motion",
            self.snapshot.reduce_motion,
        )
        .into_any_element()
    }

    /// The tip behind the sidebar's info button, with its two ways to go away.
    pub(crate) fn reorder_tip(&self, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let scope = self.order_scope();
        let has_local = self.sidebar_order.has_local(&scope);
        v_flex()
            .id("reorder-tip")
            .role(Role::Region)
            .aria_label("Arranging rooms")
            .mx(px(8.))
            .mb(px(6.))
            .p(px(10.))
            .gap(px(6.))
            .rounded(px(8.))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(
                div()
                    .text_size(px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Arrange rooms your way"),
            )
            .child(div().text_size(px(12.)).text_color(muted).child(format!(
                "Moderators set the order everyone sees. Hold {} and drag rooms or sections to arrange them just for you, on this device.",
                local_key_name()
            )))
            .child(
                h_flex()
                    .pt(px(2.))
                    .gap(px(6.))
                    .flex_wrap()
                    .child(
                        Button::new("reorder-tip-hide")
                            .small()
                            .outline()
                            .label("Hide for Now")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.reorder_tip_open = false;
                                this.reorder_tip_hidden = true;
                                cx.notify();
                            })),
                    )
                    .child(
                        Button::new("reorder-tip-never")
                            .small()
                            .ghost()
                            .label("Don't Show Again")
                            .on_click(cx.listener(|this, _, window, cx| {
                                this.reorder_tip_open = false;
                                this.preferences.sidebar.reorder_tip_dismissed = true;
                                this.save_preferences(window, cx);
                                cx.notify();
                            })),
                    )
                    .when(has_local, |row| {
                        row.child(
                            Button::new("reorder-tip-reset")
                                .small()
                                .ghost()
                                .label("Reset My Order")
                                .on_click(cx.listener(|this, _, _, cx| this.reset_local_order(cx))),
                        )
                    }),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    // Not a glob: the UI prelude exports GPUI's `test` attribute.
    use super::{Room, SidebarOrder};

    fn room(id: &str) -> Room {
        Room {
            id: id.into(),
            ..Default::default()
        }
    }
    fn ids(rooms: &[&Room]) -> Vec<String> {
        rooms.iter().map(|room| room.id.clone()).collect()
    }

    #[test]
    fn personal_moves_reorder_only_locally_and_mark_the_moved_room() {
        let rooms = [room("a"), room("b"), room("c")];
        let mut order = SidebarOrder::default();
        let shared = ids(&order.arrange("s", "rooms", rooms.iter().collect()));
        assert!(order.move_local("s", "rooms", &shared, &shared, "c", "a"));
        let shown = ids(&order.arrange("s", "rooms", rooms.iter().collect()));
        assert_eq!(shown, ["c", "a", "b"]);
        assert!(order.is_moved("s", "c"));
        assert!(!order.is_moved("s", "a"));
        assert!(order.has_local("s"));
        // Other scopes are untouched.
        assert!(!order.has_local("home"));
        // Moving it back restores the shared order and clears the mark.
        assert!(order.move_local("s", "rooms", &shown, &shared, "c", "b"));
        assert_eq!(
            ids(&order.arrange("s", "rooms", rooms.iter().collect())),
            ["a", "b", "c"]
        );
        assert!(!order.is_moved("s", "c"));
        assert!(!order.has_local("s"));
    }

    #[test]
    fn new_rooms_appear_after_a_personal_order_and_reset_clears_it() {
        let mut order = SidebarOrder::default();
        let first = [room("a"), room("b")];
        let shared = ids(&first.iter().collect::<Vec<_>>());
        order.move_local("s", "rooms", &shared, &shared, "b", "a");
        let later = [room("a"), room("new"), room("b")];
        assert_eq!(
            ids(&order.arrange("s", "rooms", later.iter().collect())),
            ["b", "a", "new"]
        );
        order.reset("s");
        assert_eq!(
            ids(&order.arrange("s", "rooms", later.iter().collect())),
            ["a", "new", "b"]
        );
        assert!(!order.is_moved("s", "b"));
    }

    #[test]
    fn sections_reorder_per_scope_and_report_what_moved() {
        let mut order = SidebarOrder::default();
        assert!(order.move_section("home", "direct", "favourites"));
        assert_eq!(order.sections("home"), ["direct", "favourites", "rooms"]);
        assert!(order.section_moved("home", "direct"));
        assert_eq!(order.sections("s"), ["favourites", "rooms", "direct"]);
        assert!(order.move_section("home", "direct", "rooms"));
        assert!(!order.has_local("home"));
    }

    #[test]
    fn shared_order_is_the_base_for_personal_order() {
        let rooms = [room("a"), room("b"), room("c")];
        let mut order = SidebarOrder::default();
        let incoming = ids(&rooms.iter().collect::<Vec<_>>());
        assert!(order.move_shared("s", "rooms", &incoming, "a", "c"));
        let shared = ids(&order.arrange("s", "rooms", rooms.iter().collect()));
        assert_eq!(shared, ["b", "c", "a"]);
        assert!(!order.is_moved("s", "a"));
        order.move_local("s", "rooms", &shared, &shared, "a", "b");
        assert_eq!(
            ids(&order.arrange("s", "rooms", rooms.iter().collect())),
            ["a", "b", "c"]
        );
        assert!(order.is_moved("s", "a"));
    }
}
