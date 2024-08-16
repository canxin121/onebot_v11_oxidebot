use std::str::FromStr;

use oxidebot::source::{
    message::{File, Message, MessageSegment},
    user::{User, UserProfile},
};
use onebot_v11::message::segment::{AtData, JsonData, RecordData, VideoData, XmlData};

pub(crate) fn parse_uri(uri: &str) -> Option<hyper::Uri> {
    if let Ok(uri) = hyper::Uri::from_str(uri) {
        Some(uri)
    } else {
        tracing::error!("Failed to convert url({uri}) to uri");
        None
    }
}

pub(crate) fn cast_segment(segment: onebot_v11::MessageSegment) -> MessageSegment {
    match segment {
        onebot_v11::MessageSegment::Text { data } => MessageSegment::Text { content: data.text },
        onebot_v11::MessageSegment::Face { data } => MessageSegment::Emoji { id: data.id },
        onebot_v11::MessageSegment::Mface { data } => MessageSegment::Image {
            id: None,
            file: Some(File {
                name: data.summary,
                uri: parse_uri(&data.url),
                mime: None,
                size: None,
                base64: None,
                id: None,
            }),
        },
        onebot_v11::MessageSegment::At { data } => {
            if data.qq == "all" {
                MessageSegment::AtAll
            } else {
                MessageSegment::At { user_id: data.qq }
            }
        }
        onebot_v11::MessageSegment::Image { data } => {
            let name = data.file;
            let file = Some(File {
                name: name.clone(),
                uri: data.url.and_then(|url| parse_uri(&url)),
                mime: {
                    if let Some(mime) = mime_guess::from_path(&name).first() {
                        Some(mime)
                    } else {
                        if let Some(mime) = mime_guess::from_path(name).first() {
                            Some(mime)
                        } else {
                            None
                        }
                    }
                },
                size: None,
                base64: None,
                id: None,
            });
            MessageSegment::Image {
                id: None,
                file: file,
            }
        }
        onebot_v11::MessageSegment::Record { data } => {
            let mime = {
                if let Some(mime) = mime_guess::from_path(&data.file).first() {
                    Some(mime)
                } else {
                    None
                }
            };
            MessageSegment::Audio {
                id: None,
                file: Some(File {
                    name: data.file,
                    uri: data.url.and_then(|u| parse_uri(&u)),
                    mime: mime,
                    size: None,
                    base64: None,
                    id: None,
                }),
                length: None,
            }
        }
        onebot_v11::MessageSegment::Video { data } => MessageSegment::Video {
            file: {
                let mime = {
                    if let Some(mime) = mime_guess::from_path(&data.file).first() {
                        Some(mime)
                    } else {
                        None
                    }
                };
                Some(File {
                    name: data.file,
                    uri: data.url.and_then(|u| parse_uri(&u)),
                    mime: mime,
                    size: None,
                    base64: None,
                    id: None,
                })
            },
            id: None,
            length: None,
        },
        onebot_v11::MessageSegment::File { data } => MessageSegment::File {
            id: Some(data.file_id),
            file: {
                let mime = {
                    if let Some(mime) = mime_guess::from_path(&data.file).first() {
                        Some(mime)
                    } else {
                        None
                    }
                };
                Some(File {
                    name: data.file,
                    uri: data.url.and_then(|u| parse_uri(&u)),
                    mime: mime,
                    size: {
                        if let Ok(size) = data.file_size.parse() {
                            Some(size)
                        } else {
                            None
                        }
                    },
                    base64: None,
                    id: None,
                })
            },
        },
        onebot_v11::MessageSegment::Rps { .. } => MessageSegment::Emoji {
            id: "Rps".to_string(),
        },
        onebot_v11::MessageSegment::Dice { .. } => MessageSegment::Emoji {
            id: "Dice".to_string(),
        },
        onebot_v11::MessageSegment::Shake { .. } => MessageSegment::CustomString {
            r#type: "Shake".to_string(),
            data: "".to_string(),
        },
        onebot_v11::MessageSegment::Poke { .. } => MessageSegment::CustomString {
            r#type: "Poke".to_string(),
            data: "".to_string(),
        },
        onebot_v11::MessageSegment::Anonymous { data } => MessageSegment::CustomValue {
            r#type: "anonymous".to_string(),
            data: serde_json::to_value(data).unwrap_or_else(|e| {
                tracing::error!("Failed to convert anonymous message segment to value, error: {e}");
                serde_json::Value::Null
            }),
        },
        onebot_v11::MessageSegment::Share { data } => MessageSegment::Share {
            title: data.title.clone(),
            content: data.content,
            url: data.url,
            image: data.image.and_then(|u| {
                Some(File {
                    name: data.title,
                    uri: parse_uri(&u),
                    mime: None,
                    size: None,
                    base64: None,
                    id: None,
                })
            }),
        },
        onebot_v11::MessageSegment::Contact { data } => MessageSegment::CustomValue {
            r#type: "contact".to_string(),
            data: serde_json::to_value(data).unwrap_or_else(|e| {
                tracing::error!("Failed to convert contact message segment to value, error: {e}");
                serde_json::Value::Null
            }),
        },
        onebot_v11::MessageSegment::Location { data } => {
            fn parse_num(num: &str) -> f64 {
                if let Ok(num) = num.parse() {
                    num
                } else {
                    tracing::error!("Failed to parse number({num})");
                    0.0
                }
            }
            MessageSegment::Location {
                latitude: parse_num(&data.lat),
                longitude: parse_num(&data.lon),
                title: data.title.unwrap_or_default(),
                content: data.content,
            }
        }
        onebot_v11::MessageSegment::Music { data } => MessageSegment::CustomValue {
            r#type: "music".to_string(),
            data: serde_json::to_value(data).unwrap_or_else(|e| {
                tracing::error!("Failed to convert music message segment to value, error: {e}");
                serde_json::Value::Null
            }),
        },
        onebot_v11::MessageSegment::CustomMusic { data } => MessageSegment::Share {
            title: data.title.clone(),
            content: data.content,
            url: data.url,
            image: data.image.and_then(|u| {
                Some(File {
                    name: data.title,
                    uri: parse_uri(&u),
                    mime: None,
                    size: None,
                    base64: None,
                    id: None,
                })
            }),
        },
        onebot_v11::MessageSegment::Reply { data } => MessageSegment::Reply {
            message_id: data.id,
        },
        onebot_v11::MessageSegment::Forward { data } => MessageSegment::ForwardNode {
            message_id: data.id,
        },
        onebot_v11::MessageSegment::Node { data } => MessageSegment::ForwardNode {
            message_id: data.id,
        },
        onebot_v11::MessageSegment::CustomNode { data } => MessageSegment::ForwardCustomNode {
            user: {
                match data.uin {
                    Some(uin) => {
                        let profile = match data.name {
                            Some(name) => Some(UserProfile {
                                nickname: Some(name),
                                ..Default::default()
                            }),
                            None => None,
                        };
                        Some(User {
                            id: uin.to_string(),
                            profile,
                            group_info: None,
                        })
                    }
                    None => None,
                }
            },
            message: {
                let segments = data.content.into_iter().map(|s| cast_segment(s)).collect();
                Message {
                    segments,
                    id: String::with_capacity(0),
                }
            },
        },
        onebot_v11::MessageSegment::Xml { data } => MessageSegment::CustomString {
            r#type: "xml".to_string(),
            data: data.data,
        },
        onebot_v11::MessageSegment::Json { data } => MessageSegment::CustomString {
            r#type: "json".to_string(),
            data: data.data,
        },
    }
}

pub(crate) fn parse_segment(segment: MessageSegment) -> onebot_v11::MessageSegment {
    match segment {
        MessageSegment::Text { content } => onebot_v11::MessageSegment::text(content),
        MessageSegment::Image { file, .. } => {
            let file = file.unwrap_or_else(|| {
                tracing::error!("Onebotv11: MessageSegment::Image file is None");
                Default::default()
            });
            let file_string;
            if let Some(base64) = file.base64 {
                file_string = base64;
            } else {
                if let Some(uri) = file.uri {
                    file_string = uri.to_string();
                } else {
                    tracing::error!("Onebotv11: MessageSegment::Image uri is None");
                    file_string = String::with_capacity(0);
                }
            }
            onebot_v11::MessageSegment::easy_image(file_string, Some(file.name))
        }
        MessageSegment::Video { file, .. } => onebot_v11::MessageSegment::Video {
            data: VideoData {
                file: {
                    if let Some(file) = file {
                        let file_string;
                        if let Some(base64) = file.base64 {
                            file_string = base64;
                        } else {
                            if let Some(uri) = file.uri {
                                file_string = uri.to_string();
                            } else {
                                tracing::error!("OnebotV11: MessageSegment::Video uri is None");
                                file_string = String::with_capacity(0);
                            }
                        }
                        file_string
                    } else {
                        tracing::error!("OnebotV11: MessageSegment::Video file is None");
                        String::with_capacity(0)
                    }
                },
                url: None,
                cache: None,
                proxy: None,
                timeout: None,
            },
        },
        MessageSegment::Audio { file, .. } => onebot_v11::MessageSegment::Record {
            data: RecordData {
                file: {
                    if let Some(file) = file {
                        let file_string;
                        if let Some(base64) = file.base64 {
                            file_string = base64;
                        } else {
                            if let Some(uri) = file.uri {
                                file_string = uri.to_string();
                            } else {
                                tracing::error!("OnebotV11: MessageSegment::Audio uri is None");
                                file_string = String::with_capacity(0);
                            }
                        }
                        file_string
                    } else {
                        tracing::error!("OnebotV11: MessageSegment::Audio file is None");
                        String::with_capacity(0)
                    }
                },
                magic: None,
                url: None,
                cache: None,
                proxy: None,
                timeout: None,
            },
        },
        MessageSegment::File { file, .. } => {
            let (file, name) = {
                if let Some(file) = file {
                    let file_string;
                    if let Some(base64) = file.base64 {
                        file_string = base64;
                    } else {
                        if let Some(uri) = file.uri {
                            file_string = uri.to_string();
                        } else {
                            tracing::error!("OnebotV11: MessageSegment::File uri is None");
                            file_string = String::with_capacity(0);
                        }
                    }
                    (file_string, file.name)
                } else {
                    tracing::error!("OnebotV11: MessageSegment::File file is None");
                    (String::with_capacity(0), String::with_capacity(0))
                }
            };
            onebot_v11::MessageSegment::file(file, Some(name))
        }

        MessageSegment::Reply { message_id } => onebot_v11::MessageSegment::reply(message_id),
        MessageSegment::At { user_id } => onebot_v11::MessageSegment::at(user_id),
        MessageSegment::AtAll => onebot_v11::MessageSegment::At {
            data: AtData {
                qq: "all".to_string(),
            },
        },
        MessageSegment::Reference { message_id } => onebot_v11::MessageSegment::reply(message_id),
        MessageSegment::Share {
            title,
            content,
            url,
            image,
        } => onebot_v11::MessageSegment::share(
            url,
            title,
            content,
            Some({
                if let Some(image) = image {
                    if let Some(uri) = image.uri {
                        uri.to_string()
                    } else {
                        tracing::error!("OnebotV11: MessageSegment::Share image uri is None");
                        String::with_capacity(0)
                    }
                } else {
                    tracing::error!("OnebotV11: MessageSegment::Share image is None");
                    String::with_capacity(0)
                }
            }),
        ),
        MessageSegment::Location {
            latitude,
            longitude,
            title,
            content,
        } => onebot_v11::MessageSegment::location(
            latitude.to_string(),
            longitude.to_string(),
            Some(title),
            content,
        ),
        MessageSegment::Emoji { id } => onebot_v11::MessageSegment::face(id),
        MessageSegment::ForwardNode { message_id } => onebot_v11::MessageSegment::node(message_id),
        MessageSegment::ForwardCustomNode { user, message } => {
            let user = user.unwrap_or_default();
            onebot_v11::MessageSegment::custom_node(
                user.id.parse().unwrap_or_default(),
                user.profile
                    .and_then(|p| Some(p.nickname.and_then(|n| Some(n)).unwrap_or_default()))
                    .unwrap_or_default(),
                message
                    .segments
                    .into_iter()
                    .map(|s| parse_segment(s))
                    .collect(),
            )
        }
        MessageSegment::CustomString { r#type, data } => match r#type.as_str() {
            "xml" => onebot_v11::MessageSegment::Xml {
                data: XmlData { data },
            },
            "json" => onebot_v11::MessageSegment::Json {
                data: JsonData { data },
            },
            _ => {
                tracing::error!("OnebotV11: Unknown CustomString type: {type}");
                onebot_v11::MessageSegment::text(String::new())
            }
        },
        MessageSegment::CustomValue { r#type, data } => match r#type.as_str() {
            "contact" => {
                if let Ok(contact) =
                    serde_json::from_value::<onebot_v11::message::segment::ContactData>(data)
                {
                    onebot_v11::MessageSegment::Contact { data: contact }
                } else {
                    tracing::error!("OnebotV11: Failed to parse contact data");
                    onebot_v11::MessageSegment::text(String::new())
                }
            }
            "music" => {
                if let Ok(music) =
                    serde_json::from_value::<onebot_v11::message::segment::MusicData>(data)
                {
                    onebot_v11::MessageSegment::Music { data: music }
                } else {
                    tracing::error!("OnebotV11: Failed to parse music data");
                    onebot_v11::MessageSegment::text(String::new())
                }
            }
            _ => {
                tracing::error!("OnebotV11: Unknown CustomValue type: {type}");
                onebot_v11::MessageSegment::text(String::new())
            }
        },
    }
}
