use anyhow::Result;

use chrono::DateTime;
use onebot_v11::api::{
    payload::{
        ApiPayload, DeleteMsg, GetFile, GetGroupFileCount, GetGroupFileList, GetGroupMemberList,
        GetStrangerInfo, SetFriendAddRequest, SetGroupAddRequest, SetGroupAdmin, SetGroupBan,
        SetGroupCard, SetGroupFileFolder, SetGroupKick, SetGroupName, SetGroupWholeBan,
        SetMsgEmojiLike, SetQQAvatar,
    },
    resp::{SendGroupMsgResponse, SendPrivateMsgResponse},
};
use onebot_v11::connect::ws_reverse::{ReverseWsConfig, ReverseWsConnect};
use oxidebot::{
    api::{
        payload::{GroupAdminChangeType, GroupMuteType, RequestResponse, SendMessageTarget},
        BotGetFriendListResponse, BotGetGroupListResponse, BotGetProfileResponse, CallApiTrait,
        GetMessageDetailResponse, GroupGetFileCountResponse, GroupGetFsListResponse,
        GroupGetProfileResponse, GroupMemberListResponse, SendMessageResponse,
        UserGetProfileResponse,
    },
    bot::BotObject,
    matcher::Matcher,
    source::{
        bot::BotInfo,
        group::GroupProfile,
        message::{File, Folder, FsNode, MessageSegment},
        user::{Sex, User, UserGroupInfo, UserProfile},
    },
    BotTrait,
};
use std::{any::Any, sync::Arc};
use tokio::sync::broadcast;
use tracing::warn;

use std::time::Duration;

use crate::{
    event::EventWrapper,
    segment::{cast_segment, parse_segment},
    PLATFORM,
};

#[derive(Clone)]
pub struct OnebotV11ReverseWsBot {
    connect: Arc<ReverseWsConnect>,
}

impl BotTrait for OnebotV11ReverseWsBot {
    fn bot_info<'life0, 'async_trait>(
        &'life0 self,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = BotInfo> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let bot_id = self.connect.bot_id.read().await;
            let bot_id = bot_id.as_ref().and_then(|s| Some(s.to_string()));
            BotInfo {
                id: bot_id,
                nickname: None,
            }
        })
    }
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn start_sending_events<'life0, 'async_trait>(
        &'life0 self,
        sender: broadcast::Sender<Matcher>,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = ()> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        Box::pin(async move {
            let mut subscriber = self.connect.subscribe().await;
            while let Ok(event) = subscriber.recv().await {
                for matcher in Matcher::new(
                    Box::new(EventWrapper(Arc::new(event))),
                    <Self as BotTrait>::clone_box(self),
                ) {
                    match sender.send(matcher) {
                        Ok(_) => {}
                        Err(e) => {
                            tracing::error!("Onebotv11: Failed to send event: {:?}", e);
                        }
                    }
                }
            }
        })
    }

    fn clone_box(&self) -> BotObject {
        Box::new(self.clone())
    }

    fn server(&self) -> &'static str {
        PLATFORM
    }

    fn as_any(&self) -> &dyn Any {
        self
    }
}

impl OnebotV11ReverseWsBot {
    pub async fn new(config: ReverseWsConfig) -> BotObject {
        let connect = ReverseWsConnect::new(config).await.unwrap();
        Box::new(Self { connect })
    }
    pub async fn call_api(
        &self,
        payload: onebot_v11::api::payload::ApiPayload,
    ) -> Result<onebot_v11::api::resp::ApiResp> {
        self.connect.clone().call_api(payload).await
    }
}

impl CallApiTrait for OnebotV11ReverseWsBot {
    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn send_message<'life0, 'async_trait>(
        &'life0 self,
        message: Vec<MessageSegment>,
        target: SendMessageTarget,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<Vec<SendMessageResponse>>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload: Result<ApiPayload> = match target {
            oxidebot::api::payload::SendMessageTarget::Group(id) => {
                Ok(onebot_v11::api::payload::ApiPayload::SendGroupMsg(
                    onebot_v11::api::payload::SendGroupMsg {
                        group_id: id.parse().unwrap_or_else(|e| {
                            tracing::error!(
                                "Onebotv11: Failed to parse group id: {}, error: {}",
                                id,
                                e
                            );
                            Default::default()
                        }),
                        message: message.into_iter().map(parse_segment).collect(),
                        auto_escape: true,
                    },
                ))
            }
            oxidebot::api::payload::SendMessageTarget::Private(id) => {
                Ok(onebot_v11::api::payload::ApiPayload::SendPrivateMsg(
                    onebot_v11::api::payload::SendPrivateMsg {
                        user_id: id.parse().unwrap_or_else(|e| {
                            tracing::error!(
                                "Onebotv11: Failed to parse user id: {}, error: {}",
                                id,
                                e
                            );
                            Default::default()
                        }),
                        message: message.into_iter().map(parse_segment).collect(),
                        auto_escape: true,
                    },
                ))
            }
        };
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload?).await?;
            if resp.status == "ok" {
                match resp.data {
                    onebot_v11::api::resp::ApiRespData::SendPrivateMsgResponse(
                        SendPrivateMsgResponse { message_id },
                    )
                    | onebot_v11::api::resp::ApiRespData::SendGroupMsgResponse(
                        SendGroupMsgResponse { message_id },
                    ) => Ok(vec![SendMessageResponse {
                        sent_message_id: message_id.to_string(),
                    }]),
                    _ => Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to send message, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn delete_message<'life0, 'async_trait>(
        &'life0 self,
        message_id: String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::DeleteMsg(DeleteMsg {
            message_id: message_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse message id: {}, error: {}",
                    message_id,
                    e
                );
                Default::default()
            }),
        });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to delete message, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_message_detail<'life0, 'async_trait>(
        &'life0 self,
        message_id: String,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<GetMessageDetailResponse>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload =
            onebot_v11::api::payload::ApiPayload::GetMsg(onebot_v11::api::payload::GetMsg {
                message_id: message_id.parse().unwrap_or_else(|e| {
                    tracing::error!(
                        "Onebotv11: Failed to parse message id: {}, error: {}",
                        message_id,
                        e
                    );
                    Default::default()
                }),
            });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                match resp.data {
                    onebot_v11::api::resp::ApiRespData::GetMsgResponse(resp) => {
                        Ok(GetMessageDetailResponse {
                            message: resp.message.into_iter().map(cast_segment).collect(),
                            sender: Some(User {
                                id: resp.sender.user_id.unwrap_or_default().to_string(),
                                profile: Some(UserProfile {
                                    nickname: resp.sender.nickname,
                                    sex: Some(Sex::from(
                                        resp.sender.sex.unwrap_or_default().as_str(),
                                    )),
                                    age: resp.sender.age.and_then(|a| Some(a as u64)),
                                    avatar: None,
                                    email: None,
                                    phone: None,
                                    signature: None,
                                    level: None,
                                }),
                                group_info: None,
                            }),
                            time: DateTime::from_timestamp(resp.time, 0),
                        })
                    }
                    _ => Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to get message detail, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn set_message_reaction<'life0, 'async_trait>(
        &'life0 self,
        message_id: String,
        reaction_id: String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::SetMsgEmojiLike(SetMsgEmojiLike {
            message_id: message_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse message id: {}, error: {}",
                    message_id,
                    e
                );
                Default::default()
            }),
            emoji_id: reaction_id,
        });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to set message reaction, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_group_member_list<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<GroupMemberListResponse>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload =
            onebot_v11::api::payload::ApiPayload::GetGroupMemberList(GetGroupMemberList {
                group_id: group_id.parse().unwrap_or_else(|e| {
                    tracing::error!(
                        "Onebotv11: Failed to parse group id: {}, error: {}",
                        group_id,
                        e
                    );
                    Default::default()
                }),
            });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                match resp.data {
                    onebot_v11::api::resp::ApiRespData::GetGroupMemberListResponse(resp) => {
                        Ok(GroupMemberListResponse {
                            members: resp
                                .into_iter()
                                .map(|m| User {
                                    id: m.user_id.to_string(),
                                    profile: Some(UserProfile {
                                        nickname: Some(m.nickname),
                                        sex: Some(Sex::from(m.sex.as_str())),
                                        age: Some(m.age as u64),
                                        avatar: None,
                                        email: None,
                                        phone: None,
                                        signature: None,
                                        level: None,
                                    }),
                                    group_info: Some(UserGroupInfo {
                                        role: Some(match m.role.to_lowercase().as_str() {
                                            "owner" => oxidebot::source::user::Role::Owner,
                                            "admin" => oxidebot::source::user::Role::Admin,
                                            "membet" => oxidebot::source::user::Role::Member,
                                            _ => oxidebot::source::user::Role::Unknown,
                                        }),
                                        join_time: DateTime::from_timestamp(m.join_time, 0),
                                        last_active_time: DateTime::from_timestamp(
                                            m.last_sent_time,
                                            0,
                                        ),
                                        level: Some(m.level),
                                        alias: Some(m.title),
                                    }),
                                })
                                .collect(),
                        })
                    }
                    _ => Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to get group member list, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn kick_group_member<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        user_id: String,
        reject_add_request: Option<bool>,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = SetGroupKick {
            group_id: group_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse group id: {}, error: {}",
                    group_id,
                    e
                );
                Default::default()
            }),
            user_id: user_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse user id: {}, error: {}",
                    user_id,
                    e
                );
                Default::default()
            }),
            reject_add_request: reject_add_request.unwrap_or(false),
        };
        Box::pin(async move {
            let resp = self
                .connect
                .clone()
                .call_api(onebot_v11::api::payload::ApiPayload::SetGroupKick(payload))
                .await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to kick group member, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn mute_group<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        duration: Option<Duration>,
        r#type: GroupMuteType,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        if duration.is_some() {
            warn!("Onebotv11: Mute Group duration is not supported");
        }
        let payload = SetGroupWholeBan {
            group_id: group_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse group id: {}, error: {}",
                    group_id,
                    e
                );
                Default::default()
            }),
            enable: match r#type {
                oxidebot::api::payload::GroupMuteType::Mute => true,
                oxidebot::api::payload::GroupMuteType::Unmute => false,
            },
        };
        Box::pin(async move {
            let resp = self
                .connect
                .clone()
                .call_api(onebot_v11::api::payload::ApiPayload::SetGroupWholeBan(
                    payload,
                ))
                .await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to mute group, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn mute_group_member<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        user_id: String,
        r#type: GroupMuteType,
        duration: Option<Duration>,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = SetGroupBan {
            group_id: group_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse group id: {}, error: {}",
                    group_id,
                    e
                );
                Default::default()
            }),
            user_id: user_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse user id: {}, error: {}",
                    user_id,
                    e
                );
                Default::default()
            }),
            duration: match r#type {
                GroupMuteType::Mute => duration.unwrap_or(Duration::from_secs(60)).as_secs() as i64,
                GroupMuteType::Unmute => 0,
            },
        };
        Box::pin(async move {
            let resp = self
                .connect
                .clone()
                .call_api(onebot_v11::api::payload::ApiPayload::SetGroupBan(payload))
                .await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to mute group member, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn change_group_admin<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        user_id: String,
        r#type: GroupAdminChangeType,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = SetGroupAdmin {
            group_id: group_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse group id: {}, error: {}",
                    group_id,
                    e
                );
                Default::default()
            }),
            user_id: user_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse user id: {}, error: {}",
                    user_id,
                    e
                );
                Default::default()
            }),
            enable: match r#type {
                oxidebot::api::payload::GroupAdminChangeType::Set => true,
                oxidebot::api::payload::GroupAdminChangeType::Unset => false,
            },
        };
        Box::pin(async move {
            let resp = self
                .connect
                .clone()
                .call_api(onebot_v11::api::payload::ApiPayload::SetGroupAdmin(payload))
                .await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to change group admin, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn set_group_member_alias<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        user_id: String,
        new_alias: String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::SetGroupCard(SetGroupCard {
            group_id: group_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse group id: {}, error: {}",
                    group_id,
                    e
                );
                Default::default()
            }),
            user_id: user_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse user id: {}, error: {}",
                    user_id,
                    e
                );
                Default::default()
            }),
            card: new_alias,
        });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to set group member alias, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_group_profile<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<GroupGetProfileResponse>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::GetGroupInfo(
            onebot_v11::api::payload::GetGroupInfo {
                group_id: group_id.parse().unwrap_or_else(|e| {
                    tracing::error!(
                        "Onebotv11: Failed to parse group id: {}, error: {}",
                        group_id,
                        e
                    );
                    Default::default()
                }),
                no_cache: true,
            },
        );
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                match resp.data {
                    onebot_v11::api::resp::ApiRespData::GetGroupInfoResponse(resp) => {
                        Ok(GroupGetProfileResponse {
                            profile: oxidebot::source::group::GroupProfile {
                                name: Some(resp.group_name),
                                avatar: None,
                                member_count: {
                                    if resp.member_count > u64::MAX as i64 {
                                        Some(u64::MAX)
                                    } else if resp.member_count < u64::MIN as i64 {
                                        Some(u64::MIN)
                                    } else {
                                        Some(resp.member_count as u64)
                                    }
                                },
                            },
                        })
                    }
                    _ => Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to get group profile, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn set_group_profile<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        new_profile: GroupProfile,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        if new_profile.avatar.is_some() {
            tracing::warn!("Onebotv11: Set Group avatar is not supported");
        }
        let payload = {
            if let Some(new_name) = new_profile.name {
                Some(onebot_v11::api::payload::ApiPayload::SetGroupName(
                    SetGroupName {
                        group_id: group_id.parse().unwrap_or_else(|e| {
                            tracing::error!(
                                "Onebotv11: Failed to parse group id: {}, error: {}",
                                group_id,
                                e
                            );
                            Default::default()
                        }),
                        group_name: new_name,
                    },
                ))
            } else {
                None
            }
        };
        Box::pin(async move {
            let payload = payload.ok_or(anyhow::anyhow!("Onebotv11: No new name to set"))?;
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to set group profile, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_group_file_count<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        parent_folder_id: Option<String>,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<GroupGetFileCountResponse>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        if parent_folder_id.is_some() {
            tracing::error!("Onebotv11: Get Group File Parent Fold ID is not supported");
        }
        let payload = onebot_v11::api::payload::ApiPayload::GetGroupFileCount(GetGroupFileCount {
            group_id: group_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse group id: {}, error: {}",
                    group_id,
                    e
                );
                Default::default()
            }),
        });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                match resp.data {
                    onebot_v11::api::resp::ApiRespData::GetGroupFileCountResponse(resp) => {
                        Ok(GroupGetFileCountResponse { count: resp.count })
                    }
                    _ => Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to get group file count, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_group_fs_list<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        start_index: u64,
        count: u64,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<GroupGetFsListResponse>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::GetGroupFileList(GetGroupFileList {
            group_id: group_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse group id: {}, error: {}",
                    group_id,
                    e
                );
                Default::default()
            }),
            start_index: {
                if start_index > i64::MAX as u64 {
                    tracing::error!("Onebotv11: Start index is too large: {}", start_index);
                    i64::MAX
                } else {
                    start_index as i64
                }
            },
            file_count: {
                if count > i64::MAX as u64 {
                    tracing::error!("Onebotv11: Count is too large: {}", count);
                    i64::MAX
                } else {
                    count as i64
                }
            },
        });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                match resp.data{
                    onebot_v11::api::resp::ApiRespData::GetGroupFileListResponse(resp)=>{
                        Ok(GroupGetFsListResponse {
                            fs_tree: resp
                                .file_list
                                .into_iter()
                                .map(|f| {
                                    if let Some(folder_info) = f.folder_info {
                                        FsNode::Folder(Folder {
                                            id: folder_info.folder_id,
                                            name: folder_info.folder_name,
                                            file_amount: {
                                                if folder_info.total_file_count > u64::MAX as i64 {
                                                    u64::MAX
                                                } else if folder_info.total_file_count < u64::MIN as i64 {
                                                    u64::MIN
                                                } else {
                                                    folder_info.total_file_count as u64
                                                }
                                            },
                                            children: Vec::with_capacity(0),
                                        })
                                    } else if let Some(file_info) = f.file_info {
                                        FsNode::File(File {
                                            id: Some(file_info.file_id),
                                            name: file_info.file_name,
                                            uri: None,
                                            base64: None,
                                            r#mime: None,
                                            size: {
                                                match file_info.file_size.parse::<u64>() {
                                                    Ok(size) => Some(size),
                                                    Err(e) =>{
                                                        tracing::error!("Onebotv11: Failed to parse file size: {}, error: {}", file_info.file_size, e);
                                                        None
                                                    },
                                                }
                                            },
                                        })
                                    } else {
                                        FsNode::Unknown
                                    }
                                })
                                .collect(),
                        })
                    }
                    _=> Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to get group file list, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn delete_group_file<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        file_id: String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::DelGroupFile(
            onebot_v11::api::payload::DelGroupFile {
                group_id: group_id.parse().unwrap_or_else(|e| {
                    tracing::error!(
                        "Onebotv11: Failed to parse group id: {}, error: {}",
                        group_id,
                        e
                    );
                    Default::default()
                }),
                file_id: file_id,
            },
        );
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to delete group file, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn delete_group_folder<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        folder_id: String,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::DelGroupFileFolder(
            onebot_v11::api::payload::DelGroupFileFolder {
                group_id: group_id.parse().unwrap_or_else(|e| {
                    tracing::error!(
                        "Onebotv11: Failed to parse group id: {}, error: {}",
                        group_id,
                        e
                    );
                    Default::default()
                }),
                folder_id: folder_id,
            },
        );
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to delete group folder, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn create_group_folder<'life0, 'async_trait>(
        &'life0 self,
        group_id: String,
        folder_name: String,
        parent_folder_id: Option<String>,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        if parent_folder_id.is_some() {
            tracing::error!("Onebotv11: Create Group Folder Parent Fold ID is not supported");
        }
        let payload =
            onebot_v11::api::payload::ApiPayload::SetGroupFileFolder(SetGroupFileFolder {
                group_id: group_id.parse().unwrap_or_else(|e| {
                    tracing::error!(
                        "Onebotv11: Failed to parse group id: {}, error: {}",
                        group_id,
                        e
                    );
                    Default::default()
                }),
                folder_name: folder_name,
            });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to create group folder, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_user_profile<'life0, 'async_trait>(
        &'life0 self,
        user_id: String,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<UserGetProfileResponse>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::GetStrangerInfo(GetStrangerInfo {
            user_id: user_id.parse().unwrap_or_else(|e| {
                tracing::error!(
                    "Onebotv11: Failed to parse user id: {}, error: {}",
                    user_id,
                    e
                );
                Default::default()
            }),
            no_cache: true,
        });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                match resp.data {
                    onebot_v11::api::resp::ApiRespData::GetStrangerInfoResponse(resp) => {
                        Ok(UserGetProfileResponse {
                            profile: UserProfile {
                                nickname: Some(resp.nickname),
                                sex: Some(Sex::from(resp.sex.as_str())),
                                age: Some(resp.age as u64),
                                ..Default::default()
                            },
                        })
                    }
                    _ => Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to get user profile, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn set_bot_profile<'life0, 'async_trait>(
        &'life0 self,
        new_profile: UserProfile,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = if let Some(avatar) = new_profile.avatar {
            Ok(onebot_v11::api::payload::ApiPayload::SetQQAvatar(
                SetQQAvatar {
                    file: avatar.to_string(),
                },
            ))
        } else {
            Err(anyhow::anyhow!("Onebotv11: New Bot avatar is None"))
        };
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload?).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to set bot profile, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_bot_profile<'life0, 'async_trait>(
        &'life0 self,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<BotGetProfileResponse>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::GetLoginInfo(
            onebot_v11::api::payload::GetLoginInfo {},
        );
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                match resp.data {
                    onebot_v11::api::resp::ApiRespData::GetLoginInfoResponse(resp) => {
                        Ok(BotGetProfileResponse {
                            profile: UserProfile {
                                nickname: Some(resp.nickname),
                                ..Default::default()
                            },
                        })
                    }
                    _ => Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to get bot profile, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_bot_friend_list<'life0, 'async_trait>(
        &'life0 self,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<BotGetFriendListResponse>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::GetFriendList(
            onebot_v11::api::payload::GetFriendList {},
        );

        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                match resp.data {
                    onebot_v11::api::resp::ApiRespData::GetFriendListResponse(resp) => {
                        Ok(BotGetFriendListResponse {
                            friends: resp
                                .into_iter()
                                .map(|f| User {
                                    id: f.user_id.to_string(),
                                    profile: Some(UserProfile {
                                        nickname: Some(f.nickname),
                                        ..Default::default()
                                    }),
                                    group_info: None,
                                })
                                .collect(),
                        })
                    }
                    _ => Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to get bot friend list, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_bot_group_list<'life0, 'async_trait>(
        &'life0 self,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<BotGetGroupListResponse>>
                + ::core::marker::Send
                + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::GetGroupList(
            onebot_v11::api::payload::GetGroupList {},
        );
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                match resp.data {
                    onebot_v11::api::resp::ApiRespData::GetGroupListResponse(resp) => {
                        Ok(BotGetGroupListResponse {
                            groups: resp
                                .into_iter()
                                .map(|g| oxidebot::source::group::Group {
                                    id: g.group_id.to_string(),
                                    profile: Some(oxidebot::source::group::GroupProfile {
                                        name: Some(g.group_name),
                                        avatar: None,
                                        member_count: {
                                            if g.member_count > u64::MAX as i64 {
                                                Some(u64::MAX)
                                            } else if g.member_count < u64::MIN as i64 {
                                                Some(u64::MIN)
                                            } else {
                                                Some(g.member_count as u64)
                                            }
                                        },
                                    }),
                                })
                                .collect(),
                        })
                    }
                    _ => Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to get bot group list, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn handle_add_friend_request<'life0, 'async_trait>(
        &'life0 self,
        id: String,
        response: RequestResponse,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload =
            onebot_v11::api::payload::ApiPayload::SetFriendAddRequest(SetFriendAddRequest {
                flag: id.parse().unwrap_or_else(|e| {
                    tracing::error!("Onebotv11: Failed to parse user id: {}, error: {}", id, e);
                    Default::default()
                }),
                approve: match response {
                    oxidebot::api::payload::RequestResponse::Approve => true,
                    oxidebot::api::payload::RequestResponse::Reject => false,
                },
                remark: None,
            });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to handle add friend request, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn handle_add_group_request<'life0, 'async_trait>(
        &'life0 self,
        id: String,
        response: RequestResponse,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload =
            onebot_v11::api::payload::ApiPayload::SetGroupAddRequest(SetGroupAddRequest {
                flag: id.parse().unwrap_or_else(|e| {
                    tracing::error!("Onebotv11: Failed to parse user id: {}, error: {}", id, e);
                    Default::default()
                }),
                sub_type: "add".to_string(),
                reason: None,
                approve: match response {
                    oxidebot::api::payload::RequestResponse::Approve => true,
                    oxidebot::api::payload::RequestResponse::Reject => false,
                },
            });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to handle add group request, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn handle_invite_group_request<'life0, 'async_trait>(
        &'life0 self,
        id: String,
        response: RequestResponse,
    ) -> ::core::pin::Pin<
        Box<dyn ::core::future::Future<Output = Result<()>> + ::core::marker::Send + 'async_trait>,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload =
            onebot_v11::api::payload::ApiPayload::SetGroupAddRequest(SetGroupAddRequest {
                flag: id.parse().unwrap_or_else(|e| {
                    tracing::error!("Onebotv11: Failed to parse user id: {}, error: {}", id, e);
                    Default::default()
                }),
                sub_type: "invite".to_string(),
                reason: None,
                approve: match response {
                    oxidebot::api::payload::RequestResponse::Approve => true,
                    oxidebot::api::payload::RequestResponse::Reject => false,
                },
            });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                Ok(())
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to handle invite group request, resp: {:?}",
                    resp
                ))
            }
        })
    }

    #[must_use]
    #[allow(clippy::type_complexity, clippy::type_repetition_in_bounds)]
    fn get_file_info<'life0, 'async_trait>(
        &'life0 self,
        file_id: String,
    ) -> ::core::pin::Pin<
        Box<
            dyn ::core::future::Future<Output = Result<File>> + ::core::marker::Send + 'async_trait,
        >,
    >
    where
        'life0: 'async_trait,
        Self: 'async_trait,
    {
        let payload = onebot_v11::api::payload::ApiPayload::GetFile(GetFile { file_id: file_id });
        Box::pin(async move {
            let resp = self.connect.clone().call_api(payload).await?;
            if resp.status == "ok" {
                match resp.data {
                    onebot_v11::api::resp::ApiRespData::GetFileResponse(resp) => Ok(File {
                        name: resp.file_name,
                        uri: None,
                        mime: None,
                        size: Some(resp.file_size),
                        base64: Some(resp.base64),
                        id: None,
                    }),
                    _ => Err(anyhow::anyhow!("Onebotv11: Unexpected response")),
                }
            } else {
                Err(anyhow::anyhow!(
                    "Onebotv11: Failed to get file info, resp: {:?}",
                    resp
                ))
            }
        })
    }
}
