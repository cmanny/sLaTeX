extern crate hyper;
extern crate rustc_serialize;
extern crate websocket;
extern crate url;

use std::fs::File;
use std::io::Read;
use std::error::Error;
use std::str;
use rustc_serialize::json::{encode, Json};

use hyper::client::{Client, IntoUrl};
use hyper::header::ContentType;

use websocket::message::Type;
use websocket::{Client as wsClient, Message, WebSocketStream, Sender as tSender,
                Receiver as tReciever};
use websocket::client::{Sender, Receiver};
use websocket::client::request::Url;

use url::form_urlencoded::Serializer;
use url::percent_encoding::{percent_encode, QUERY_ENCODE_SET};


mod endpoints;

use endpoints::SlackEndpoint;


struct Slack<'a> {
    token: &'a str,
    web_client: Client,
}

impl<'a> Slack<'a> {
    fn new(token: &'a str, hyper_client: Client) -> Slack {
        Slack {
            token: token,
            web_client: hyper_client,
        }
    }

    fn get(self: &Self, api_endpoint: SlackEndpoint) -> Result<Json, Box<Error>> {
        let mut url: Url = try!(api_endpoint.into_url());
        url.query_pairs_mut().append_pair("token", self.token);
        let mut resp = try!(self.web_client.get(url).send());
        let json = try!(Json::from_reader(resp.by_ref()));
        if let Json::Boolean(a) = json["ok"] {
            if a {
                Ok(json)
            } else {
                Err(From::from(json["error"].as_string().unwrap_or("No error provided.")))
            }
        } else {
            Err(From::from("Could not complete request."))
        }
    }

    fn post(self: &Self, api_endpoint: SlackEndpoint) -> Result<Json, Box<Error>> {
        let mut url: Url = try!(api_endpoint.clone().into_url());
        if let Json::Object(b) = api_endpoint.post_body().unwrap() {
            let mut pstring = String::new();
            let mut params = Serializer::new(pstring);
            for (k, v) in b {
                params.append_pair(k.as_str(),
                                   if let Json::String(s) = v {
                                           s
                                       } else {
                                           encode(&v).unwrap()
                                       }
                                       .as_str());
            }
            url.query_pairs_mut().append_pair("token", self.token);
            let mut req = self.web_client.post(url);
            // req = req.body(&body).header(ContentType::json());
            pstring = params.finish();
            req = req.body(&pstring).header(ContentType::form_url_encoded());
            let mut resp = try!(req.send());
            let json = try!(Json::from_reader(resp.by_ref()));
            if let Json::Boolean(a) = json["ok"] {
                if a {
                    Ok(json)
                } else {
                    Err(From::from(json["error"].as_string().unwrap_or("No error provided.")))
                }
            } else {
                Err(From::from("Could not complete request."))
            }
        } else {
            Err(From::from("Couldn't encode post body."))
        }
    }

    fn rtm_begin(self: &mut Self)
                 -> Result<(Sender<WebSocketStream>, Receiver<WebSocketStream>), Box<Error>> {
        let response = try!(self.get(SlackEndpoint::SlackRTMStart));
        let request = try!(wsClient::connect(Url::parse(response["url"].as_string().unwrap())
            .unwrap()));
        let response = try!(request.send());
        try!(response.validate());
        Ok(response.begin().split())
    }
}

fn main() {
    let mut token_file = File::open("token.txt").unwrap();
    let mut token = String::new();
    token_file.read_to_string(&mut token).unwrap();
    let client = Client::new();
    let mut slack = Slack::new(token.trim(), client);
    if let Ok((mut send, mut recv)) = slack.rtm_begin() {
        for message in recv.incoming_messages() {
            let message: Message = message.unwrap();
            match message.opcode {
                Type::Ping => {
                    send.send_message(&Message::pong(message.payload)).unwrap();
                }
                Type::Text => {
                    let content = Json::from_str(str::from_utf8(&message.payload.into_owned())
                            .unwrap())
                        .unwrap();
                    match content["type"] {
                        Json::String(ref msg_type) => {
                            if msg_type == "hello" {
                                println!("Hello received.");
                            } else if msg_type == "message" && content.find("text").is_some() {
                                let mut recv = String::from(content["text"].as_string().unwrap());
                                // println!("Message received {:?}", recv);
                                if recv.starts_with("$") && recv.pop().unwrap_or('_') == '$' &&
                                   recv.len() > 0 {
                                    println!("LaTeX Request {:?}", &recv[1..]);
                                    let image_url = format!("http://latex.codecogs.com/png.\
                                                             latex?%5Cdpi%7B300%7D%20{}",
                                                            percent_encode(recv[1..].as_bytes(),
                                                                           QUERY_ENCODE_SET));
                                    if let Err(e) = slack.post(SlackEndpoint::SlackMessagePost {
                                        channel: String::from(content["channel"]
                                            .as_string()
                                            .unwrap()),
                                        text: None,
                                        parse: None,
                                        link_names: None,
                                        as_user: Some(true),
                                        attachments: Some(vec![Json::Object(vec!(
                                                (
                                                    String::from("fallback"),
                                                    Json::String(String::from("test"))
                                                ),
                                                (
                                                    String::from("image_url"),
                                                    Json::String(image_url)
                                                ),
                                            )
                                                                   .into_iter()
                                                                   .collect())]),
                                    }) {
                                        println!("Failed to post LaTeX: {:?}", e);
                                    }
                                }
                            }
                        }
                        _ => (),
                    }
                }
                _ => println!("{:?}", message),
            };
        }
    }
}
