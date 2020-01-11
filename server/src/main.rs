use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

use actix::*;
use actix_web::{App, Error, HttpRequest, HttpResponse, HttpServer, web};
use actix_web_actors::ws;
use bincode::deserialize;
use rand::{self, Rng, rngs::ThreadRng};

use meta_net::*;

type ClientID = usize;

#[path = "../../src/meta_net.rs"]
mod meta_net;

#[derive(Message)]
#[rtype(result = "()")]
pub struct Message(pub Vec<u8>);

#[derive(Message)]
#[rtype(usize)]
pub struct Connect {
    pub addr: Recipient<Message>,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Disconnect {
    pub id: ClientID,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct ClientMessage {
    pub id: ClientID,
    pub msg: Vec<u8>,
    pub game_id: GameID,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Join {
    pub id: ClientID,
    pub game_id: GameID,
}

#[derive(Message)]
#[rtype(result = "()")]
pub struct Leave {
    pub id: ClientID,
}

pub struct GameServer {
    sessions: HashMap<ClientID, Recipient<Message>>,
    games: HashMap<GameID, HashSet<ClientID>>,
    rng: ThreadRng,
}

impl Default for GameServer {
    fn default() -> GameServer {
        // default room
        let games = HashMap::new();

        GameServer {
            sessions: HashMap::new(),
            games,
            rng: rand::thread_rng(),
        }
    }
}

impl GameServer {
    /// Send message to all users in the game
    fn send_message(&self, game: GameID, message: &[u8], skip_id: ClientID) {
        if let Some(sessions) = self.games.get(&game) {
            for id in sessions {
                if *id != skip_id {
                    if let Some(addr) = self.sessions.get(id) {
                        let _ = addr.do_send(Message(message.to_vec()));
                    }
                }
            }
        }
    }
}

impl Actor for GameServer {
    /// We are going to use simple Context, we just need ability to communicate
    /// with other actors.
    type Context = Context<Self>;
}

/// Handler for Connect message.
///
/// Register new session and assign unique id to this session
impl Handler<Connect> for GameServer {
    type Result = usize;

    fn handle(&mut self, msg: Connect, _: &mut Context<Self>) -> Self::Result {
        println!("Someone joined");

        // register session with random id
        let id = self.rng.gen::<usize>();
        self.sessions.insert(id, msg.addr);

        // send id back
        id
    }
}

/// Handler for Disconnect message.
impl Handler<Disconnect> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: Disconnect, _: &mut Context<Self>) {
        println!("Someone disconnected");

        // remove address
        if self.sessions.remove(&msg.id).is_some() {
            // remove session from all games
            for sessions in self.games.values_mut() {
                sessions.remove(&msg.id);
            }
        }
    }
}

/// Handler for Message message.
impl Handler<ClientMessage> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: ClientMessage, _: &mut Context<Self>) {
        self.send_message(msg.game_id, &msg.msg, msg.id);
    }
}

/// Join room, send disconnect message to old game
/// send join message to new game
impl Handler<Join> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: Join, _: &mut Context<Self>) {
        let Join { id, game_id } = msg;

        // remove session from all games
        for sessions in self.games.values_mut() {
            sessions.remove(&id);
        }

        if self.games.get_mut(&game_id).is_none() {
            self.games.insert(game_id.clone(), HashSet::new());
        }
        self.games.get_mut(&game_id).unwrap().insert(id);
    }
}

/// Handler for Leave message.
impl Handler<Leave> for GameServer {
    type Result = ();

    fn handle(&mut self, msg: Leave, _: &mut Context<Self>) {
        println!("Someone left");

        // remove session from all games
        for sessions in self.games.values_mut() {
            sessions.remove(&msg.id);
        }
    }
}

/// How often heartbeat pings are sent
const HEARTBEAT_INTERVAL: Duration = Duration::from_secs(5);
/// How long before lack of client response causes a timeout
const CLIENT_TIMEOUT: Duration = Duration::from_secs(10);

/// Entry point for our route
async fn game_route(
    req: HttpRequest,
    stream: web::Payload,
    srv: web::Data<Addr<GameServer>>,
) -> Result<HttpResponse, Error> {
    ws::start(
        GameSession {
            id: 0,
            hb: Instant::now(),
            game: None,
            addr: srv.get_ref().clone(),
        },
        &req,
        stream,
    )
}

struct GameSession {
    /// unique session id
    id: ClientID,
    /// Client must send ping at least once per 10 seconds (CLIENT_TIMEOUT),
    /// otherwise we drop connection.
    hb: Instant,
    /// joined game
    game: Option<GameID>,
    /// Chat server
    addr: Addr<GameServer>,
}

impl Actor for GameSession {
    type Context = ws::WebsocketContext<Self>;

    /// Method is called on actor start.
    /// We register ws session with GameServer
    fn started(&mut self, ctx: &mut Self::Context) {
        // we'll start heartbeat process on session start.
        self.hb(ctx);

        // register self in chat server. `AsyncContext::wait` register
        // future within context, but context waits until this future resolves
        // before processing any other events.
        // HttpContext::state() is instance of GameSessionState, state is shared
        // across all routes within application
        let addr = ctx.address();
        self.addr
            .send(Connect {
                addr: addr.recipient(),
            })
            .into_actor(self)
            .then(|res, act, ctx| {
                match res {
                    Ok(res) => act.id = res,
                    // something is wrong with chat server
                    _ => ctx.stop(),
                }
                fut::ready(())
            })
            .wait(ctx);
    }

    fn stopping(&mut self, _: &mut Self::Context) -> Running {
        // notify chat server
        self.addr.do_send(Disconnect { id: self.id });
        Running::Stop
    }
}

/// Handle messages from chat server, we simply send it to peer websocket
impl Handler<Message> for GameSession {
    type Result = ();

    fn handle(&mut self, msg: Message, ctx: &mut Self::Context) {
        ctx.binary(msg.0)
    }
}

/// WebSocket message handler
impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for GameSession {
    fn handle(
        &mut self,
        msg: Result<ws::Message, ws::ProtocolError>,
        ctx: &mut Self::Context,
    ) {
        let msg = match msg {
            Err(_) => {
                ctx.stop();
                return;
            }
            Ok(msg) => msg,
        };

        match msg {
            ws::Message::Ping(msg) => {
                self.hb = Instant::now();
                ctx.pong(&msg);
            }
            ws::Message::Pong(_) => {
                self.hb = Instant::now();
            }
            ws::Message::Text(_) => {
                // TODO handle?
            }
            ws::Message::Binary(data) => {
                let message = deserialize::<MetaMessage>(&data);
                println!("WEBSOCKET MESSAGE: {:?}", message);
                match message {
                    Ok(MetaMessage::Join(game)) => {
                        self.game = Some(game);
                        self.addr.do_send(Join {
                            id: self.id,
                            game_id: game,
                        });
                    }
                    Ok(MetaMessage::Leave) => {
                        self.game = None;
                        self.addr.do_send(Leave {
                            id: self.id,
                        });
                    }
                    Ok(MetaMessage::Message(data)) => {
                        if let Some(game) = self.game {
                            self.addr.do_send(ClientMessage {
                                id: self.id,
                                msg: data,
                                game_id: game,
                            });
                        }
                    }
                    Err(e) => {
                        eprintln!("Got bad message: {}", e);
                    }
                }
            },
            ws::Message::Close(_) => {
                ctx.stop();
            }
            ws::Message::Continuation(_) => {
                ctx.stop();
            }
            ws::Message::Nop => (),
        }
    }
}

impl GameSession {
    /// helper method that sends ping to client every second.
    ///
    /// also this method checks heartbeats from client
    fn hb(&self, ctx: &mut ws::WebsocketContext<Self>) {
        ctx.run_interval(HEARTBEAT_INTERVAL, |act, ctx| {
            // check client heartbeats
            if Instant::now().duration_since(act.hb) > CLIENT_TIMEOUT {
                // heartbeat timed out
                println!("Websocket Client heartbeat failed, disconnecting!");

                // notify chat server
                act.addr.do_send(Disconnect { id: act.id });

                // stop actor
                ctx.stop();

                // don't try to send a ping
                return;
            }

            ctx.ping(b"");
        });
    }
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    // Start chat server actor
    let server = GameServer::default().start();

    // Create Http server with websocket support
    HttpServer::new(move || {
        App::new()
            .data(server.clone())
            // websocket
            .service(web::resource("/").to(game_route))
    })
        .bind(("127.0.0.1", option_env!("PORT").unwrap_or("8080").parse().unwrap()))?
        .run()
        .await
}
