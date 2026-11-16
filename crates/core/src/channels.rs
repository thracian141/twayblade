//! Persistent channel kind; signalling/media remain native MatrixRTC.
use matrix_sdk::ruma::room::RoomType;

pub const VOICE_ROOM_TYPE: &str = "com.twayblade.voice";

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChannelKind {
    #[default]
    Text,
    Voice,
}

impl ChannelKind {
    pub fn from_room_type(room_type: Option<&RoomType>) -> Self {
        if room_type.is_some_and(|kind| kind.as_str() == VOICE_ROOM_TYPE) {
            Self::Voice
        } else {
            Self::Text
        }
    }
    pub fn of(room: &matrix_sdk::Room) -> Self {
        Self::from_room_type(room.room_type().as_ref())
    }
}

impl crate::auth::Account {
    pub fn is_voice_channel(&self, room: &str) -> bool {
        matrix_sdk::ruma::RoomId::parse(room)
            .ok()
            .and_then(|id| self.client.get_room(&id))
            .is_some_and(|room| ChannelKind::of(&room) == ChannelKind::Voice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_our_persistent_voice_room_type_disables_text() {
        assert_eq!(ChannelKind::from_room_type(None), ChannelKind::Text);
        assert_eq!(
            ChannelKind::from_room_type(Some(&RoomType::Space)),
            ChannelKind::Text
        );
        assert_eq!(
            ChannelKind::from_room_type(Some(&RoomType::from(VOICE_ROOM_TYPE))),
            ChannelKind::Voice
        );
        assert_eq!(
            ChannelKind::from_room_type(Some(&RoomType::from("other.client.room"))),
            ChannelKind::Text
        );
    }
}
