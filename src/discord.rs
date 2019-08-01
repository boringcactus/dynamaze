extern crate bincode;
extern crate discord_game_sdk;

use std::ffi::CString;
use std::sync::mpsc;

use bincode::{deserialize, serialize};
use discord_game_sdk::*;
pub use discord_game_sdk::{Activity, Lobby};

use crate::net::{Message, MessageCtrl};
use crate::Outbox;

use self::discord_game_sdk::event::activities::Join;
use self::discord_game_sdk::event::lobbies::MemberConnect;

#[allow(clippy::unreadable_literal)]
const CLIENT_ID: i64 = 605927677744119827;
const CHANNEL: u8 = 0;

// this is just a pattern i was using a lot
trait SoftExpect {
    fn soft_expect(self, message: &str);
}

impl<T: std::fmt::Display> SoftExpect for std::result::Result<(), T> {
    fn soft_expect(self, message: &str) {
        if let Err(e) = self {
            eprintln!("{}: {}", message, e);
        }
    }
}

pub trait DiscordEasy {
    fn register(&mut self);
    fn run_callbacks(&mut self);
    fn update_activity(&mut self, activity: &Activity);
    fn got_lobby(&mut self, lobby: Result<Lobby>);
}

enum LobbyHandle {
    Pending(mpsc::Receiver<i64>),
    Done(i64),
}

impl LobbyHandle {
    fn read(&mut self) -> Option<i64> {
        if let LobbyHandle::Pending(recv) = self {
            if let Ok(data) = recv.try_recv() {
                *self = LobbyHandle::Done(data);
            }
        }
        if let LobbyHandle::Done(data) = self {
            Some(*data)
        } else {
            None
        }
    }
}

pub struct DiscordHandle<'a> {
    pub discord: Discord<'a>,
    lobby: Option<LobbyHandle>,
}

impl<'a> DiscordHandle<'a> {
    pub fn new() -> Result<Self> {
        let discord = Discord::with_create_flags(CLIENT_ID, CreateFlags::NoRequireDiscord)?;
        Ok(Self {
            discord,
            lobby: None,
        })
    }

    fn lobby_id(&mut self) -> Option<i64> {
        if let Some(ref mut handle) = self.lobby {
            handle.read()
        } else {
            None
        }
    }

    pub fn register(&mut self) {
        let path = std::env::current_exe().unwrap();
        let path = path.into_os_string();
        let path = path.to_string_lossy();
        let path = String::from(path);
        let path = CString::new(path).unwrap();
        self.discord.register_launch_command(&path).soft_expect("Discord register error");
    }

    pub fn run_callbacks(&mut self) {
        self.discord.empty_event_receivers();
        self.discord.run_callbacks().soft_expect("Discord callback error");
    }

    pub fn update_activity(&mut self, activity: &Activity) {
        self.discord.update_activity(activity, |_, result| {
            result.soft_expect("Discord activity error");
        });
    }

    pub fn got_lobby(discord: &mut Discord, lobby: Result<Lobby>, dest: &mut mpsc::Sender<i64>) {
        match lobby {
            Ok(lobby) => {
                let id = lobby.id();
                discord.connect_lobby_network(id).soft_expect("Discord network connect error");
                discord.open_lobby_network_channel(id, CHANNEL, true).soft_expect("Discord network connect error");
                dest.send(id).soft_expect("Discord network connect error");
            }
            Err(x) => {
                eprintln!("Discord lobby error: {:?}", x);
            }
        }
    }

    pub fn create_lobby(&mut self) {
        let mut txn = LobbyTransaction::new();
        txn
            .kind(LobbyKind::Public)
            .capacity(crate::MAX_PLAYERS);
        let (mut send, recv) = mpsc::channel();
        self.lobby = Some(LobbyHandle::Pending(recv));
        self.discord.create_lobby(&txn, move |discord, lobby| Self::got_lobby(discord, lobby, &mut send));
    }

    pub fn join_lobby(&mut self, secret: String) {
        let secret = CString::new(secret).expect("Lobby secret error");
        let (mut send, recv) = mpsc::channel();
        self.lobby = Some(LobbyHandle::Pending(recv));
        self.discord.connect_lobby_with_activity_secret(secret, move |discord, lobby| Self::got_lobby(discord, lobby, &mut send));
    }

    pub fn lock_lobby(&mut self) {
        if let Some(lobby) = self.lobby_id() {
            let mut txn = LobbyTransaction::new();
            txn.locked(true);
            self.discord.update_lobby(lobby, &txn, |_, result| {
                result.soft_expect("Discord lobby edit error");
            })
        }
    }

    pub fn flush_network(&mut self) {
        self.discord.flush_lobby_network().soft_expect("Discord network flush error");
    }

    pub fn get_id_and_secret(&mut self) -> Option<(i64, String)> {
        if let Some(lobby) = self.lobby_id() {
            let secret = self.discord.lobby_activity_secret(lobby).expect("Discord lobby info error");
            Some((lobby, secret))
        } else {
            None
        }
    }

    fn do_send_message(&mut self, message: &MessageCtrl) -> Result<()> {
        if let Some(lobby) = self.lobby_id() {
            let members = self.discord.all_lobby_member_ids(lobby)?;
            let me = self.discord.current_user()?.id();
            let real_message = match message {
                MessageCtrl::SendGlobal(m) => m,
                MessageCtrl::SendNearGlobal(m, _) => m,
                MessageCtrl::Disconnect => unreachable!()
            };
            let buf = serialize(real_message).expect("Discord network send error");
            for user in members {
                if user != me && message.should_send(user) {
                    self.discord.send_lobby_network_message(lobby, user, CHANNEL, &buf)
                        .soft_expect("Discord network send error");
                }
            }
        }
        Ok(())
    }

    pub fn send_message(&mut self, message: &MessageCtrl) {
        match message {
            MessageCtrl::Disconnect => self.disconnect(),
            _ => self.do_send_message(message).soft_expect("Discord network send error")
        }
    }

    pub fn drain_messages(&mut self, messages: &mut Outbox) {
        if self.lobby_id().is_some() {
            while let Some(message) = messages.pop_front() {
                self.send_message(&message);
            }
        }
    }

    pub fn peek_message(&mut self) -> Option<Message> {
        let receivers = self.discord.event_receivers();
        receivers.lobbies_network_message.try_recv()
            .ok()
            .map(|message| {
                deserialize(&message.buffer).expect("Discord network receive error")
            })
    }

    pub fn peek_join(&mut self) -> Option<Join> {
        let receivers = self.discord.event_receivers();
        receivers.activities_join.try_recv().ok()
    }

    pub fn peek_connect(&mut self) -> Option<MemberConnect> {
        let receivers = self.discord.event_receivers();
        receivers.lobbies_member_connect.try_recv().ok()
    }

    pub fn disconnect(&mut self) {
        if let Some(lobby) = self.lobby_id() {
            self.discord.disconnect_lobby_network(lobby).soft_expect("Discord disconnect error");
            self.discord.disconnect_lobby(lobby, |_, r| r.soft_expect("Discord disconnect error"));
        }
        self.lobby = None;
    }

    pub fn my_name(&mut self) -> String {
        self.discord.current_user().map(|x| x.username().to_string()).unwrap_or_default()
    }
}
