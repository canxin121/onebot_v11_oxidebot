use std::{ops::Deref, sync::Arc, time::Duration};

use anyhow::Result;
use chrono::DateTime;
use onebot_v11::event::notice::{GroupFileUploadEvent, GroupMemberHonorChangeEvent};
use oxidebot::{
    event::{
        any::{AnyEvent, AnyEventDataTrait},
        notice::{
            GroupAdminChangeEvent, GroupAdminChangeType, GroupHightLightChangeEvent,
            GroupHightLightChangeType, GroupMemberAliasChangeEvent, GroupMemberDecreaseEvent,
            GroupMemberIncreseEvent, GroupMemberMuteChangeEvent, MessageDeletedEvent,
        },
        request::{FriendAddEvent, GroupAddEvent, GroupInviteEvent},
        Event, MessageEvent,
    },
    source::{
        group::Group,
        message::Message,
        user::{Role, Sex, User, UserGroupInfo, UserProfile},
    },
    EventTrait,
};

use crate::{segment::cast_segment, PLATFORM};

pub struct EventWrapper(pub Arc<onebot_v11::Event>);

impl EventTrait for EventWrapper {
    fn into_event(&self) -> Result<oxidebot::event::Event> {
        match self.0.as_ref() {
            onebot_v11::Event::Message(event) => match event {
                onebot_v11::event::message::Message::PrivateMessage(event) => {
                    Ok(Event::MessageEvent(MessageEvent {
                        id: event.message_id.to_string(),
                        platform: PLATFORM,
                        time: DateTime::from_timestamp(event.time, 0),
                        sender: User {
                            id: event.user_id.to_string(),
                            profile: Some(UserProfile {
                                nickname: Some(
                                    event
                                        .sender
                                        .nickname
                                        .clone()
                                        .unwrap_or(String::with_capacity(0)),
                                ),
                                sex: {
                                    if let Some(sex) = &event.sender.sex {
                                        Some(Sex::from(sex.as_str()))
                                    } else {
                                        None
                                    }
                                },
                                avatar: None,
                                email: None,
                                phone: None,
                                signature: None,
                                level: None,
                                age: {
                                    if let Some(age) = &event.sender.age {
                                        if *age > u8::MAX as i32 {
                                            Some(u8::MAX)
                                        } else {
                                            Some((*age) as u8)
                                        }
                                    } else {
                                        None
                                    }
                                },
                            }),
                            group_info: None,
                        },
                        group: None,
                        message: Message {
                            id: event.message_id.to_string(),
                            segments: event
                                .message
                                .clone()
                                .into_iter()
                                .map(|segment| cast_segment(segment))
                                .collect(),
                        },
                    }))
                }
                onebot_v11::event::message::Message::GroupMessage(event) => {
                    Ok(Event::MessageEvent(MessageEvent {
                        id: event.message_id.to_string(),
                        platform: PLATFORM,
                        time: DateTime::from_timestamp(event.time, 0),
                        sender: User {
                            id: event.user_id.to_string(),
                            profile: Some(UserProfile {
                                nickname: Some(
                                    event
                                        .sender
                                        .nickname
                                        .clone()
                                        .unwrap_or(String::with_capacity(0)),
                                ),
                                sex: {
                                    if let Some(sex) = &event.sender.sex {
                                        Some(Sex::from(sex.as_str()))
                                    } else {
                                        None
                                    }
                                },
                                avatar: None,
                                email: None,
                                phone: None,
                                signature: None,
                                level: {
                                    if let Some(level) = &event.sender.level {
                                        Some(level.to_string())
                                    } else {
                                        None
                                    }
                                },
                                age: {
                                    if let Some(age) = &event.sender.age {
                                        if *age > u8::MAX as i32 {
                                            Some(u8::MAX)
                                        } else {
                                            Some((*age) as u8)
                                        }
                                    } else {
                                        None
                                    }
                                },
                            }),
                            group_info: Some(UserGroupInfo {
                                title: event.sender.card.clone(),
                                role: if let Some(role) = &event.sender.role {
                                    if role == "owner" {
                                        Some(Role::Owner)
                                    } else if role == "admin" {
                                        Some(Role::Admin)
                                    } else if role == "member" {
                                        Some(Role::Member)
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                },
                                join_time: None,
                                last_active_time: None,
                                level: event.sender.level.clone(),
                            }),
                        },
                        group: Some(Group {
                            id: event.group_id.to_string(),
                            profile: None,
                        }),

                        message: Message {
                            id: event.message_id.to_string(),
                            segments: event
                                .message
                                .clone()
                                .into_iter()
                                .map(|segment| cast_segment(segment))
                                .collect(),
                        },
                    }))
                }
            },
            onebot_v11::Event::Meta(event) => match event {
                onebot_v11::event::meta::Meta::Lifecycle(event) => {
                    if event.sub_type == "connect" || event.sub_type == "enable" {
                        Ok(Event::MetaEvent(oxidebot::event::MetaEvent::ConnectEvent))
                    } else if event.sub_type == "disable" {
                        Ok(Event::MetaEvent(
                            oxidebot::event::MetaEvent::DisconnectEvent,
                        ))
                    } else {
                        Err(anyhow::anyhow!("Unknown lifecycle event"))
                    }
                }
                onebot_v11::event::meta::Meta::Heartbeat(event) => Ok(Event::AnyEvent(AnyEvent {
                    server: PLATFORM,
                    r#type: "meta_event.heartbeat".to_string(),
                    data: Box::new(HeartbeatWrapper::from(event.clone())),
                })),
            },
            onebot_v11::Event::Notice(event) => match event {
                onebot_v11::event::notice::Notice::GroupFileUpload(event) => {
                    Ok(Event::AnyEvent(AnyEvent {
                        server: PLATFORM,
                        r#type: "notice.group_upload".to_string(),
                        data: Box::new(GroupFileUploadEventWrapper::from(event.clone())),
                    }))
                }
                onebot_v11::event::notice::Notice::GroupAdminChange(event) => Ok(
                    Event::NoticeEvent(oxidebot::event::NoticeEvent::GroupAdminChangeEvent(
                        GroupAdminChangeEvent {
                            group: Group {
                                id: event.group_id.to_string(),
                                profile: None,
                            },
                            user: User {
                                id: event.user_id.to_string(),
                                profile: None,
                                group_info: None,
                            },
                            r#type: {
                                if event.sub_type == "set" {
                                    GroupAdminChangeType::Set
                                } else if event.sub_type == "unset" {
                                    GroupAdminChangeType::Unset
                                } else {
                                    GroupAdminChangeType::Unknown
                                }
                            },
                        },
                    )),
                ),
                onebot_v11::event::notice::Notice::GroupMemberDecrease(event) => Ok(
                    Event::NoticeEvent(oxidebot::event::NoticeEvent::GroupMemberDecreaseEvent(
                        GroupMemberDecreaseEvent {
                            group: Group {
                                id: event.group_id.to_string(),
                                profile: None,
                            },
                            user: User {
                                id: event.user_id.to_string(),
                                profile: None,
                                group_info: None,
                            },

                            reason: {
                                if event.sub_type == "leave" {
                                    oxidebot::event::notice::GroupMemberDecreaseReason::Leave
                                } else if event.sub_type == "kick" {
                                    oxidebot::event::notice::GroupMemberDecreaseReason::Kick {
                                        operator: Some(User {
                                            id: event.operator_id.to_string(),
                                            profile: None,
                                            group_info: None,
                                        }),
                                    }
                                } else if event.sub_type == "kick_me" {
                                    oxidebot::event::notice::GroupMemberDecreaseReason::KickMe {
                                        operator: Some(User {
                                            id: event.operator_id.to_string(),
                                            profile: None,
                                            group_info: None,
                                        }),
                                    }
                                } else {
                                    oxidebot::event::notice::GroupMemberDecreaseReason::Unknown
                                }
                            },
                        },
                    )),
                ),
                onebot_v11::event::notice::Notice::GroupMemberIncrease(event) => Ok(
                    Event::NoticeEvent(oxidebot::event::NoticeEvent::GroupMemberIncreseEvent(
                        GroupMemberIncreseEvent {
                            group: Group {
                                id: event.group_id.to_string(),
                                profile: None,
                            },
                            user: User {
                                id: event.user_id.to_string(),
                                profile: None,
                                group_info: None,
                            },
                            reason: {
                                if event.sub_type == "approve" {
                                    oxidebot::event::notice::GroupMemberIncreseReason::Approve {
                                        operator: Some(User {
                                            id: event.operator_id.to_string(),
                                            profile: None,
                                            group_info: None,
                                        }),
                                    }
                                } else if event.sub_type == "invite" {
                                    oxidebot::event::notice::GroupMemberIncreseReason::Invite {
                                        inviter: None,
                                        operator: Some(User {
                                            id: event.operator_id.to_string(),
                                            profile: None,
                                            group_info: None,
                                        }),
                                    }
                                } else {
                                    oxidebot::event::notice::GroupMemberIncreseReason::Unknown
                                }
                            },
                        },
                    )),
                ),
                onebot_v11::event::notice::Notice::GroupBan(event) => Ok(Event::NoticeEvent(
                    oxidebot::event::NoticeEvent::GroupMemberMuteChangeEvent(
                        GroupMemberMuteChangeEvent {
                            group: Group {
                                id: event.group_id.to_string(),
                                profile: None,
                            },
                            user: User {
                                id: event.user_id.to_string(),
                                profile: None,
                                group_info: None,
                            },
                            operator: Some(User {
                                id: event.operator_id.to_string(),
                                profile: None,
                                group_info: None,
                            }),
                            r#type: {
                                if event.sub_type == "ban" {
                                    oxidebot::event::notice::MuteType::Mute {
                                        duration: Some(Duration::from_secs(event.duration)),
                                    }
                                } else if event.sub_type == "lift_ban" {
                                    oxidebot::event::notice::MuteType::UnMute
                                } else {
                                    oxidebot::event::notice::MuteType::Unknown
                                }
                            },
                        },
                    ),
                )),
                onebot_v11::event::notice::Notice::FriendAdd(event) => Ok(Event::RequestEvent(
                    oxidebot::event::RequestEvent::FriendAddEvent(FriendAddEvent {
                        user: User {
                            id: event.user_id.to_string(),
                            profile: None,
                            group_info: None,
                        },
                        id: event.user_id.to_string(),
                        message: None,
                    }),
                )),
                onebot_v11::event::notice::Notice::GroupMessageRecall(event) => {
                    Ok(Event::NoticeEvent(
                        oxidebot::event::NoticeEvent::MessageDeletedEvent(MessageDeletedEvent {
                            user: User {
                                id: event.user_id.to_string(),
                                profile: None,
                                group_info: None,
                            },
                            group: Some(Group {
                                id: event.group_id.to_string(),
                                profile: None,
                            }),
                            operator: Some(User {
                                id: event.operator_id.to_string(),
                                profile: None,
                                group_info: None,
                            }),
                            message: Message {
                                id: event.message_id.to_string(),
                                segments: Vec::with_capacity(0),
                            },
                        }),
                    ))
                }
                onebot_v11::event::notice::Notice::FriendMessageRecall(event) => {
                    Ok(Event::NoticeEvent(
                        oxidebot::event::NoticeEvent::MessageDeletedEvent(MessageDeletedEvent {
                            user: User {
                                id: event.user_id.to_string(),
                                profile: None,
                                group_info: None,
                            },
                            group: None,
                            operator: Some(User {
                                id: event.user_id.to_string(),
                                profile: None,
                                group_info: None,
                            }),
                            message: Message {
                                id: event.message_id.to_string(),
                                segments: Vec::with_capacity(0),
                            },
                        }),
                    ))
                }
                onebot_v11::event::notice::Notice::GroupPoke(event) => {
                    Ok(Event::AnyEvent(AnyEvent {
                        server: PLATFORM,
                        r#type: "notice.notify.poke".to_string(),
                        data: Box::new(GroupPokeEventWrapper::from(event.clone())),
                    }))
                }
                onebot_v11::event::notice::Notice::GroupLuckyKing(event) => {
                    Ok(Event::AnyEvent(AnyEvent {
                        server: PLATFORM,
                        r#type: "notice.notify.lucky_king".to_string(),
                        data: Box::new(GroupLuckyKingEventWrapper::from(event.clone())),
                    }))
                }
                onebot_v11::event::notice::Notice::GroupMemberHonorChange(event) => {
                    Ok(Event::AnyEvent(AnyEvent {
                        server: PLATFORM,
                        r#type: "notice.notify.honor".to_string(),
                        data: Box::new(GroupMemberHonorChangeEventWrapper::from(event.clone())),
                    }))
                }
                onebot_v11::event::notice::Notice::FriendInputStatusChange(event) => {
                    Ok(Event::AnyEvent(AnyEvent {
                        server: PLATFORM,
                        r#type: "notice.notify.input_status".to_string(),
                        data: Box::new(FriendInputStatusChangeEventWrapper::from(event.clone())),
                    }))
                }
                onebot_v11::event::notice::Notice::GroupEssenceMessageChange(event) => Ok(
                    Event::NoticeEvent(oxidebot::event::NoticeEvent::GroupHightLightChangeEvent(
                        GroupHightLightChangeEvent {
                            group: Group {
                                id: event.group_id.to_string(),
                                profile: None,
                            },
                            r#type: {
                                match event.sub_type {
                                    onebot_v11::event::notice::EssenseMessageChangeType::Add => {
                                        GroupHightLightChangeType::Set
                                    }
                                    onebot_v11::event::notice::EssenseMessageChangeType::Delete => {
                                        GroupHightLightChangeType::Unset
                                    }
                                }
                            },
                            message: Message {
                                id: event.message_id.to_string(),
                                segments: Vec::with_capacity(0),
                            },
                            sender: Some(User {
                                id: event.sender_id.to_string(),
                                profile: None,
                                group_info: None,
                            }),
                            operator: Some(User {
                                id: event.user_id.to_string(),
                                profile: None,
                                group_info: None,
                            }),
                        },
                    )),
                ),
                onebot_v11::event::notice::Notice::GroupCardChange(event) => Ok(
                    Event::NoticeEvent(oxidebot::event::NoticeEvent::GroupMemberAliasChangeEvent(
                        GroupMemberAliasChangeEvent {
                            group: Group {
                                id: event.group_id.to_string(),
                                profile: None,
                            },
                            user: User {
                                id: event.user_id.to_string(),
                                profile: None,
                                group_info: None,
                            },
                            operator: None,
                            old_alias: Some(event.card_old.clone()),
                            new_alias: Some(event.card_new.clone()),
                        },
                    )),
                ),
            },
            onebot_v11::Event::Request(event) => match event {
                onebot_v11::event::request::Request::FriendRequestEvent(event) => {
                    Ok(Event::RequestEvent(
                        oxidebot::event::RequestEvent::FriendAddEvent(FriendAddEvent {
                            id: event.flag.to_string(),
                            user: User {
                                id: event.user_id.to_string(),
                                profile: None,
                                group_info: None,
                            },
                            message: Some(event.comment.clone()),
                        }),
                    ))
                }
                onebot_v11::event::request::Request::GroupRequestEvent(event) => {
                    if event.sub_type == "add" {
                        Ok(Event::RequestEvent(
                            oxidebot::event::RequestEvent::GroupAddEvent(GroupAddEvent {
                                id: event.flag.clone(),
                                user: User {
                                    id: event.user_id.to_string(),
                                    profile: None,
                                    group_info: None,
                                },
                                group: Group {
                                    id: event.group_id.to_string(),
                                    profile: None,
                                },
                                message: Some(event.comment.clone()),
                            }),
                        ))
                    } else if event.sub_type == "invite" {
                        Ok(Event::RequestEvent(
                            oxidebot::event::RequestEvent::GroupInviteEvent(GroupInviteEvent {
                                id: event.flag.clone(),
                                user: User {
                                    id: event.user_id.to_string(),
                                    profile: None,
                                    group_info: None,
                                },
                                group_id: event.group_id.to_string(),
                                message: Some(event.comment.clone()),
                            }),
                        ))
                    } else {
                        Err(anyhow::anyhow!("Unknown group request event"))
                    }
                }
            },
            onebot_v11::Event::ApiRespBuilder(_) => Err(anyhow::anyhow!(
                "ApiRespBuilder event can't be converted to Event"
            )),
        }
    }

    fn clone_box(&self) -> oxidebot::event::EventObject {
        Box::new(EventWrapper(Arc::clone(&self.0)))
    }

    fn server(&self) -> &'static str {
        PLATFORM
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

pub struct HeartbeatWrapper(pub onebot_v11::event::meta::Heartbeat);
impl Deref for HeartbeatWrapper {
    type Target = onebot_v11::event::meta::Heartbeat;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<onebot_v11::event::meta::Heartbeat> for HeartbeatWrapper {
    fn from(value: onebot_v11::event::meta::Heartbeat) -> Self {
        HeartbeatWrapper(value)
    }
}

impl AnyEventDataTrait for HeartbeatWrapper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn AnyEventDataTrait> {
        Box::new(HeartbeatWrapper(self.0.clone()))
    }
}

pub struct GroupFileUploadEventWrapper(pub GroupFileUploadEvent);
impl Deref for GroupFileUploadEventWrapper {
    type Target = GroupFileUploadEvent;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<GroupFileUploadEvent> for GroupFileUploadEventWrapper {
    fn from(value: GroupFileUploadEvent) -> Self {
        GroupFileUploadEventWrapper(value)
    }
}

impl AnyEventDataTrait for GroupFileUploadEventWrapper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn AnyEventDataTrait> {
        Box::new(GroupFileUploadEventWrapper(self.0.clone()))
    }
}
pub struct GroupPokeEventWrapper(onebot_v11::event::notice::GroupPokeEvent);

impl Deref for GroupPokeEventWrapper {
    type Target = onebot_v11::event::notice::GroupPokeEvent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<onebot_v11::event::notice::GroupPokeEvent> for GroupPokeEventWrapper {
    fn from(value: onebot_v11::event::notice::GroupPokeEvent) -> Self {
        GroupPokeEventWrapper(value)
    }
}

impl AnyEventDataTrait for GroupPokeEventWrapper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn AnyEventDataTrait> {
        Box::new(GroupPokeEventWrapper(self.0.clone()))
    }
}

pub struct GroupMemberHonorChangeEventWrapper(GroupMemberHonorChangeEvent);

impl Deref for GroupMemberHonorChangeEventWrapper {
    type Target = GroupMemberHonorChangeEvent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<GroupMemberHonorChangeEvent> for GroupMemberHonorChangeEventWrapper {
    fn from(value: GroupMemberHonorChangeEvent) -> Self {
        GroupMemberHonorChangeEventWrapper(value)
    }
}

impl AnyEventDataTrait for GroupMemberHonorChangeEventWrapper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn AnyEventDataTrait> {
        Box::new(GroupMemberHonorChangeEventWrapper(self.0.clone()))
    }
}

pub struct GroupLuckyKingEventWrapper(onebot_v11::event::notice::GroupLuckyKingEvent);

impl Deref for GroupLuckyKingEventWrapper {
    type Target = onebot_v11::event::notice::GroupLuckyKingEvent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<onebot_v11::event::notice::GroupLuckyKingEvent> for GroupLuckyKingEventWrapper {
    fn from(value: onebot_v11::event::notice::GroupLuckyKingEvent) -> Self {
        GroupLuckyKingEventWrapper(value)
    }
}

impl AnyEventDataTrait for GroupLuckyKingEventWrapper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn AnyEventDataTrait> {
        Box::new(GroupLuckyKingEventWrapper(self.0.clone()))
    }
}

pub struct FriendInputStatusChangeEventWrapper(
    onebot_v11::event::notice::FriendInputStatusChangeEvent,
);

impl Deref for FriendInputStatusChangeEventWrapper {
    type Target = onebot_v11::event::notice::FriendInputStatusChangeEvent;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl From<onebot_v11::event::notice::FriendInputStatusChangeEvent>
    for FriendInputStatusChangeEventWrapper
{
    fn from(value: onebot_v11::event::notice::FriendInputStatusChangeEvent) -> Self {
        FriendInputStatusChangeEventWrapper(value)
    }
}

impl AnyEventDataTrait for FriendInputStatusChangeEventWrapper {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn clone_box(&self) -> Box<dyn AnyEventDataTrait> {
        Box::new(FriendInputStatusChangeEventWrapper(self.0.clone()))
    }
}
