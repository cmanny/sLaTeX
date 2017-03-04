extern crate hyper;
extern crate rustc_serialize;

use hyper::Url;
use hyper::client::IntoUrl;
use hyper::error::ParseError;

use std::str::FromStr;

use rustc_serialize::json::{Array, Json, Object};

#[derive(Clone)]
pub enum SlackEndpoint {
    SlackRTMStart,
    SlackMessagePost {
        channel: String,
        text: Option<String>,
        parse: Option<String>,
        link_names: Option<bool>,
        as_user: Option<bool>,
        attachments: Option<Array>,
    },
}

impl IntoUrl for SlackEndpoint {
    fn into_url(self: Self) -> Result<Url, ParseError> {
        format!("https://slack.com/api/{}",
                match self {
                    SlackEndpoint::SlackRTMStart => "rtm.start",
                    SlackEndpoint::SlackMessagePost { .. } => "chat.postMessage",
                })
            .into_url()
    }
}

impl SlackEndpoint {
    pub fn post_body(self: Self) -> Option<Json> {
        match self {
            SlackEndpoint::SlackMessagePost { channel,
                                              text,
                                              parse,
                                              link_names,
                                              as_user,
                                              attachments } => {
                let mut body: Object = vec![(String::from_str("channel").unwrap(),
                                             Json::String(channel))]
                    .into_iter()
                    .collect();
                if let Some(v) = text {
                    body.insert(String::from("text"), Json::String(v));
                }
                if let Some(v) = parse {
                    body.insert(String::from("parse"), Json::String(v));
                }
                if let Some(v) = link_names {
                    body.insert(String::from("link_names"), Json::Boolean(v));
                }
                if let Some(v) = as_user {
                    body.insert(String::from("as_user"), Json::Boolean(v));
                }
                if let Some(v) = attachments {
                    body.insert(String::from("attachments"), Json::Array(v));
                }
                Some(Json::Object(body))
            }
            _ => None,
        }
    }
}
