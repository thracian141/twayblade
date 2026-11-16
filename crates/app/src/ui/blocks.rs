//! Information channels: a read-mostly room of formatted blocks, such as rules,
//! guides and announcements. There is no chat composer. Moderators add a block
//! through an editor with a formatting toolbar and a live preview of exactly
//! what will be sent (Markdown, sanitized HTML, then rich text, as received).
use super::*;
use gpui_kit::component::input::{InputEvent, TextareaState};
use std::sync::Arc;
use twayblade_core::rich_text::{FormattingError, RichText};

/// The reading column, so long rules keep a comfortable line length.
const BLOCK_COLUMN: f32 = 720.;

pub(crate) struct BlockEditor {
    pub room: String,
    pub input: Entity<TextareaState>,
    /// The source as the timeline will show it, recomputed on every change.
    preview: Result<Option<Arc<RichText>>, FormattingError>,
    pub publishing: bool,
    pub status: String,
    _subscription: Subscription,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Format {
    Heading,
    Bold,
    Italic,
    Strike,
    Code,
    Link,
    Bullets,
    Numbers,
    Quote,
    Table,
    Divider,
}

impl Format {
    fn icon(self) -> IconName {
        match self {
            Self::Heading => IconName::Heading2,
            Self::Bold => IconName::Bold,
            Self::Italic => IconName::Italic,
            Self::Strike => IconName::Strikethrough,
            Self::Code => IconName::Code,
            Self::Link => IconName::Link,
            Self::Bullets => IconName::List,
            Self::Numbers => IconName::ListOrdered,
            Self::Quote => IconName::TextQuote,
            Self::Table => IconName::Table,
            Self::Divider => IconName::SeparatorHorizontal,
        }
    }
    fn label(self) -> &'static str {
        match self {
            Self::Heading => "Heading",
            Self::Bold => "Bold",
            Self::Italic => "Italic",
            Self::Strike => "Strikethrough",
            Self::Code => "Code",
            Self::Link => "Link",
            Self::Bullets => "Bulleted List",
            Self::Numbers => "Numbered List",
            Self::Quote => "Quote",
            Self::Table => "Table",
            Self::Divider => "Divider",
        }
    }
    fn id(self) -> &'static str {
        match self {
            Self::Heading => "block-format-heading",
            Self::Bold => "block-format-bold",
            Self::Italic => "block-format-italic",
            Self::Strike => "block-format-strike",
            Self::Code => "block-format-code",
            Self::Link => "block-format-link",
            Self::Bullets => "block-format-bullets",
            Self::Numbers => "block-format-numbers",
            Self::Quote => "block-format-quote",
            Self::Table => "block-format-table",
            Self::Divider => "block-format-divider",
        }
    }
}

/// The toolbar, in groups: inline text, paragraphs, then inserted blocks.
const TOOLBAR: [&[Format]; 3] = [
    &[
        Format::Heading,
        Format::Bold,
        Format::Italic,
        Format::Strike,
        Format::Code,
        Format::Link,
    ],
    &[Format::Bullets, Format::Numbers, Format::Quote],
    &[Format::Table, Format::Divider],
];

const TABLE_TEMPLATE: &str = "| Column | Column |\n| --- | --- |\n| Cell | Cell |\n";

/// Applies `format` to `text` with the UTF-8 `selection`. Returns the range to
/// replace, its replacement, and the range to select afterwards (absolute,
/// in the new text).
pub(crate) fn apply_format(
    text: &str,
    selection: std::ops::Range<usize>,
    format: Format,
) -> (std::ops::Range<usize>, String, std::ops::Range<usize>) {
    let start = selection.start.min(text.len());
    let end = selection.end.clamp(start, text.len());
    let selected = &text[start..end];
    let wrap = |open: &str, close: &str, placeholder: &str| {
        let inner = if selected.is_empty() {
            placeholder
        } else {
            selected
        };
        let replacement = format!("{open}{inner}{close}");
        let inner_start = start + open.len();
        (
            start..end,
            replacement,
            inner_start..inner_start + inner.len(),
        )
    };
    match format {
        Format::Bold => wrap("**", "**", "bold text"),
        Format::Italic => wrap("_", "_", "italic text"),
        Format::Strike => wrap("~~", "~~", "struck text"),
        Format::Code => wrap("`", "`", "code"),
        Format::Link => {
            let label = if selected.is_empty() {
                "link text"
            } else {
                selected
            };
            let replacement = format!("[{label}](https://)");
            // Select the address, the part still to fill in.
            let url_start = start + 1 + label.len() + 2;
            (
                start..end,
                replacement,
                url_start..url_start + "https://".len(),
            )
        }
        Format::Heading | Format::Bullets | Format::Numbers | Format::Quote => {
            // Prefix every line the selection touches.
            let line_start = text[..start].rfind('\n').map_or(0, |ix| ix + 1);
            let line_end = text[end..].find('\n').map_or(text.len(), |ix| end + ix);
            let lines: Vec<String> = text[line_start..line_end]
                .split('\n')
                .enumerate()
                .map(|(ix, line)| {
                    match format {
                        // Replace an existing heading level rather than nest it.
                        Format::Heading => format!("## {}", line.trim_start_matches(['#', ' '])),
                        Format::Bullets => format!("- {line}"),
                        Format::Numbers => format!("{}. {line}", ix + 1),
                        _ => format!("> {line}"),
                    }
                })
                .collect();
            let replacement = lines.join("\n");
            let caret = line_start + replacement.len();
            (line_start..line_end, replacement, caret..caret)
        }
        Format::Table | Format::Divider => {
            // Block syntax needs a blank line before it: "---" straight after
            // a line of text would turn that line into a heading instead.
            let before = &text[..start];
            let lead = if before.is_empty() || before.ends_with("\n\n") {
                ""
            } else if before.ends_with('\n') {
                "\n"
            } else {
                "\n\n"
            };
            let body = if format == Format::Table {
                TABLE_TEMPLATE
            } else {
                "---\n"
            };
            let replacement = format!("{lead}{body}\n");
            if format == Format::Table {
                // Select the first header so typing renames it.
                let header = start + lead.len() + 2;
                (start..end, replacement, header..header + "Column".len())
            } else {
                let caret = start + replacement.len();
                (start..end, replacement, caret..caret)
            }
        }
    }
}

impl MatrixApp {
    /// You may add blocks: an admin or moderator of the open room.
    ///
    /// Placeholder until the backend exposes the room's power level for
    /// sending messages; the server still enforces its own rules.
    pub(crate) fn can_publish_blocks(&self) -> bool {
        self.snapshot.members.iter().any(|member| {
            member.user_id == self.snapshot.user_id
                && matches!(member.role, MemberRole::Admin | MemberRole::Moderator)
        })
    }

    pub(crate) fn open_block_editor(
        &mut self,
        room: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.emoji_picker = None;
        let input = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder("Write the block. Use the toolbar for headings, lists and tables.")
                .auto_grow(14, 28)
        });
        let subscription =
            cx.subscribe_in(&input, window, |this, state, event: &InputEvent, _, cx| {
                if matches!(event, InputEvent::Change)
                    && let Some(editor) = &mut this.block_editor
                {
                    editor.preview =
                        twayblade_core::rich_text::compose_preview(&state.read(cx).value());
                    editor.status.clear();
                    cx.notify();
                }
            });
        input.update(cx, |state, cx| state.focus(window, cx));
        self.block_editor = Some(BlockEditor {
            room,
            input,
            preview: Err(FormattingError::Empty),
            publishing: false,
            status: String::new(),
            _subscription: subscription,
        });
        cx.notify();
    }

    pub(crate) fn close_block_editor(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.block_editor.take().is_some() {
            self.focus.focus(window, cx);
            cx.notify();
        }
    }

    fn format_block(&mut self, format: Format, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &self.block_editor else {
            return;
        };
        editor.input.update(cx, |state, cx| {
            let text = state.value().to_string();
            let (range, replacement, select) = apply_format(&text, state.selected_range(), format);
            state.set_selected_range(range, cx);
            state.replace(replacement, window, cx);
            state.set_selected_range(select, cx);
            state.focus(window, cx);
        });
    }

    fn publish_block(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(editor) = &mut self.block_editor else {
            return;
        };
        let body = editor.input.read(cx).value().to_string();
        if editor.publishing || body.trim().is_empty() || editor.preview.is_err() {
            return;
        }
        let room = editor.room.clone();
        if self.preview {
            // Offline sample: show the block here; nothing is sent.
            let rich = editor.preview.clone().ok().flatten();
            let ix = self.snapshot.messages.len();
            self.snapshot.messages.push(crate::backend::Message {
                id: format!("sample-block-{ix}"),
                event_id: Some(format!("$sample-block-{ix}:preview.invalid")),
                sender: self.snapshot.user_id.clone(),
                display_name: "You".into(),
                body,
                rich,
                kind: MessageKind::Text,
                timestamp_ms: chrono::Utc::now().timestamp_millis(),
                delivery: DeliveryState::Remote,
                ..Default::default()
            });
            self.groups = model::grouped(&self.snapshot.messages);
            let last = self.groups.len().saturating_sub(1);
            self.scroller.update(cx, |state, cx| {
                state.reset(self.groups.len(), cx);
                state.scroll_to_item(last, cx);
            });
            self.status = "Sample data · The block was added on this device only.".into();
            self.close_block_editor(window, cx);
            return;
        }
        editor.publishing = true;
        editor.status.clear();
        let account = self.snapshot.user_id.clone();
        self.job(
            window,
            cx,
            move |backend| backend.send_text(room, body),
            move |this, result, window, cx| {
                if this.snapshot.user_id != account {
                    return;
                }
                match result {
                    Ok(()) => this.close_block_editor(window, cx),
                    Err(error) => {
                        if let Some(editor) = &mut this.block_editor {
                            editor.publishing = false;
                            editor.status = error.to_string();
                        }
                    }
                }
            },
        );
        cx.notify();
    }

    /// One timeline group, shown as full-width blocks rather than chat rows.
    pub(crate) fn block_group(&self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(range) = self.groups.get(ix).cloned() else {
            return div().into_any_element();
        };
        v_flex()
            .w_full()
            .items_center()
            .px(px(20.))
            .pt(px(14.))
            .when(ix + 1 == self.groups.len(), |list| list.pb(px(14.)))
            .gap(px(14.))
            .children(
                self.snapshot.messages[range]
                    .iter()
                    .map(|message| self.block_card(message, cx)),
            )
            .into_any_element()
    }

    fn block_card(&self, message: &crate::backend::Message, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let content = match message.kind {
            MessageKind::Undecryptable => div()
                .italic()
                .text_color(muted)
                .child("This block can't be decrypted yet.")
                .into_any_element(),
            _ => self.block_text(
                SharedString::from(format!("block-{}", message.id)),
                message.rich.as_ref(),
                &model::message_text(message),
                cx,
            ),
        };
        let when = Local
            .timestamp_millis_opt(message.timestamp_ms)
            .single()
            .map(|time| time.format("%-d %B %Y, %H:%M").to_string())
            .unwrap_or_default();
        self.block_frame(
            SharedString::from(format!("block-card-{}", message.id)),
            content,
            self.block_byline(
                &message.sender,
                message.sender_avatar.as_deref(),
                &message.display_name,
                when,
                message.edited,
                cx,
            ),
            cx,
        )
    }

    /// The card around a block: the same frame in the channel and the preview.
    fn block_frame(
        &self,
        id: SharedString,
        content: AnyElement,
        byline: AnyElement,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        v_flex()
            .id(id)
            .w_full()
            .max_w(px(BLOCK_COLUMN))
            .rounded(px(10.))
            .border_1()
            .border_color(cx.theme().border)
            .bg(cx.theme().background)
            .child(div().px(px(22.)).pt(px(18.)).pb(px(16.)).child(content))
            .child(
                div()
                    .px(px(22.))
                    .py(px(8.))
                    .border_t_1()
                    .border_color(cx.theme().border)
                    .child(byline),
            )
            .into_any_element()
    }

    fn block_text(
        &self,
        id: SharedString,
        rich: Option<&Arc<RichText>>,
        plain: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        div()
            .font_family(theme::message_font(cx))
            .text_size(px(f32::from(self.preferences.appearance.message_size)))
            .child(
                TextView::markdown(
                    id,
                    match rich {
                        Some(rich) => model::rich_markdown(rich),
                        None => model::plain_text(plain),
                    },
                )
                .selectable(true)
                .link_tooltip(|url, window, cx| {
                    gpui_kit::component::tooltip::Tooltip::new(url.clone()).build(window, cx)
                })
                .on_link_click(|url, _, _, cx| {
                    if url::Url::parse(url)
                        .is_ok_and(|url| matches!(url.scheme(), "https" | "http" | "mailto"))
                    {
                        cx.open_url(url);
                    }
                }),
            )
            .into_any_element()
    }

    fn block_byline(
        &self,
        user_id: &str,
        avatar: Option<&str>,
        name: &str,
        when: String,
        edited: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        h_flex()
            .gap(px(6.))
            .text_size(px(11.5))
            .text_color(muted)
            .child(self.avatar_image(avatar, user_id, name, 16., 8., cx))
            .child(
                div()
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(cx.theme().foreground)
                    .child(name.to_owned()),
            )
            .child(when)
            .when(edited, |row| row.child("· Edited"))
            .into_any_element()
    }

    /// No blocks yet: say what the channel is for and who can add to it.
    pub(crate) fn blocks_empty(&self, room: &Room, cx: &mut Context<Self>) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let can_publish = self.can_publish_blocks();
        let room_id = room.id.clone();
        v_flex()
            .flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .gap(px(6.))
            .px(px(24.))
            .child(Icon::new(IconName::ScrollText).size(px(32.)).text_color(muted))
            .child(
                div()
                    .pt(px(4.))
                    .text_size(px(14.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("No blocks yet"),
            )
            .child(
                div()
                    .max_w(px(360.))
                    .text_center()
                    .text_color(muted)
                    .child(if can_publish {
                        "Add rules, guides or announcements as formatted blocks. Members can read them but can't reply here."
                    } else {
                        "Moderators haven't added anything to this channel yet."
                    }),
            )
            .when(can_publish, |view| {
                view.child(
                    Button::new("blocks-empty-new")
                        .primary()
                        .mt(px(8.))
                        .h(px(28.))
                        .px(px(12.))
                        .icon(IconName::Plus)
                        .label("New Block")
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.open_block_editor(room_id.clone(), window, cx)
                        })),
                )
            })
            .into_any_element()
    }

    /// Takes the composer's place: no message field, only the way to add a block.
    pub(crate) fn blocks_footer(
        &self,
        room: &Room,
        notices: Vec<String>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = cx.theme().muted_foreground;
        let can_publish = self.can_publish_blocks();
        let room_id = room.id.clone();
        v_flex()
            .px(px(20.))
            .pt(px(2.))
            .pb(px(10.))
            .gap(px(4.))
            .children(notices.into_iter().enumerate().map(|(index, text)| {
                div()
                    .id(("blocks-note", index))
                    .role(Role::Status)
                    .aria_label(text.clone())
                    .pl(px(12.))
                    .text_size(px(13.))
                    .text_color(muted)
                    .child(text)
            }))
            .child(
                h_flex()
                    .id("blocks-footer")
                    .min_h(px(44.))
                    .px(px(12.))
                    .gap(px(8.))
                    .rounded(px(9.))
                    .border_1()
                    .border_color(cx.theme().border)
                    .bg(cx.theme().secondary.opacity(0.5))
                    .child(
                        Icon::new(if can_publish {
                            IconName::ScrollText
                        } else {
                            IconName::Lock
                        })
                        .size(px(14.))
                        .text_color(muted),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .text_size(px(12.))
                            .text_color(muted)
                            .child(if can_publish {
                                "This is an information channel. Members read it; moderators add blocks."
                            } else {
                                "This is an information channel. Only moderators can add to it."
                            }),
                    )
                    .when(can_publish, |row| {
                        row.child(
                            Button::new("new-block")
                                .primary()
                                .h(px(28.))
                                .px(px(12.))
                                .rounded(px(6.))
                                .icon(IconName::Plus)
                                .label("New Block")
                                .tooltip("Add a formatted block to this channel")
                                .disabled(self.block_editor.is_some())
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.open_block_editor(room_id.clone(), window, cx)
                                })),
                        )
                    }),
            )
            .into_any_element()
    }

    /// The block editor sheet: toolbar, source and live preview side by side.
    pub(crate) fn block_editor_view(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let editor = self.block_editor.as_ref()?;
        let muted = cx.theme().muted_foreground;
        let palette = theme::palette(cx);
        let room_name = self
            .snapshot
            .rooms
            .iter()
            .find(|room| room.id == editor.room)
            .map(|room| room.name.clone())
            .unwrap_or_default();
        let empty = editor.input.read(cx).value().trim().is_empty();
        let (preview, problem) = match &editor.preview {
            Ok(rich) => (
                Some(self.block_text(
                    "block-editor-preview".into(),
                    rich.as_ref(),
                    &editor.input.read(cx).value(),
                    cx,
                )),
                None,
            ),
            Err(FormattingError::Empty) => (None, None),
            Err(FormattingError::Limit) => (None, Some("This block is too long to publish.")),
        };
        let mut toolbar = h_flex()
            .h(px(40.))
            .flex_none()
            .px(px(10.))
            .gap(px(2.))
            .border_b_1()
            .border_color(cx.theme().border);
        for (index, group) in TOOLBAR.iter().enumerate() {
            if index > 0 {
                toolbar =
                    toolbar.child(div().w(px(1.)).h(px(16.)).mx(px(6.)).bg(cx.theme().border));
            }
            for &format in group.iter() {
                toolbar = toolbar.child(
                    Button::new(format.id())
                        .ghost()
                        .size(px(28.))
                        .icon(Icon::new(format.icon()).size(px(15.)))
                        .accessibility_label(format.label())
                        .tooltip(format.label())
                        .disabled(editor.publishing)
                        .on_click(cx.listener(move |this, _, window, cx| {
                            this.format_block(format, window, cx)
                        })),
                );
            }
        }
        toolbar = toolbar.child(div().flex_1()).child(
            div()
                .text_size(px(11.5))
                .text_color(muted)
                .child("Markdown formatting"),
        );
        let pane_title = |title: &'static str| {
            div()
                .px(px(16.))
                .pt(px(10.))
                .pb(px(4.))
                .text_size(px(12.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(muted)
                .child(title)
        };
        let write = v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .border_r_1()
            .border_color(cx.theme().border)
            .child(pane_title("Write"))
            .child(
                div()
                    .id("block-editor-source")
                    .flex_1()
                    .min_h_0()
                    .px(px(16.))
                    .pb(px(12.))
                    .overflow_y_scroll()
                    .font_family(theme::message_font(cx))
                    .child(
                        Textarea::new(&editor.input)
                            .accessibility_id("block-editor")
                            .appearance(false)
                            .w_full()
                            .disabled(editor.publishing)
                            .aria_label("Block text"),
                    ),
            );
        let preview_pane = v_flex()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .bg(palette.chrome)
            .child(pane_title("Preview"))
            .child(
                div()
                    .id("block-editor-preview-scroll")
                    .flex_1()
                    .min_h_0()
                    .px(px(16.))
                    .pb(px(16.))
                    .overflow_y_scroll()
                    .child(match preview {
                        Some(content) => {
                            let byline = self.block_byline(
                                &self.snapshot.user_id,
                                self.own_avatar.as_deref(),
                                "You",
                                "Not published yet".into(),
                                false,
                                cx,
                            );
                            self.block_frame("block-editor-card".into(), content, byline, cx)
                        }
                        None => div()
                            .pt(px(24.))
                            .text_center()
                            .text_color(muted)
                            .child(
                                problem
                                    .unwrap_or("Your block appears here as members will see it."),
                            )
                            .into_any_element(),
                    }),
            );
        let footer = h_flex()
            .h(px(52.))
            .flex_none()
            .px(px(16.))
            .gap(px(8.))
            .border_t_1()
            .border_color(cx.theme().border)
            .child(
                div()
                    .id("block-editor-status")
                    .role(Role::Status)
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(px(12.))
                    .text_color(if editor.status.is_empty() {
                        muted
                    } else {
                        palette.error
                    })
                    .child(if !editor.status.is_empty() {
                        editor.status.clone()
                    } else if self.preview {
                        "Sample data · Publishing adds the block on this device only.".into()
                    } else {
                        "Blocks are sent as formatted messages, readable in any Matrix app.".into()
                    }),
            )
            .child(
                Button::new("block-editor-cancel")
                    .outline()
                    .h(px(28.))
                    .px(px(12.))
                    .label("Cancel")
                    .on_click(
                        cx.listener(|this, _, window, cx| this.close_block_editor(window, cx)),
                    ),
            )
            .child(
                Button::new("block-editor-publish")
                    .primary()
                    .h(px(28.))
                    .px(px(12.))
                    .label(if editor.publishing {
                        "Publishing…"
                    } else {
                        "Publish Block"
                    })
                    .disabled(empty || problem.is_some() || editor.publishing)
                    .on_click(cx.listener(|this, _, window, cx| this.publish_block(window, cx))),
            );
        Some(
            v_flex()
                .id("block-editor")
                .size_full()
                .rounded(px(12.))
                .overflow_hidden()
                .border_1()
                .border_color(cx.theme().border)
                .bg(cx.theme().background)
                .shadow_lg()
                .on_click(|_, _, cx| cx.stop_propagation())
                .child(
                    h_flex()
                        .h(px(48.))
                        .flex_none()
                        .px(px(16.))
                        .gap(px(8.))
                        .border_b_1()
                        .border_color(cx.theme().border)
                        .child(
                            Icon::new(IconName::ScrollText)
                                .size(px(15.))
                                .text_color(muted),
                        )
                        .child(
                            div()
                                .text_size(px(14.))
                                .font_weight(FontWeight::SEMIBOLD)
                                .child("New Block"),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_color(muted)
                                .child(format!("in {room_name}")),
                        )
                        .child(
                            Button::new("block-editor-close")
                                .ghost()
                                .size(px(26.))
                                .icon(IconName::Close)
                                .accessibility_label("Close")
                                .tooltip("Close  Esc")
                                .on_click(cx.listener(|this, _, window, cx| {
                                    this.close_block_editor(window, cx)
                                })),
                        ),
                )
                .child(toolbar)
                .child(
                    h_flex()
                        .flex_1()
                        .min_h_0()
                        .items_stretch()
                        .child(write)
                        .child(preview_pane),
                )
                .child(footer)
                .into_any_element(),
        )
    }
}

#[cfg(test)]
mod tests {
    // Not a glob: the UI prelude exports GPUI's `test` attribute.
    use super::{Format, apply_format};
    use crate::model;

    fn run(text: &str, selection: std::ops::Range<usize>, format: Format) -> (String, String) {
        let (range, replacement, select) = apply_format(text, selection, format);
        let mut out = text.to_owned();
        out.replace_range(range, &replacement);
        let selected = out[select].to_owned();
        (out, selected)
    }

    #[test]
    fn inline_formats_wrap_the_selection_or_a_placeholder() {
        assert_eq!(
            run("Be kind", 3..7, Format::Bold),
            ("Be **kind**".into(), "kind".into())
        );
        assert_eq!(
            run("Hi ", 3..3, Format::Italic),
            ("Hi _italic text_".into(), "italic text".into())
        );
        assert_eq!(
            run("docs", 0..4, Format::Link),
            ("[docs](https://)".into(), "https://".into())
        );
    }

    #[test]
    fn line_formats_prefix_every_touched_line() {
        let (out, _) = run("One\nTwo\nThree", 1..6, Format::Numbers);
        assert_eq!(out, "1. One\n2. Two\nThree");
        let (out, _) = run("Intro\n# Old", 8..8, Format::Heading);
        assert_eq!(out, "Intro\n## Old");
    }

    #[test]
    fn inserted_blocks_start_on_their_own_paragraph() {
        // A divider straight after text would make that text a heading.
        let (out, _) = run("Rules", 5..5, Format::Divider);
        assert_eq!(out, "Rules\n\n---\n\n");
        let (out, selected) = run("", 0..0, Format::Table);
        assert!(out.starts_with("| Column | Column |\n| --- | --- |"));
        assert_eq!(selected, "Column");
        // The template renders as a real table once sent.
        let rich = twayblade_core::rich_text::compose_preview(&out)
            .unwrap()
            .expect("formatted");
        assert!(model::rich_markdown(&rich).contains("| Column"));
    }
}
